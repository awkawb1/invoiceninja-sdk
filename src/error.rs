use serde::Deserialize;
use std::collections::HashMap;

/// Error shape returned by the Invoice Ninja API for failed requests.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiErrorBody {
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub errors: HashMap<String, Vec<String>>,
}

/// Marked `#[non_exhaustive]`: matching on it must include a catch-all arm,
/// so that new variants can be added without breaking callers.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Rendered with its whole cause chain, because `reqwest`'s own message
    /// stops at "error sending request for url (...)" and leaves out the part
    /// that says why: "Connection refused", a DNS failure, a TLS handshake
    /// error, a timeout.
    #[error("http transport error: {}", cause_chain(.0))]
    Http(#[from] reqwest::Error),

    /// The response was not the JSON this endpoint is documented to return.
    ///
    /// Carries an excerpt of what actually arrived. Without it the inner
    /// serde message ("expected value at line 1 column 1") gives no clue
    /// that the body was, say, an HTML error page from a reverse proxy or a
    /// login redirect from a misconfigured base URL.
    #[error("failed to decode JSON response: {source}; body began: {excerpt}")]
    Json {
        #[source]
        source: serde_json::Error,
        excerpt: String,
    },

    /// A non-2xx response: the server's message, plus any per-field
    /// validation errors.
    ///
    /// **A missing record is `400`, not `404`.** Invoice Ninja's exception
    /// handler maps Laravel's `ModelNotFoundException` to a 400 whose message
    /// is `No query results for model [App\Models\Client].`, and reserves 404
    /// for a route that does not exist or a method that route does not
    /// support. Matching on 404 to mean "not found" will never fire — match
    /// the status together with the message.
    #[error("api returned {status}: {message}")]
    Api {
        status: reqwest::StatusCode,
        message: String,
        errors: HashMap<String, Vec<String>>,
    },

    #[error("invalid base url: {0}")]
    InvalidBaseUrl(String),

    #[error("invalid api token: {0}")]
    InvalidToken(String),

    /// An id or action could not be used as a single URL path segment.
    ///
    /// Raised for values that would change which endpoint is addressed rather
    /// than identifying a record — an empty id (which would hit the
    /// collection route) or one made only of dots (which the URL parser
    /// resolves away). Invoice Ninja ids are hashids, so this only rejects
    /// input that was never a valid id.
    #[error("`{0}` cannot be used as a url path segment")]
    InvalidPathSegment(String),

    /// An upload was refused before any request was sent: no files, a file
    /// with no name, or a MIME type that does not parse.
    ///
    /// Raised locally because the server would otherwise answer an empty
    /// upload with a success that changed nothing.
    #[error("invalid upload: {0}")]
    InvalidUpload(String),

    /// A `list_all_*` call kept receiving full pages past the point where any
    /// plausible result set would have ended.
    ///
    /// This means the server is not paginating the way the client expects —
    /// typically because it ignores the `page` parameter and keeps replaying
    /// the same page. Raised instead of returning the partial result, so the
    /// truncation cannot be mistaken for a complete answer.
    #[error("pagination did not terminate for {path} after {pages} pages")]
    PaginationLimit { path: String, pages: u64 },
}

pub type Result<T> = std::result::Result<T, Error>;

/// Joins an error and every error beneath it with `: `.
///
/// `reqwest::Error`'s `Display` prints only the outermost layer, so the
/// reason a request failed lives in `source()` and is otherwise invisible to
/// anyone who just prints the error.
fn cause_chain(error: &(dyn std::error::Error + 'static)) -> String {
    let mut message = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        let text = cause.to_string();
        // Some layers repeat their cause's text; don't print it twice.
        if !message.contains(&text) {
            message.push_str(": ");
            message.push_str(&text);
        }
        source = cause.source();
    }
    message
}
