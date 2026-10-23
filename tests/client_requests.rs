//! Request-plumbing tests: pagination, path escaping, headers, timeouts, and
//! error mapping.
//!
//! These run against a `wiremock` server bound to loopback, so they need no
//! network access and no credentials. `tests/model_roundtrip.rs` covers the
//! serde layer; this file covers everything between a method call and the
//! bytes on the wire.

use std::time::Duration;

use invoiceninjars::{models::ListParams, Error, InvoiceNinjaClient};
use serde_json::json;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Builds a client pointed at the mock server.
fn client(server: &MockServer) -> InvoiceNinjaClient {
    InvoiceNinjaClient::new(server.uri(), "test-token").expect("mock uri is a valid base url")
}

/// A `data`-wrapped list page carrying pagination metadata.
fn page(rows: usize, current_page: u64, total_pages: u64) -> serde_json::Value {
    let data: Vec<_> = (0..rows)
        .map(|i| json!({ "id": format!("id{i}") }))
        .collect();
    json!({
        "data": data,
        "meta": {
            "pagination": {
                "total": rows as u64 * total_pages,
                "count": rows as u64,
                "per_page": rows as u64,
                "current_page": current_page,
                "total_pages": total_pages,
            }
        }
    })
}

// --- auth and headers ---------------------------------------------------

#[tokio::test]
async fn token_and_accept_headers_are_sent() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/clients"))
        .and(header("X-Api-Token", "test-token"))
        .and(header("Accept", "application/json"))
        .and(header("X-Requested-With", "XMLHttpRequest"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page(0, 1, 1)))
        .mount(&server)
        .await;

    client(&server)
        .list_clients(&ListParams::new())
        .await
        .expect("request should succeed");
}

#[tokio::test]
async fn api_password_header_is_attached_when_configured() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/users/u1"))
        .and(header("X-API-PASSWORD", "hunter2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": { "id": "u1" } })))
        .mount(&server)
        .await;

    let client = client(&server)
        .with_api_password("hunter2")
        .expect("password is a valid header value");
    let user = client.get_user("u1").await.expect("request should succeed");
    assert_eq!(user.id.as_deref(), Some("u1"));
}

#[tokio::test]
async fn api_password_header_is_absent_by_default() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/clients"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page(0, 1, 1)))
        .mount(&server)
        .await;

    client(&server)
        .list_clients(&ListParams::new())
        .await
        .expect("request should succeed");

    let sent = &server.received_requests().await.unwrap()[0];
    assert!(
        sent.headers.get("X-API-PASSWORD").is_none(),
        "password header must not be sent unless configured"
    );
}

// --- path escaping ------------------------------------------------------

#[tokio::test]
async fn ids_are_escaped_into_a_single_path_segment() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": { "id": "x" } })))
        .mount(&server)
        .await;

    // A `/` in the id must not open a new path segment: the whole value
    // stays under /clients rather than reaching /companies.
    client(&server)
        .get_client("../companies/1")
        .await
        .expect("escaped id should still reach the clients endpoint");

    let sent = &server.received_requests().await.unwrap()[0];
    assert_eq!(sent.url.path(), "/clients/%2E%2E%2Fcompanies%2F1");
}

/// Escaping by itself does not make a dot-only id safe: the WHATWG rules the
/// `url` crate follows decode `%2E` back to `.` before resolving dot
/// segments, so `clients/%2E%2E` still collapses to `/`. Such ids have to be
/// refused before a request is built.
#[tokio::test]
async fn dot_only_ids_are_rejected_before_any_request_is_sent() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    for id in ["..", ".", "..."] {
        let err = client(&server)
            .delete_client(id)
            .await
            .expect_err("a dot-only id must not be sent");
        assert!(
            matches!(err, Error::InvalidPathSegment(ref v) if v == id),
            "expected InvalidPathSegment for {id:?}, got {err:?}"
        );
    }

    assert!(
        server.received_requests().await.unwrap().is_empty(),
        "nothing should have reached the server"
    );
}

#[tokio::test]
async fn empty_ids_are_rejected_rather_than_hitting_the_collection() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    // `delete_client("")` would otherwise address DELETE /clients.
    let err = client(&server)
        .delete_client("")
        .await
        .expect_err("an empty id must not be sent");
    assert!(matches!(err, Error::InvalidPathSegment(_)), "got {err:?}");
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn query_injection_via_id_is_escaped() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/invoices/abc%3Fper_page%3D1/download"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"%PDF-1.4".to_vec()))
        .mount(&server)
        .await;

    let pdf = client(&server)
        .download_invoice("abc?per_page=1")
        .await
        .expect("escaped id should not inject a query string");
    assert_eq!(pdf, b"%PDF-1.4");
}

#[tokio::test]
async fn ordinary_hashids_are_left_readable() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/clients/Wpmbk5ezJn"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "data": { "id": "Wpmbk5ezJn" } })),
        )
        .mount(&server)
        .await;

    let got = client(&server)
        .get_client("Wpmbk5ezJn")
        .await
        .expect("a normal id should pass through unescaped");
    assert_eq!(got.id.as_deref(), Some("Wpmbk5ezJn"));
}

// --- pagination ---------------------------------------------------------

#[tokio::test]
async fn list_all_follows_total_pages_from_metadata() {
    let server = MockServer::start().await;
    for p in 1..=3u64 {
        Mock::given(method("GET"))
            .and(path("/clients"))
            .and(query_param("page", p.to_string()))
            .respond_with(ResponseTemplate::new(200).set_body_json(page(50, p, 3)))
            .mount(&server)
            .await;
    }

    let all = client(&server)
        .list_all_clients(&ListParams::new())
        .await
        .expect("should walk every page");
    assert_eq!(all.len(), 150, "expected all three pages");
}

/// The regression this rewrite exists for. The server caps `per_page` at
/// 5000, so asking for more returns a page shorter than requested while more
/// records remain. The old short-page-means-last heuristic stopped there and
/// silently dropped the rest.
#[tokio::test]
async fn capped_page_size_does_not_truncate_the_walk() {
    let server = MockServer::start().await;
    for p in 1..=2u64 {
        Mock::given(method("GET"))
            .and(path("/clients"))
            .and(query_param("page", p.to_string()))
            // 10 rows returned against a much larger requested page size.
            .respond_with(ResponseTemplate::new(200).set_body_json(page(10, p, 2)))
            .mount(&server)
            .await;
    }

    let all = client(&server)
        .list_all_clients(&ListParams::new().per_page(9999))
        .await
        .expect("should keep paging past a server-capped page");
    assert_eq!(
        all.len(),
        20,
        "a short page must not be read as the last page when metadata says otherwise"
    );
}

#[tokio::test]
async fn per_page_is_clamped_to_the_server_maximum() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/clients"))
        .and(query_param("per_page", "5000"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page(0, 1, 1)))
        .mount(&server)
        .await;

    client(&server)
        .list_all_clients(&ListParams::new().per_page(100_000))
        .await
        .expect("oversized per_page should be clamped to 5000, not sent verbatim");
}

/// `per_page(0)` made the old loop compare `got < 0`, which nothing
/// satisfies — it paged forever. It must fall back to the default size.
#[tokio::test]
async fn zero_per_page_does_not_spin() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/clients"))
        .and(query_param("per_page", "50"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page(0, 1, 1)))
        .mount(&server)
        .await;

    let all = tokio::time::timeout(
        Duration::from_secs(10),
        client(&server).list_all_clients(&ListParams::new().per_page(0)),
    )
    .await
    .expect("must terminate rather than page forever")
    .expect("request should succeed");
    assert!(all.is_empty());
}

#[tokio::test]
async fn empty_page_terminates_even_if_metadata_claims_more() {
    let server = MockServer::start().await;
    // Metadata insists there are 99 pages, but the first comes back empty.
    Mock::given(method("GET"))
        .and(path("/clients"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page(0, 1, 99)))
        .mount(&server)
        .await;

    let all = tokio::time::timeout(
        Duration::from_secs(10),
        client(&server).list_all_clients(&ListParams::new()),
    )
    .await
    .expect("an empty page must end the walk")
    .expect("request should succeed");
    assert!(all.is_empty());
}

#[tokio::test]
async fn list_all_falls_back_to_short_page_without_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/clients"))
        .and(query_param("page", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": (0..50).map(|i| json!({ "id": format!("id{i}") })).collect::<Vec<_>>()
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/clients"))
        .and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{ "id": "last" }]
        })))
        .mount(&server)
        .await;

    let all = client(&server)
        .list_all_clients(&ListParams::new())
        .await
        .expect("should page without metadata");
    assert_eq!(all.len(), 51);
}

#[tokio::test]
async fn list_all_honours_a_starting_page() {
    let server = MockServer::start().await;
    for p in 2..=3u64 {
        Mock::given(method("GET"))
            .and(path("/clients"))
            .and(query_param("page", p.to_string()))
            .respond_with(ResponseTemplate::new(200).set_body_json(page(50, p, 3)))
            .mount(&server)
            .await;
    }

    let all = client(&server)
        .list_all_clients(&ListParams::new().page(2))
        .await
        .expect("should start at the requested page");
    assert_eq!(all.len(), 100, "pages 2 and 3 only");
}

#[tokio::test]
async fn custom_params_are_sent_on_every_page() {
    let server = MockServer::start().await;
    for p in 1..=2u64 {
        Mock::given(method("GET"))
            .and(path("/clients"))
            .and(query_param("page", p.to_string()))
            .and(query_param("client_id", "abc"))
            .and(query_param("status", "active"))
            .respond_with(ResponseTemplate::new(200).set_body_json(page(50, p, 2)))
            .mount(&server)
            .await;
    }

    let all = client(&server)
        .list_all_clients(&ListParams::new().status("active").param("client_id", "abc"))
        .await
        .expect("filters must survive pagination");
    assert_eq!(all.len(), 100);
}

// --- error mapping ------------------------------------------------------

#[tokio::test]
async fn validation_errors_keep_the_whole_field_map() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/clients"))
        .respond_with(ResponseTemplate::new(422).set_body_json(json!({
            "message": "The given data was invalid.",
            "errors": {
                "name": ["The name field is required."],
                "contacts.0.email": ["The email must be valid.", "Already taken."]
            }
        })))
        .mount(&server)
        .await;

    let err = client(&server)
        .create_client(&Default::default())
        .await
        .expect_err("422 should surface as Error::Api");

    match err {
        Error::Api {
            status,
            message,
            errors,
        } => {
            assert_eq!(status, 422);
            assert_eq!(message, "The given data was invalid.");
            assert_eq!(errors["name"], vec!["The name field is required."]);
            assert_eq!(
                errors["contacts.0.email"],
                vec!["The email must be valid.", "Already taken."],
                "every message is kept, not just the first"
            );
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}

#[tokio::test]
async fn non_json_error_body_falls_back_to_the_raw_text() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/clients"))
        .respond_with(ResponseTemplate::new(502).set_body_string("<html>bad gateway</html>"))
        .mount(&server)
        .await;

    let err = client(&server)
        .list_clients(&ListParams::new())
        .await
        .expect_err("502 should surface as Error::Api");

    match err {
        Error::Api {
            status, message, ..
        } => {
            assert_eq!(status, 502);
            assert!(
                message.contains("bad gateway"),
                "a non-JSON body should be preserved verbatim, got {message:?}"
            );
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}

#[tokio::test]
async fn delete_reports_api_errors_rather_than_succeeding() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/clients/abc"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({ "message": "forbidden" })))
        .mount(&server)
        .await;

    let err = client(&server)
        .delete_client("abc")
        .await
        .expect_err("403 should not read as a successful delete");
    assert!(matches!(err, Error::Api { status, .. } if status == 403));
}

// --- base url validation ------------------------------------------------

#[test]
fn base_url_without_a_scheme_is_rejected_at_construction() {
    let err = InvoiceNinjaClient::new("invoicing.co/api/v1", "t")
        .expect_err("a schemeless url should fail fast, not 404 on every call");
    assert!(matches!(err, Error::InvalidBaseUrl(_)), "got {err:?}");
}

#[test]
fn non_http_base_url_is_rejected() {
    let err = InvoiceNinjaClient::new("ftp://example.com/api/v1", "t")
        .expect_err("only http(s) is usable");
    assert!(matches!(err, Error::InvalidBaseUrl(_)), "got {err:?}");
}

#[test]
fn empty_base_url_is_rejected() {
    let err = InvoiceNinjaClient::new("   ", "t").expect_err("empty base url should fail");
    assert!(matches!(err, Error::InvalidBaseUrl(_)), "got {err:?}");
}

#[test]
fn trailing_slashes_are_tolerated() {
    InvoiceNinjaClient::new("https://example.com/api/v1/", "t")
        .expect("a trailing slash should not be an error");
}

// --- timeouts -----------------------------------------------------------

#[tokio::test]
async fn requests_give_up_once_the_timeout_elapses() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/clients"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(page(0, 1, 1))
                .set_delay(Duration::from_secs(30)),
        )
        .mount(&server)
        .await;

    let client = client(&server).with_timeout(Duration::from_millis(150));
    let err = tokio::time::timeout(
        Duration::from_secs(10),
        client.list_clients(&ListParams::new()),
    )
    .await
    .expect("the client's own timeout should fire well before this one")
    .expect_err("a stalled response must not hang the caller");

    assert!(
        matches!(&err, Error::Http(e) if e.is_timeout()),
        "expected a timeout error, got {err:?}"
    );
}

// --- envelope handling --------------------------------------------------

#[tokio::test]
async fn bulk_response_without_a_data_key_reads_as_empty() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/invoices/bulk"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "message": "No Invoices Found" })),
        )
        .mount(&server)
        .await;

    let got = client(&server)
        .bulk_invoices("archive", &["missing"])
        .await
        .expect("a no-match bulk action is not an error");
    assert!(got.is_empty());
}

#[tokio::test]
async fn ping_accepts_an_empty_success_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ping"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    client(&server).ping().await.expect("ping should succeed");
}

// --- retry and backoff --------------------------------------------------

use invoiceninjars::RetryPolicy;

/// Fast policy so the tests do not actually sleep for seconds.
fn quick_retry() -> RetryPolicy {
    RetryPolicy::new(3)
        .base_delay(Duration::from_millis(1))
        .max_delay(Duration::from_millis(20))
}

#[tokio::test]
async fn transient_5xx_on_a_get_is_retried_until_it_succeeds() {
    let server = MockServer::start().await;
    // First two attempts fail, third succeeds.
    Mock::given(method("GET"))
        .and(path("/clients"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(2)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/clients"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page(1, 1, 1)))
        .mount(&server)
        .await;

    let got = client(&server)
        .with_retry_policy(quick_retry())
        .list_clients(&ListParams::new())
        .await
        .expect("a GET should ride out transient 5xx");
    assert_eq!(got.len(), 1);
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
}

/// The safety property that matters most here: a POST may already have
/// created a record even though the response failed, so replaying it could
/// double-charge. It must surface instead.
#[tokio::test]
async fn post_is_not_replayed_on_5xx() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/clients"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({ "message": "boom" })))
        .mount(&server)
        .await;

    let err = client(&server)
        .with_retry_policy(quick_retry())
        .create_client(&Default::default())
        .await
        .expect_err("a failed create must not be retried");
    assert!(matches!(err, Error::Api { status, .. } if status == 500));
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        1,
        "exactly one create attempt may reach the server"
    );
}

/// 429 is the exception: the server states it refused the request, so no
/// work was done and even a POST is safe to repeat.
#[tokio::test]
async fn post_is_retried_on_429_because_nothing_was_processed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/clients"))
        .respond_with(ResponseTemplate::new(429))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/clients"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": { "id": "new" } })))
        .mount(&server)
        .await;

    let created = client(&server)
        .with_retry_policy(quick_retry())
        .create_client(&Default::default())
        .await
        .expect("a throttled create should be retried");
    assert_eq!(created.id.as_deref(), Some("new"));
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn client_errors_are_never_retried() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/clients/abc"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({ "message": "not found" })))
        .mount(&server)
        .await;

    let err = client(&server)
        .with_retry_policy(quick_retry())
        .get_client("abc")
        .await
        .expect_err("404 is not transient");
    assert!(matches!(err, Error::Api { status, .. } if status == 404));
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn retries_are_bounded_by_the_policy() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/clients"))
        .respond_with(ResponseTemplate::new(502))
        .mount(&server)
        .await;

    let err = client(&server)
        .with_retry_policy(RetryPolicy::new(2).base_delay(Duration::from_millis(1)))
        .list_clients(&ListParams::new())
        .await
        .expect_err("should give up and surface the last error");
    assert!(matches!(err, Error::Api { status, .. } if status == 502));
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        3,
        "one initial attempt plus two retries"
    );
}

#[tokio::test]
async fn retry_policy_none_attempts_once() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/clients"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let _ = client(&server)
        .with_retry_policy(RetryPolicy::none())
        .list_clients(&ListParams::new())
        .await
        .expect_err("retrying is disabled");
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn retry_after_longer_than_max_delay_is_surfaced_rather_than_slept_through() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/clients"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "3600"))
        .mount(&server)
        .await;

    let err = tokio::time::timeout(
        Duration::from_secs(5),
        client(&server)
            .with_retry_policy(quick_retry())
            .list_clients(&ListParams::new()),
    )
    .await
    .expect("must not sleep for the full hour the server asked for")
    .expect_err("the throttle should be reported");
    assert!(matches!(err, Error::Api { status, .. } if status == 429));
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn a_short_retry_after_is_honoured() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/clients"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "0"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/clients"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page(0, 1, 1)))
        .mount(&server)
        .await;

    client(&server)
        .with_retry_policy(quick_retry())
        .list_clients(&ListParams::new())
        .await
        .expect("should wait the requested time and retry");
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn retried_requests_keep_their_body_and_headers() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/clients/abc"))
        .and(header("X-Api-Token", "test-token"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/clients/abc"))
        .and(header("X-Api-Token", "test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": { "id": "abc" } })))
        .mount(&server)
        .await;

    let body = invoiceninjars::models::Client {
        name: "Acme".into(),
        ..Default::default()
    };
    client(&server)
        .with_retry_policy(quick_retry())
        .update_client("abc", &body)
        .await
        .expect("the replayed PUT should carry the same payload");

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 2);
    for request in &sent {
        let json: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(json["name"], "Acme", "body must survive the replay");
    }
}

// --- narrowed response types --------------------------------------------

#[tokio::test]
async fn statics_decodes_into_typed_tables() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/statics"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "currencies": [{
                "id": 1, "name": "US Dollar", "code": "USD", "symbol": "$",
                "precision": 2, "thousand_separator": ",", "decimal_separator": ".",
                "exchange_rate": 1.0, "swap_currency_symbol": false
            }],
            // payment_types is the table that returns a string id; the rest
            // return numbers, and both must land as String.
            "payment_types": [{ "id": "1", "name": "Bank Transfer" }],
            "industries": [{ "id": 3, "name": "Advertising" }],
            "countries": [{ "id": 840, "name": "United States", "iso_3166_2": "US" }],
            "date_formats": [{ "id": 1, "format": "d/M/Y", "format_moment": "DD/MMM/YYYY" }],
            "gateways": [{ "id": 1, "name": "Stripe", "fields": { "nested": true } }],
            "bulk_updates": { "client": ["name"] },
            "license_key": "self-host-only"
        })))
        .mount(&server)
        .await;

    let statics = client(&server)
        .statics()
        .await
        .expect("statics should decode");

    let usd = &statics.currencies[0];
    assert_eq!(usd.id, "1", "a numeric id is normalised to a string");
    assert_eq!(usd.code, "USD");
    assert_eq!(usd.precision, Some(2));
    assert_eq!(
        statics.payment_types[0].id, "1",
        "a string id passes through"
    );
    assert_eq!(statics.industries[0].name, "Advertising");
    assert_eq!(statics.date_formats[0].format, "d/M/Y");
    assert_eq!(
        statics.countries[0].extra["iso_3166_2"], "US",
        "untyped per-table fields stay reachable via extra"
    );
    assert!(statics.gateways.is_array(), "gateways stays untyped");
    assert_eq!(
        statics.extra["license_key"], "self-host-only",
        "unmodeled top-level keys are preserved"
    );
}

#[tokio::test]
async fn statics_tolerates_tables_it_does_not_model() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/statics"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "currencies": null,
            "brand_new_table": [{ "whatever": 1 }]
        })))
        .mount(&server)
        .await;

    let statics = client(&server)
        .statics()
        .await
        .expect("should not hard-fail");
    assert!(statics.currencies.is_empty(), "null decodes as empty");
    assert!(statics.extra.contains_key("brand_new_table"));
}

#[tokio::test]
async fn search_decodes_the_non_elasticsearch_shape() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "clients": [{
                "name": "Acme", "type": "/client",
                "id": "Wpmbk5ezJn", "path": "/clients/Wpmbk5ezJn"
            }],
            "client_contacts": [],
            "invoices": [],
            "projects": [],
            "settings": [{ "name": "Tax Settings", "type": "/settings", "path": "/settings/tax" }]
        })))
        .mount(&server)
        .await;

    let results = client(&server)
        .search(&json!({ "search": "acme" }))
        .await
        .expect("search should decode");

    assert_eq!(results.clients[0].name, "Acme");
    assert_eq!(results.clients[0].id, "Wpmbk5ezJn");
    assert_eq!(results.clients[0].r#type, "/client");
    assert_eq!(results.settings[0].name, "Tax Settings");
    // Buckets only the Elasticsearch path returns are absent, not an error.
    assert!(results.quotes.is_empty());
    assert!(results.vendors.is_empty());
}

#[tokio::test]
async fn search_decodes_the_elasticsearch_shape() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "clients": [],
            "quotes": [{ "name": "Q1", "type": "/quote", "id": "q1", "path": "/quotes/q1" }],
            "purchase_orders": [{ "name": "PO", "type": "/purchase_order", "id": "p1", "path": "/x" }]
        })))
        .mount(&server)
        .await;

    let results = client(&server)
        .search(&json!({ "search": "q" }))
        .await
        .expect("the wider shape should decode too");
    assert_eq!(results.quotes[0].id, "q1");
    assert_eq!(results.purchase_orders[0].name, "PO");
}

#[tokio::test]
async fn health_check_decodes_the_self_hosted_report() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/health_check"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "system_health": true,
            "api_version": "5.8.56",
            "php_version": {
                "minimum_php_version": "8.1",
                "current_php_version": "8.2.10",
                "current_php_cli_version": "8.2.10",
                "is_okay": true,
                "memory_limit": "512M"
            },
            "simple_db_check": true,
            "queue": "database",
            "pdf_engine": "SnapPDF",
            "is_docker": false,
            "extensions": [{ "curl": true }]
        })))
        .mount(&server)
        .await;

    let health = client(&server).health_check().await.expect("should decode");
    assert_eq!(health.system_health, Some(true));
    assert_eq!(health.api_version, "5.8.56");
    assert_eq!(health.queue, "database");
    assert_eq!(
        health
            .php_version
            .expect("php block present")
            .current_php_version,
        "8.2.10"
    );
}

/// The hosted app skips the check and returns a bare message, so the same
/// type has to cover both deployments.
#[tokio::test]
async fn health_check_decodes_the_hosted_message_shape() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/health_check"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "message": "", "errors": []
        })))
        .mount(&server)
        .await;

    let health = client(&server).health_check().await.expect("should decode");
    assert_eq!(
        health.system_health, None,
        "no verdict is reported on hosted"
    );
    assert!(health.api_version.is_empty());
}

/// Regression: the endpoint answers with a `data`-wrapped entity, but this
/// used to be decoded as a bare body, so callers got the envelope.
#[tokio::test]
async fn send_email_unwraps_the_data_envelope() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/emails"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": { "id": "inv1", "number": "INV-0001" }
        })))
        .mount(&server)
        .await;

    let invoice: invoiceninjars::models::Invoice = client(&server)
        .send_email(&json!({ "entity": "invoice", "entity_id": "inv1" }))
        .await
        .expect("should return the refreshed entity");
    assert_eq!(invoice.id.as_deref(), Some("inv1"));
    assert_eq!(invoice.number, "INV-0001");
}

#[tokio::test]
async fn generic_escape_hatches_still_work() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/statics"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "sizes": [] })))
        .mount(&server)
        .await;

    let raw: serde_json::Value = client(&server)
        .statics_as()
        .await
        .expect("statics_as should bypass the typed model");
    assert!(raw["sizes"].is_array());
}
