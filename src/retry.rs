//! Retry policy for transient failures.
//!
//! The rules here are deliberately conservative about which requests may be
//! replayed. This is a billing API: replaying a `POST` that the server
//! already processed but whose response was lost would create a second
//! invoice or payment, which is far worse than surfacing an error.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::{header, Method, StatusCode};

/// How transient failures are retried.
///
/// The default retries up to 3 times with full-jitter exponential backoff
/// starting at 500ms. Use [`RetryPolicy::none`] to disable retrying.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Retries *after* the initial attempt, so 3 means up to 4 requests.
    pub max_retries: u32,
    /// Backoff for the first retry; doubles each attempt.
    pub base_delay: Duration,
    /// Ceiling on any single wait, including one asked for by `Retry-After`.
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(10),
        }
    }
}

impl RetryPolicy {
    /// Retries `max_retries` times with the default delays.
    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            ..Self::default()
        }
    }

    /// Disables retrying; every request is attempted exactly once.
    pub fn none() -> Self {
        Self::new(0)
    }

    pub fn base_delay(mut self, base_delay: Duration) -> Self {
        self.base_delay = base_delay;
        self
    }

    pub fn max_delay(mut self, max_delay: Duration) -> Self {
        self.max_delay = max_delay;
        self
    }

    /// Full-jitter exponential backoff: a uniform pick from
    /// `[0, min(max_delay, base_delay * 2^attempt)]`.
    ///
    /// Jittering the whole interval rather than a fraction of it is what
    /// stops a set of clients that failed together from retrying together.
    pub(crate) fn backoff(&self, attempt: u32) -> Duration {
        let exponential = self
            .base_delay
            .checked_mul(1u32.checked_shl(attempt).unwrap_or(u32::MAX))
            .unwrap_or(self.max_delay)
            .min(self.max_delay);
        Duration::from_millis(pseudo_random_millis(exponential.as_millis() as u64))
    }
}

/// Picks a value in `[0, upper]` without pulling in a random-number crate.
///
/// Decorrelating retries is all this needs to do — it is not used for
/// anything requiring unpredictability. The counter is mixed in so that
/// calls landing in the same nanosecond still diverge.
fn pseudo_random_millis(upper: u64) -> u64 {
    if upper == 0 {
        return 0;
    }
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);

    // splitmix64 finalizer, enough to spread adjacent seeds.
    let mut x = nanos ^ seq.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    x % (upper + 1)
}

/// Whether replaying the request can be assumed not to double an effect.
///
/// `POST` is excluded: it creates records, runs bulk actions, issues refunds
/// and sends email, none of which is safe to repeat blindly.
fn is_idempotent(method: &Method) -> bool {
    matches!(
        *method,
        Method::GET | Method::HEAD | Method::PUT | Method::DELETE | Method::OPTIONS
    )
}

/// Whether a response status should be retried.
pub(crate) fn status_is_retryable(status: StatusCode, method: &Method) -> bool {
    match status.as_u16() {
        // Rate limited. The server states it did not process the request, so
        // replaying cannot duplicate anything — safe even for POST.
        429 => true,
        // The request may or may not have been applied before the error: a
        // 502 or 504 can mean the response was lost rather than the work
        // undone. Only replay when repeating is harmless.
        500 | 502 | 503 | 504 => is_idempotent(method),
        _ => false,
    }
}

/// Whether a transport-level failure should be retried.
///
/// Same reasoning as a 5xx: a timeout gives no evidence about whether the
/// server processed the request.
pub(crate) fn transport_error_is_retryable(error: &reqwest::Error, method: &Method) -> bool {
    (error.is_timeout() || error.is_connect()) && is_idempotent(method)
}

/// Reads a `Retry-After` delay, which Invoice Ninja sends on throttled
/// responses.
///
/// Only the delta-seconds form is parsed; the HTTP-date form falls through to
/// normal backoff rather than pulling in a date parser.
pub(crate) fn retry_after(headers: &header::HeaderMap) -> Option<Duration> {
    let seconds: u64 = headers
        .get(header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()?;
    Some(Duration::from_secs(seconds))
}
