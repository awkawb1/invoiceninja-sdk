//! Request-plumbing tests: pagination, path escaping, headers, timeouts, and
//! error mapping.
//!
//! These run against a `wiremock` server bound to loopback, so they need no
//! network access and no credentials. `tests/model_roundtrip.rs` covers the
//! serde layer; this file covers everything between a method call and the
//! bytes on the wire.

use std::time::Duration;

use invoiceninja_sdk::{models::ListParams, Error, InvoiceNinjaClient};
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

use invoiceninja_sdk::RetryPolicy;

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

    let body = invoiceninja_sdk::models::Client {
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

    let invoice: invoiceninja_sdk::models::Invoice = client(&server)
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

// --- secret handling ----------------------------------------------------

/// `reqwest`'s `Client` Debug prints its default headers, so the only thing
/// keeping the API token out of every `{:?}` log is the sensitive flag set
/// when the header is built. Lock that in.
#[test]
fn debug_output_never_contains_the_token_or_password() {
    let client = InvoiceNinjaClient::new("https://example.com/api/v1", "SUPER_SECRET_TOKEN")
        .unwrap()
        .with_api_password("SUPER_SECRET_PASSWORD")
        .unwrap();

    let rendered = format!("{client:?}");
    assert!(
        !rendered.contains("SUPER_SECRET_TOKEN"),
        "api token leaked into Debug output: {rendered}"
    );
    assert!(
        !rendered.contains("SUPER_SECRET_PASSWORD"),
        "api password leaked into Debug output: {rendered}"
    );
    assert!(
        rendered.contains("Sensitive"),
        "expected the headers to render as redacted, got: {rendered}"
    );
}

// --- error quality ------------------------------------------------------

/// A misconfigured base URL or an intercepting proxy returns HTML with a 200.
/// The bare serde message ("expected value at line 1 column 1") does not hint
/// at that, so the body excerpt is what makes it diagnosable.
#[tokio::test]
async fn decode_failures_quote_the_body_that_arrived() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/clients"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string("<html><body>Login required</body></html>"),
        )
        .mount(&server)
        .await;

    let err = client(&server)
        .list_clients(&ListParams::new())
        .await
        .expect_err("HTML is not a decodable list response");

    let rendered = err.to_string();
    assert!(
        matches!(err, Error::Json { .. }),
        "expected a decode error, got {err:?}"
    );
    assert!(
        rendered.contains("Login required"),
        "the excerpt should name what actually arrived, got: {rendered}"
    );
}

#[tokio::test]
async fn decode_excerpts_are_truncated_and_single_line() {
    let server = MockServer::start().await;
    let huge = format!("<html>\n{}\n</html>", "x".repeat(50_000));
    Mock::given(method("GET"))
        .and(path("/clients"))
        .respond_with(ResponseTemplate::new(200).set_body_string(huge))
        .mount(&server)
        .await;

    let err = client(&server)
        .list_clients(&ListParams::new())
        .await
        .expect_err("should fail to decode");

    let rendered = err.to_string();
    assert!(
        rendered.len() < 400,
        "a huge body must not become a huge error, got {} chars",
        rendered.len()
    );
    assert!(
        !rendered.contains('\n'),
        "the excerpt should be collapsed to one line"
    );
    assert!(rendered.contains('…'), "truncation should be marked");
}

#[tokio::test]
async fn empty_error_bodies_fall_back_to_the_status_reason() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/clients/abc"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let err = client(&server)
        .with_retry_policy(RetryPolicy::none())
        .delete_client("abc")
        .await
        .expect_err("404 is an error");

    match err {
        Error::Api { message, .. } => assert_eq!(
            message, "Not Found",
            "an empty body should borrow the status reason rather than render blank"
        ),
        other => panic!("expected Error::Api, got {other:?}"),
    }
}

#[tokio::test]
async fn a_json_error_body_with_no_message_still_says_something() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/clients"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({})))
        .mount(&server)
        .await;

    let err = client(&server)
        .with_retry_policy(RetryPolicy::none())
        .list_clients(&ListParams::new())
        .await
        .expect_err("403 is an error");
    assert!(err.to_string().contains("Forbidden"), "got {err}");
}

// --- retry budget -------------------------------------------------------

/// The request timeout is per attempt, so without a total budget four
/// attempts at 60s each would take four minutes. The budget caps the call.
#[tokio::test]
async fn the_total_budget_stops_retrying_even_with_retries_left() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/clients"))
        .respond_with(ResponseTemplate::new(503).set_delay(Duration::from_millis(80)))
        .mount(&server)
        .await;

    let policy = RetryPolicy::new(50)
        .base_delay(Duration::from_millis(1))
        .max_delay(Duration::from_millis(5))
        .max_elapsed(Some(Duration::from_millis(250)));

    let started = std::time::Instant::now();
    let err = client(&server)
        .with_retry_policy(policy)
        .list_clients(&ListParams::new())
        .await
        .expect_err("should give up once the budget is spent");

    assert!(matches!(err, Error::Api { status, .. } if status == 503));
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "50 retries were allowed but the budget should have ended it, took {:?}",
        started.elapsed()
    );
    let attempts = server.received_requests().await.unwrap().len();
    assert!(
        (2..=6).contains(&attempts),
        "expected the budget to allow a few attempts, got {attempts}"
    );
}

#[tokio::test]
async fn no_budget_means_only_max_retries_bounds_the_call() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/clients"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let policy = RetryPolicy::new(4)
        .base_delay(Duration::from_millis(1))
        .max_delay(Duration::from_millis(2))
        .max_elapsed(None);

    let _ = client(&server)
        .with_retry_policy(policy)
        .list_clients(&ListParams::new())
        .await
        .expect_err("still fails");
    assert_eq!(server.received_requests().await.unwrap().len(), 5);
}

// --- document uploads ---------------------------------------------------

use invoiceninja_sdk::FileUpload;

fn pdf(name: &str) -> FileUpload {
    FileUpload::new(name, b"%PDF-1.4 fake".to_vec()).mime("application/pdf")
}

fn body_text(request: &wiremock::Request) -> String {
    String::from_utf8_lossy(&request.body).into_owned()
}

async fn upload_ok_server(route: &str) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(path(route))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": { "id": "abc" } })))
        .mount(&server)
        .await;
    server
}

/// The route is registered as PUT, but PHP does not parse multipart on a PUT,
/// so the files would never arrive. The working form is POST with a
/// `_method=PUT` field, which Laravel turns back into a PUT before routing.
#[tokio::test]
async fn upload_is_a_post_that_spoofs_put_with_documents_as_an_array() {
    let server = upload_ok_server("/clients/abc/upload").await;

    let updated = client(&server)
        .upload_client("abc", vec![pdf("receipt.pdf")], None)
        .await
        .expect("upload should succeed");
    assert_eq!(updated.id.as_deref(), Some("abc"), "returns the entity");

    let sent = &server.received_requests().await.unwrap()[0];
    assert_eq!(
        sent.method.as_str(),
        "POST",
        "a real PUT would drop the files"
    );
    let content_type = sent.headers.get("content-type").unwrap().to_str().unwrap();
    assert!(
        content_type.starts_with("multipart/form-data; boundary="),
        "got {content_type}"
    );
    assert_eq!(
        sent.headers.get("x-api-token").unwrap().to_str().unwrap(),
        "test-token"
    );

    let body = body_text(sent);
    assert!(body.contains("name=\"_method\"\r\n\r\nPUT"), "body: {body}");
    assert!(
        body.contains("name=\"documents[]\"; filename=\"receipt.pdf\""),
        "body: {body}"
    );
    assert!(body.contains("Content-Type: application/pdf"));
    assert!(
        body.contains("%PDF-1.4 fake"),
        "file bytes must be in the body"
    );
}

/// Laravel's `boolean` rule accepts only true, false, 0, 1, '0' and '1' under
/// strict comparison, so the string "true" fails validation.
#[tokio::test]
async fn is_public_is_sent_as_one_or_zero_never_true_false() {
    for (flag, expected) in [
        (Some(true), Some("1")),
        (Some(false), Some("0")),
        (None, None),
    ] {
        let server = upload_ok_server("/invoices/abc/upload").await;
        client(&server)
            .upload_invoice("abc", vec![pdf("a.pdf")], flag)
            .await
            .expect("upload should succeed");

        let body = body_text(&server.received_requests().await.unwrap()[0]);
        match expected {
            Some(value) => assert!(
                body.contains(&format!("name=\"is_public\"\r\n\r\n{value}\r\n")),
                "is_public={flag:?} should send {value}, body: {body}"
            ),
            None => assert!(
                !body.contains("is_public"),
                "an unset flag must be omitted, not defaulted, body: {body}"
            ),
        }
        assert!(!body.contains("true") && !body.contains("false"));
    }
}

#[tokio::test]
async fn several_files_become_several_document_parts() {
    let server = upload_ok_server("/expenses/abc/upload").await;
    client(&server)
        .upload_expense("abc", vec![pdf("one.pdf"), pdf("two.pdf")], None)
        .await
        .expect("upload should succeed");

    let body = body_text(&server.received_requests().await.unwrap()[0]);
    assert_eq!(body.matches("name=\"documents[]\"").count(), 2);
    assert!(body.contains("filename=\"one.pdf\"") && body.contains("filename=\"two.pdf\""));
}

/// An upload with nothing in it is answered with a success that changed
/// nothing; that is caught locally instead.
#[tokio::test]
async fn invalid_uploads_are_rejected_before_any_request() {
    let server = upload_ok_server("/clients/abc/upload").await;
    let c = client(&server);

    let none = c.upload_client("abc", vec![], None).await.unwrap_err();
    assert!(matches!(none, Error::InvalidUpload(_)), "got {none:?}");

    let nameless = c
        .upload_client("abc", vec![FileUpload::new("  ", b"x".to_vec())], None)
        .await
        .unwrap_err();
    assert!(
        matches!(nameless, Error::InvalidUpload(_)),
        "got {nameless:?}"
    );

    let bad_mime = c
        .upload_client(
            "abc",
            vec![FileUpload::new("a.txt", b"x".to_vec()).mime("not a mime")],
            None,
        )
        .await
        .unwrap_err();
    assert!(
        matches!(bad_mime, Error::InvalidUpload(_)),
        "got {bad_mime:?}"
    );

    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn upload_ids_get_the_same_path_protection() {
    let server = upload_ok_server("/clients/x/upload").await;
    let err = client(&server)
        .upload_client("..", vec![pdf("a.pdf")], None)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::InvalidPathSegment(_)), "got {err:?}");
    assert!(server.received_requests().await.unwrap().is_empty());
}

/// An upload ADDS a document, so it is not idempotent whatever the HTTP verb.
/// A replay after a lost response would attach the file twice. This holds
/// only because reqwest's multipart body cannot be cloned, which nothing in
/// the types enforces, so it is pinned here for 5xx and 429 alike.
#[tokio::test]
async fn uploads_are_never_replayed() {
    for status in [500u16, 503, 429] {
        let server = MockServer::start().await;
        Mock::given(path("/clients/abc/upload"))
            .respond_with(ResponseTemplate::new(status))
            .mount(&server)
            .await;

        let err = client(&server)
            .with_retry_policy(quick_retry())
            .upload_client("abc", vec![pdf("a.pdf")], None)
            .await
            .expect_err("the failure should surface");
        assert!(
            matches!(err, Error::Api { status: s, .. } if s == status),
            "got {err:?}"
        );
        assert_eq!(
            server.received_requests().await.unwrap().len(),
            1,
            "a {status} on an upload must not be retried"
        );
    }
}

#[tokio::test]
async fn upload_validation_errors_keep_the_field_map() {
    let server = MockServer::start().await;
    Mock::given(path("/clients/abc/upload"))
        .respond_with(ResponseTemplate::new(422).set_body_json(json!({
            "message": "The given data was invalid.",
            "errors": { "documents.0": ["The documents.0 must be a file of type: pdf."] }
        })))
        .mount(&server)
        .await;

    let err = client(&server)
        .upload_client("abc", vec![pdf("a.exe")], None)
        .await
        .unwrap_err();
    match err {
        Error::Api { status, errors, .. } => {
            assert_eq!(status, 422);
            assert!(errors.contains_key("documents.0"));
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}

#[test]
fn file_upload_debug_never_prints_the_contents() {
    let file = FileUpload::new("secret.pdf", b"CONFIDENTIAL-CONTENTS".to_vec());
    let rendered = format!("{file:?}");
    assert!(!rendered.contains("CONFIDENTIAL"), "got {rendered}");
    assert!(rendered.contains("secret.pdf") && rendered.contains("21"));
}

/// The sixteen methods come from one macro, so a typo in a path or a wrong
/// resource name would only show up here.
#[tokio::test]
async fn every_upload_method_targets_its_own_route() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": {} })))
        .mount(&server)
        .await;
    let c = client(&server);
    let f = || vec![pdf("a.pdf")];

    c.upload_client("i", f(), None).await.unwrap();
    c.upload_company("i", f(), None).await.unwrap();
    c.upload_credit("i", f(), None).await.unwrap();
    c.upload_expense("i", f(), None).await.unwrap();
    c.upload_group_setting("i", f(), None).await.unwrap();
    c.upload_invoice("i", f(), None).await.unwrap();
    c.upload_payment("i", f(), None).await.unwrap();
    c.upload_product("i", f(), None).await.unwrap();
    c.upload_project("i", f(), None).await.unwrap();
    c.upload_purchase_order("i", f(), None).await.unwrap();
    c.upload_quote("i", f(), None).await.unwrap();
    c.upload_recurring_expense("i", f(), None).await.unwrap();
    c.upload_recurring_invoice("i", f(), None).await.unwrap();
    c.upload_recurring_quote("i", f(), None).await.unwrap();
    c.upload_task("i", f(), None).await.unwrap();
    c.upload_vendor("i", f(), None).await.unwrap();

    let paths: Vec<String> = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|r| r.url.path().to_string())
        .collect();
    assert_eq!(
        paths,
        [
            "clients",
            "companies",
            "credits",
            "expenses",
            "group_settings",
            "invoices",
            "payments",
            "products",
            "projects",
            "purchase_orders",
            "quotes",
            "recurring_expenses",
            "recurring_invoices",
            "recurring_quotes",
            "tasks",
            "vendors",
        ]
        .map(|resource| format!("/{resource}/i/upload"))
    );
}

// --- transport error diagnosability -------------------------------------

/// `reqwest`'s own message stops at "error sending request for url (...)".
/// The reason is in the source chain, and was invisible to anyone who just
/// printed the error.
#[tokio::test]
async fn transport_errors_say_why_the_request_failed() {
    // Port 1 is reserved and nothing listens on it.
    let client = InvoiceNinjaClient::new("http://127.0.0.1:1/api/v1", "t")
        .unwrap()
        .with_retry_policy(RetryPolicy::none());

    let err = client
        .list_clients(&ListParams::new())
        .await
        .expect_err("nothing is listening");

    assert!(matches!(err, Error::Http(_)), "got {err:?}");
    let rendered = err.to_string().to_lowercase();
    assert!(
        rendered.contains("refused"),
        "the cause should appear in the message, got: {rendered}"
    );
}

// --- per-resource extras ------------------------------------------------

/// Mounts one route answering 200 with `body`, and returns the server.
async fn route_server(verb: &str, route: &str, body: serde_json::Value) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method(verb))
        .and(path(route))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(&server)
        .await;
    server
}

/// Generates one test per extra. Each mounts the route the upstream router
/// registers, calls the method, and checks it decoded the shape the
/// controller returns. `route_server` expects exactly one request, so a call
/// that hit a different path or verb fails when the server drops.
macro_rules! extra_test {
    ($name:ident, $verb:literal, $route:literal, $response:expr,
     |$c:ident| $call:expr, |$got:ident| $check:expr) => {
        #[tokio::test]
        async fn $name() {
            let server = route_server($verb, $route, $response).await;
            let $c = client(&server);
            let $got = $call.await.expect("the extra should succeed");
            $check;
        }
    };
}

fn message() -> serde_json::Value {
    json!({ "message": "ok" })
}

fn entity() -> serde_json::Value {
    json!({ "data": { "id": "abc" } })
}

extra_test!(
    retry_webhook_posts_to_retry_and_returns_the_webhook,
    "POST",
    "/webhooks/abc/retry",
    entity(),
    |c| c.retry_webhook("abc", &json!({ "entity": "invoice", "entity_id": "i1" })),
    |hook| assert_eq!(hook.id.as_deref(), Some("abc"))
);

extra_test!(
    sort_tasks_returns_the_message,
    "POST",
    "/tasks/sort",
    message(),
    |c| c.sort_tasks(&json!({ "status_ids": ["s1"], "task_ids": { "s1": ["t1"] } })),
    |got| assert_eq!(got.message, "ok")
);

extra_test!(
    set_default_design_returns_the_message,
    "POST",
    "/designs/set/default",
    message(),
    |c| c.set_default_design(&json!({ "design_id": "d1", "entity": "invoice" })),
    |got| assert_eq!(got.message, "ok")
);

extra_test!(
    set_default_client_gateway_token_returns_the_token,
    "POST",
    "/client_gateway_tokens/abc/setAsDefault",
    json!({ "data": { "id": "abc", "is_default": true } }),
    |c| c.set_default_client_gateway_token("abc"),
    |token| assert!(token.is_default)
);

extra_test!(
    test_company_gateway_returns_the_message,
    "POST",
    "/company_gateways/abc/test",
    message(),
    |c| c.test_company_gateway("abc"),
    |got| assert_eq!(got.message, "ok")
);

extra_test!(
    clone_company_gateway_returns_the_copy,
    "POST",
    "/company_gateways/abc/clone",
    entity(),
    |c| c.clone_company_gateway("abc"),
    |copy| assert_eq!(copy.id.as_deref(), Some("abc"))
);

extra_test!(
    import_company_gateway_customers_returns_the_message,
    "POST",
    "/company_gateways/abc/import_customers",
    message(),
    |c| c.import_company_gateway_customers("abc"),
    |got| assert_eq!(got.message, "ok")
);

extra_test!(
    remove_bank_account_returns_the_integration,
    "POST",
    "/bank_integrations/remove_account/acc1",
    entity(),
    |c| c.remove_bank_account("acc1"),
    |got| assert_eq!(got.id.as_deref(), Some("abc"))
);

extra_test!(
    fetch_bank_transactions_returns_the_message,
    "POST",
    "/bank_integrations/get_transactions/acc1",
    message(),
    |c| c.fetch_bank_transactions("acc1"),
    |got| assert_eq!(got.message, "ok")
);

extra_test!(
    match_bank_transactions_returns_the_matched_rows,
    "POST",
    "/bank_transactions/match",
    json!({ "data": [{ "id": "bt1" }] }),
    |c| c.match_bank_transactions(&json!({ "transactions": [] })),
    |got| assert_eq!(got.len(), 1)
);

extra_test!(
    update_client_tax_data_returns_the_client,
    "POST",
    "/clients/abc/updateTaxData",
    entity(),
    |c| c.update_client_tax_data("abc"),
    |got| assert_eq!(got.id.as_deref(), Some("abc"))
);

extra_test!(
    client_show_settings_returns_raw_json,
    "POST",
    "/clients/abc/show_settings",
    json!({ "currency_id": "1" }),
    |c| c.client_show_settings("abc"),
    |got| assert_eq!(got["currency_id"], "1")
);

extra_test!(
    client_peppol_discovery_returns_raw_json,
    "POST",
    "/clients/abc/peppol_discovery",
    json!({ "found": false }),
    |c| c.client_peppol_discovery("abc"),
    |got| assert_eq!(got["found"], false)
);

extra_test!(
    check_client_vat_returns_the_message,
    "POST",
    "/clients/abc/check_vat",
    message(),
    |c| c.check_client_vat("abc"),
    |got| assert_eq!(got.message, "ok")
);

extra_test!(
    client_vat_status_is_a_get,
    "GET",
    "/clients/abc/vat_status",
    message(),
    |c| c.client_vat_status("abc"),
    |got| assert_eq!(got.message, "ok")
);

extra_test!(
    client_documents_returns_one_page_of_documents,
    "POST",
    "/clients/abc/documents",
    json!({ "data": [{ "id": "d1" }] }),
    |c| c.client_documents("abc", &json!({ "filter": "x" })),
    |got| assert_eq!(got.len(), 1)
);

/// The body a caller passes must reach the server, not just the route.
#[tokio::test]
async fn retry_webhook_sends_the_callers_body() {
    let server = route_server("POST", "/webhooks/abc/retry", entity()).await;
    client(&server)
        .retry_webhook("abc", &json!({ "entity": "invoice", "entity_id": "i1" }))
        .await
        .unwrap();

    let sent: serde_json::Value = server.received_requests().await.unwrap()[0]
        .body_json()
        .unwrap();
    assert_eq!(sent, json!({ "entity": "invoice", "entity_id": "i1" }));
}

/// The no-body triggers send `{}` with a JSON content type rather than an
/// empty body, which is what Laravel's request parsing is happy with.
#[tokio::test]
async fn no_body_triggers_send_an_empty_json_object() {
    let server = route_server("POST", "/company_gateways/abc/test", message()).await;
    client(&server).test_company_gateway("abc").await.unwrap();

    let request = &server.received_requests().await.unwrap()[0];
    assert_eq!(request.body_json::<serde_json::Value>().unwrap(), json!({}));
    assert_eq!(
        request.headers.get("content-type").unwrap(),
        "application/json"
    );
}

#[tokio::test]
async fn subscription_steps_decode_a_list_or_a_keyed_object_and_check_discards_the_body() {
    let s = route_server(
        "GET",
        "/subscriptions/steps",
        json!([
            { "id": "auth.login-or-register", "dependencies": [] },
            { "id": "payment_method", "dependencies": ["auth.login-or-register"] }
        ]),
    )
    .await;
    let steps = client(&s).subscription_steps().await.unwrap();
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[1].dependencies, ["auth.login-or-register"]);

    // What a real 5.13.50 server sends: an object keyed by PHP class name.
    let s = route_server(
        "GET",
        "/subscriptions/steps",
        json!({
            "App\\Livewire\\Login": { "id": "auth.login", "dependencies": [] },
            "App\\Livewire\\Pay": { "id": "payment_method", "dependencies": ["auth.login"] }
        }),
    )
    .await;
    let mut steps = client(&s).subscription_steps().await.unwrap();
    steps.sort_by(|a, b| a.id.cmp(&b.id));
    assert_eq!(steps[0].id, "auth.login");
    assert_eq!(steps[1].dependencies, ["auth.login"]);

    let s = route_server("POST", "/subscriptions/steps/check", json!([])).await;
    client(&s)
        .check_subscription_steps(&json!({ "steps": "a,b" }))
        .await
        .unwrap();
}

#[tokio::test]
async fn refresh_bank_accounts_ignores_the_meaningless_body() {
    let s = route_server("POST", "/bank_integrations/refresh_accounts", json!({})).await;
    client(&s).refresh_bank_accounts().await.unwrap();
}

#[tokio::test]
async fn extras_validate_ids_and_surface_errors() {
    let server = MockServer::start().await;
    let c = client(&server);

    // A `/` in an id stays inside one segment instead of retargeting the call.
    let _ = c.check_client_vat("a/b").await;
    let seen = server.received_requests().await.unwrap();
    assert_eq!(seen[0].url.path(), "/clients/a%2Fb/check_vat");

    assert!(matches!(
        c.check_client_vat("..").await,
        Err(Error::InvalidPathSegment(_))
    ));
    assert!(matches!(
        c.remove_bank_account("").await,
        Err(Error::InvalidPathSegment(_))
    ));
    assert!(matches!(
        c.test_company_gateway("..").await,
        Err(Error::InvalidPathSegment(_))
    ));

    Mock::given(method("POST"))
        .and(path("/subscriptions/steps/check"))
        .respond_with(ResponseTemplate::new(422).set_body_json(json!({
            "message": "invalid",
            "errors": { "steps": ["bad order"] }
        })))
        .mount(&server)
        .await;
    let err = c
        .check_subscription_steps(&json!({ "steps": "b,a" }))
        .await
        .expect_err("an invalid step list is a 422");
    assert!(
        matches!(err, Error::Api { status, .. } if status == 422),
        "{err:?}"
    );
}

/// These are all `POST`s, and some start work on the server, so a 5xx must
/// not be replayed.
#[tokio::test]
async fn extras_are_not_replayed_on_5xx() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/company_gateways/abc/import_customers"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    client(&server)
        .with_retry_policy(quick_retry())
        .import_company_gateway_customers("abc")
        .await
        .expect_err("a failed trigger must not be retried");
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

// --- password-protected actions -----------------------------------------

extra_test!(
    purge_client_returns_the_message,
    "POST",
    "/clients/abc/purge",
    message(),
    |c| c.purge_client("abc"),
    |got| assert_eq!(got.message, "ok")
);

extra_test!(
    merge_clients_names_the_survivor_first_and_returns_it,
    "POST",
    "/clients/keep/drop/merge",
    json!({ "data": { "id": "keep" } }),
    |c| c.merge_clients("keep", "drop"),
    |got| assert_eq!(got.id.as_deref(), Some("keep"))
);

extra_test!(
    merge_vendors_names_the_survivor_first_and_returns_it,
    "POST",
    "/vendors/keep/drop/merge",
    json!({ "data": { "id": "keep" } }),
    |c| c.merge_vendors("keep", "drop"),
    |got| assert_eq!(got.id.as_deref(), Some("keep"))
);

extra_test!(
    purge_company_returns_the_message,
    "POST",
    "/companies/purge/abc",
    message(),
    |c| c.purge_company("abc"),
    |got| assert_eq!(got.message, "ok")
);

extra_test!(
    purge_company_save_settings_returns_the_message,
    "POST",
    "/companies/purge_save_settings/abc",
    message(),
    |c| c.purge_company_save_settings("abc"),
    |got| assert_eq!(got.message, "ok")
);

extra_test!(
    invite_user_returns_the_message,
    "POST",
    "/users/abc/invite",
    message(),
    |c| c.invite_user("abc"),
    |got| assert_eq!(got.message, "ok")
);

extra_test!(
    new_user_template_is_a_get_returning_a_blank_user,
    "GET",
    "/users/create",
    entity(),
    |c| c.new_user_template(),
    |got| assert_eq!(got.id.as_deref(), Some("abc"))
);

/// User purge answers 204 with no body, which must not be treated as a decode
/// failure.
#[tokio::test]
async fn purge_user_accepts_an_empty_204() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/abc/purge"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    client(&server).purge_user("abc").await.unwrap();
}

/// Both ids in a merge go through the same path protection, so neither can
/// retarget the request.
#[tokio::test]
async fn merge_validates_both_ids() {
    let server = MockServer::start().await;
    let c = client(&server);

    assert!(matches!(
        c.merge_clients("..", "x").await,
        Err(Error::InvalidPathSegment(_))
    ));
    assert!(matches!(
        c.merge_clients("x", "").await,
        Err(Error::InvalidPathSegment(_))
    ));
    assert!(matches!(
        c.merge_vendors("x", "..").await,
        Err(Error::InvalidPathSegment(_))
    ));

    let _ = c.merge_clients("a/b", "c?d").await;
    let seen = server.received_requests().await.unwrap();
    assert_eq!(seen.len(), 1, "only the escaped call may reach the server");
    assert_eq!(seen[0].url.path(), "/clients/a%2Fb/c%3Fd/merge");
}

/// A destructive POST that may have been applied must never be replayed.
#[tokio::test]
async fn destructive_actions_are_not_replayed_on_5xx() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    let c = client(&server).with_retry_policy(quick_retry());

    c.purge_client("abc").await.expect_err("500");
    c.merge_clients("a", "b").await.expect_err("500");
    c.purge_company("abc").await.expect_err("500");
    c.purge_user("abc").await.expect_err("500");
    assert_eq!(server.received_requests().await.unwrap().len(), 4);
}

/// The password reaches the server on these routes, and without one the
/// header is simply absent.
#[tokio::test]
async fn the_api_password_is_sent_on_protected_actions() {
    let server = route_server("POST", "/clients/abc/purge", message()).await;
    client(&server)
        .with_api_password("hunter2")
        .unwrap()
        .purge_client("abc")
        .await
        .unwrap();

    let sent = &server.received_requests().await.unwrap()[0];
    assert_eq!(sent.headers.get("X-API-PASSWORD").unwrap(), "hunter2");
    assert!(sent.headers.get("X-API-PASSWORD-BASE64").is_none());
}

/// A password with a control character cannot be a header value. It used to be
/// rejected with a misleading `InvalidToken`; it now goes out base64 encoded,
/// under the header the server's middleware decodes. The cases cover each
/// padding form (`==`, `=`, none).
#[tokio::test]
async fn a_password_with_a_control_character_goes_out_base64_encoded() {
    for (password, expected) in [("\n", "Cg=="), ("a\n", "YQo="), ("a\nb", "YQpi")] {
        let server = route_server("POST", "/clients/abc/purge", message()).await;
        client(&server)
            .with_api_password(password)
            .expect("any password is accepted")
            .purge_client("abc")
            .await
            .unwrap();

        let sent = &server.received_requests().await.unwrap()[0];
        assert_eq!(
            sent.headers.get("X-API-PASSWORD-BASE64").unwrap(),
            expected,
            "password {password:?}"
        );
        assert!(
            sent.headers.get("X-API-PASSWORD").is_none(),
            "the plain header must not also be sent"
        );
    }
}

/// Non-ASCII text is a valid header value, so those passwords keep going out
/// as `X-API-PASSWORD` exactly as before.
#[tokio::test]
async fn a_non_ascii_password_still_goes_out_as_the_plain_header() {
    let server = route_server("POST", "/clients/abc/purge", message()).await;
    client(&server)
        .with_api_password("pässwörd")
        .unwrap()
        .purge_client("abc")
        .await
        .unwrap();

    let sent = &server.received_requests().await.unwrap()[0];
    assert_eq!(
        sent.headers.get("X-API-PASSWORD").unwrap().as_bytes(),
        "pässwörd".as_bytes()
    );
    assert!(sent.headers.get("X-API-PASSWORD-BASE64").is_none());
}

/// The base64 fallback is as secret as the password itself, so it must be
/// redacted from `Debug` too.
#[tokio::test]
async fn debug_output_redacts_a_base64_password_too() {
    let c = InvoiceNinjaClient::new("http://localhost/api/v1", "tok")
        .unwrap()
        .with_api_password("S3CRETpw\nX")
        .unwrap();
    let shown = format!("{c:?}");
    assert!(!shown.contains("S3CRET"), "{shown}");
    assert!(!shown.contains("UzNDUkVUcHcK"), "{shown}");
    assert!(shown.contains("Sensitive"), "{shown}");
}

// --- activities, email history, previews --------------------------------

extra_test!(
    list_activities_decodes_the_transformer_shape,
    "GET",
    "/activities",
    json!({ "data": [{
        "id": "a1", "activity_type_id": "10", "client_id": "c1", "invoice_id": "",
        "is_system": false, "notes": "", "ip": "127.0.0.1",
        "created_at": 1792869488, "updated_at": 1792869488
    }] }),
    |c| c.list_activities(&ListParams::new()),
    |got| {
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].activity_type_id, "10");
        assert_eq!(got[0].created_at, Some(1792869488));
    }
);

/// The feed is a different shape: numeric type, labelled references.
#[tokio::test]
async fn activity_feed_asks_for_reactv2_and_decodes_labelled_references() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/activities"))
        .and(query_param("reactv2", "1"))
        .and(query_param("rows", "5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": [{
            "id": "7", "hashed_id": "h7", "activity_type_id": 10, "notes": "", "ip": "",
            "created_at": 1792869488,
            "user": { "label": "Ann", "hashed_id": "u1" },
            "invoice": { "label": "0001", "hashed_id": "i1" },
            "payment_amount": "5.00"
        }] })))
        .expect(1)
        .mount(&server)
        .await;

    let feed = client(&server).activity_feed(Some(5)).await.unwrap();
    assert_eq!(feed[0].activity_type_id, 10);
    assert_eq!(feed[0].invoice.as_ref().unwrap().label, "0001");
    assert!(feed[0].client.is_none());
    assert_eq!(feed[0].extra["payment_amount"], "5.00");
}

#[tokio::test]
async fn activity_feed_without_rows_sends_no_rows_parameter() {
    let server = route_server("GET", "/activities", json!({ "data": [] })).await;
    client(&server).activity_feed(None).await.unwrap();

    let url = &server.received_requests().await.unwrap()[0].url;
    assert!(
        !url.query_pairs().any(|(k, _)| k == "rows"),
        "the server picks its own default, got {url}"
    );
}

/// The server tests for the key, not its value, so `rows: null` would count
/// as given and `take(null)` would drop the limit.
#[tokio::test]
async fn entity_activity_omits_rows_unless_given() {
    for (rows, expected) in [
        (None, json!({ "entity": "client", "entity_id": "c1" })),
        (
            Some(3),
            json!({ "entity": "client", "entity_id": "c1", "rows": 3 }),
        ),
    ] {
        let server = route_server("POST", "/activities/entity", json!({ "data": [] })).await;
        client(&server)
            .entity_activity("client", "c1", rows)
            .await
            .unwrap();
        let sent: serde_json::Value = server.received_requests().await.unwrap()[0]
            .body_json()
            .unwrap();
        assert_eq!(sent, expected);
    }
}

extra_test!(
    add_activity_note_returns_the_activity,
    "POST",
    "/activities/notes",
    json!({ "data": { "id": "a9", "notes": "called them" } }),
    |c| c.add_activity_note("invoices", "i1", "called them"),
    |got| assert_eq!(got.notes, "called them")
);

#[tokio::test]
async fn download_activity_entity_returns_the_pdf_bytes() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/activities/download_entity/a1"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"%PDF-1.4".to_vec()))
        .expect(1)
        .mount(&server)
        .await;
    assert_eq!(
        client(&server)
            .download_activity_entity("a1")
            .await
            .unwrap(),
        b"%PDF-1.4"
    );
}

#[tokio::test]
async fn a_missing_activity_backup_is_an_api_error_not_a_decode_failure() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_json(json!({ "message": "No backup exists", "errors": {} })),
        )
        .mount(&server)
        .await;
    let err = client(&server)
        .download_activity_entity("a1")
        .await
        .expect_err("404");
    assert!(
        matches!(err, Error::Api { status, .. } if status == 404),
        "{err:?}"
    );
}

extra_test!(
    client_email_history_decodes_a_bare_array,
    "POST",
    "/emails/clientHistory/c1",
    json!([{ "entity": "invoice", "events": [{ "event": "delivered" }] }]),
    |c| c.client_email_history("c1"),
    |got| assert_eq!(got[0]["events"][0]["event"], "delivered")
);

extra_test!(
    entity_email_history_sends_entity_and_id,
    "POST",
    "/emails/entityHistory",
    json!([]),
    |c| c.entity_email_history("invoice", "i1"),
    |got| assert!(got.is_empty())
);

/// Every preview returns raw bytes (a PDF), never decodes JSON, and is
/// routed to its own path.
#[tokio::test]
async fn previews_return_bytes_from_their_own_routes() {
    let routes = [
        "/preview",
        "/live_preview",
        "/live_design",
        "/preview/purchase_order",
        "/live_preview/purchase_order",
    ];
    for route in routes {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(route))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"%PDF-1.4".to_vec()))
            .expect(1)
            .mount(&server)
            .await;
        let c = client(&server);
        let body = json!({ "entity": "invoice", "entity_id": "-1" });
        let got = match route {
            "/preview" => c.preview(&body).await,
            "/live_preview" => c.live_preview(&body).await,
            "/live_design" => c.live_design(&body).await,
            "/preview/purchase_order" => c.preview_purchase_order(&body).await,
            _ => c.live_preview_purchase_order(&body).await,
        };
        assert_eq!(got.unwrap(), b"%PDF-1.4", "{route}");
    }
}

#[tokio::test]
async fn preview_html_asks_for_html_and_returns_text() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/preview"))
        .and(query_param("html", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html>x</html>"))
        .expect(1)
        .mount(&server)
        .await;
    let html = client(&server)
        .preview_html(&json!({ "entity": "invoice", "entity_id": "-1" }))
        .await
        .unwrap();
    assert_eq!(html, "<html>x</html>");
}

/// Previews can be rejected with a JSON validation error rather than a PDF.
#[tokio::test]
async fn a_rejected_preview_keeps_the_validation_errors() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(422).set_body_json(json!({
            "message": "invalid data access",
            "errors": { "design.design.body": ["bad twig"] }
        })))
        .mount(&server)
        .await;
    let err = client(&server).preview(&json!({})).await.expect_err("422");
    assert!(
        matches!(&err, Error::Api { errors, .. } if errors.contains_key("design.design.body")),
        "{err:?}"
    );
}

/// Adding a note appends every time, so a lost response must not be replayed.
#[tokio::test]
async fn notes_are_not_replayed_on_5xx() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    client(&server)
        .with_retry_policy(quick_retry())
        .add_activity_note("invoices", "i1", "x")
        .await
        .expect_err("500");
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

// --- invitation-key downloads -------------------------------------------

/// Mounts one GET route answering 200 with raw bytes. Expects exactly one
/// request, so a call that built a different path fails when the server drops.
async fn raw_route_server(route: &str, body: &'static [u8]) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(route))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body.to_vec()))
        .expect(1)
        .mount(&server)
        .await;
    server
}

/// One test per invitation-key download. The route each mounts is the one
/// `routes/api.php` registers — note the singular entity segment, which is
/// what these would get wrong if the plural CRUD path were reused.
macro_rules! invitation_download_test {
    ($name:ident, $route:literal, $body:literal, |$c:ident| $call:expr) => {
        #[tokio::test]
        async fn $name() {
            let server = raw_route_server($route, $body).await;
            let $c = client(&server);
            let got = $call.await.expect("the download should succeed");
            assert_eq!(got, $body);
        }
    };
}

invitation_download_test!(
    download_invoice_by_invitation_key_hits_the_singular_route,
    "/invoice/key1/download",
    b"%PDF-1.4",
    |c| c.download_invoice_by_invitation_key("key1")
);

invitation_download_test!(
    download_e_invoice_by_invitation_key_hits_the_singular_route,
    "/invoice/key1/download_e_invoice",
    b"<?xml version=\"1.0\"?>",
    |c| c.download_e_invoice_by_invitation_key("key1")
);

invitation_download_test!(
    download_quote_by_invitation_key_hits_the_singular_route,
    "/quote/key1/download",
    b"%PDF-1.4",
    |c| c.download_quote_by_invitation_key("key1")
);

invitation_download_test!(
    download_e_quote_by_invitation_key_hits_the_singular_route,
    "/quote/key1/download_e_quote",
    b"<?xml version=\"1.0\"?>",
    |c| c.download_e_quote_by_invitation_key("key1")
);

invitation_download_test!(
    download_credit_by_invitation_key_hits_the_singular_route,
    "/credit/key1/download",
    b"%PDF-1.4",
    |c| c.download_credit_by_invitation_key("key1")
);

invitation_download_test!(
    download_e_credit_by_invitation_key_hits_the_singular_route,
    "/credit/key1/download_e_credit",
    b"<?xml version=\"1.0\"?>",
    |c| c.download_e_credit_by_invitation_key("key1")
);

invitation_download_test!(
    download_purchase_order_by_invitation_key_hits_the_singular_route,
    "/purchase_order/key1/download",
    b"%PDF-1.4",
    |c| c.download_purchase_order_by_invitation_key("key1")
);

invitation_download_test!(
    download_e_purchase_order_by_invitation_key_hits_the_singular_route,
    "/purchase_order/key1/download_e_purchase_order",
    b"<?xml version=\"1.0\"?>",
    |c| c.download_e_purchase_order_by_invitation_key("key1")
);

/// These are authenticated like any other call, which is the whole reason
/// they fit the client at all. A token-less variant would need a separate
/// entry point.
#[tokio::test]
async fn an_invitation_download_still_sends_the_api_token() {
    let server = raw_route_server("/invoice/key1/download", b"%PDF-1.4").await;
    client(&server)
        .download_invoice_by_invitation_key("key1")
        .await
        .unwrap();

    let headers = &server.received_requests().await.unwrap()[0].headers;
    assert_eq!(headers["X-Api-Token"], "test-token");
}

/// An unknown key is a `400` carrying `no record found`, not a `404`: the
/// controllers return that directly rather than raising
/// `ModelNotFoundException`.
#[tokio::test]
async fn an_unknown_invitation_key_is_a_400_not_a_404() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/invoice/gone/download"))
        .respond_with(
            ResponseTemplate::new(400).set_body_json(json!({ "message": "no record found" })),
        )
        .mount(&server)
        .await;

    let err = client(&server)
        .download_invoice_by_invitation_key("gone")
        .await
        .expect_err("an unknown key should fail");
    assert!(
        matches!(&err, Error::Api { status, message, .. }
            if status.as_u16() == 400 && message == "no record found"),
        "{err:?}"
    );
}

/// `invoice//download` is a different route, so an empty key must not reach
/// the server.
#[tokio::test]
async fn an_empty_invitation_key_is_refused_before_the_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"%PDF-1.4".to_vec()))
        .mount(&server)
        .await;

    let err = client(&server)
        .download_invoice_by_invitation_key("")
        .await
        .expect_err("an empty key must not be sent");
    assert!(matches!(err, Error::InvalidPathSegment(_)), "got {err:?}");
    assert!(server.received_requests().await.unwrap().is_empty());
}

// --- import and export --------------------------------------------------

#[tokio::test]
async fn export_posts_and_returns_the_collection_url() {
    let server = route_server(
        "POST",
        "/export",
        json!({ "message": "Processing", "url": "https://x.test/protected_download/abc" }),
    )
    .await;

    let job = client(&server)
        .export(&json!({ "send_email": false }))
        .await
        .expect("export should be accepted");
    assert_eq!(job.message, "Processing");
    assert_eq!(job.url, "https://x.test/protected_download/abc");
}

#[tokio::test]
async fn preimport_sends_each_csv_under_its_entity_type() {
    let server = MockServer::start().await;
    Mock::given(path("/preimport"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "hash": "h1",
            "mappings": {
                "client": { "available": { "name": "Name" }, "headers": [["name"]], "hints": [0] }
            }
        })))
        .mount(&server)
        .await;

    let preview = client(&server)
        .preimport(vec![
            ("client".to_string(), pdf("clients.csv")),
            ("invoice".to_string(), pdf("invoices.csv")),
        ])
        .await
        .expect("preimport should succeed");
    assert_eq!(preview.hash, "h1");
    assert!(preview.mappings.contains_key("client"));

    let sent = &server.received_requests().await.unwrap()[0];
    assert_eq!(sent.method.as_str(), "POST");
    let body = body_text(sent);
    // Keyed by entity type, not `documents[]` — a different field name from
    // the `/{id}/upload` routes.
    assert!(
        body.contains("name=\"files[client]\"; filename=\"clients.csv\""),
        "body: {body}"
    );
    assert!(
        body.contains("name=\"files[invoice]\"; filename=\"invoices.csv\""),
        "body: {body}"
    );
    assert!(!body.contains("_method"), "preimport is a real POST");
}

#[tokio::test]
async fn preimport_with_no_files_is_refused_before_the_request() {
    let server = MockServer::start().await;
    Mock::given(path("/preimport"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "hash": "h1" })))
        .mount(&server)
        .await;

    let err = client(&server)
        .preimport(vec![])
        .await
        .expect_err("an empty import must not be sent");
    assert!(matches!(err, Error::InvalidUpload(_)), "got {err:?}");
    assert!(server.received_requests().await.unwrap().is_empty());
}

/// The second phase is ordinary JSON, not multipart: the hash stands in for
/// the files, so there is nothing to encode as a form.
#[tokio::test]
async fn import_sends_json_rather_than_a_form() {
    let server = route_server("POST", "/import", json!({ "message": "Import started" })).await;

    let got = client(&server)
        .import(&json!({ "hash": "h1", "column_map": { "client": { "mapping": {} } } }))
        .await
        .expect("import should be accepted");
    assert_eq!(got.message, "Import started");

    let sent = &server.received_requests().await.unwrap()[0];
    let content_type = sent.headers.get("content-type").unwrap().to_str().unwrap();
    assert_eq!(content_type, "application/json");
    assert_eq!(
        sent.body_json::<serde_json::Value>().unwrap()["hash"],
        "h1",
        "the hash is what stands in for the upload"
    );
}

#[tokio::test]
async fn import_json_sends_one_file_named_files() {
    let server = route_server(
        "POST",
        "/import_json",
        json!({ "message": "Processing", "success": true }),
    )
    .await;

    let got = client(&server)
        .import_json(pdf("backup.zip"))
        .await
        .expect("import_json should be accepted");
    assert_eq!(got.message, "Processing");
    assert_eq!(
        got.extra["success"], true,
        "the success flag is kept in extra"
    );

    let body = body_text(&server.received_requests().await.unwrap()[0]);
    assert!(
        body.contains("name=\"files\"; filename=\"backup.zip\""),
        "singular `files`, not `files[]`: body: {body}"
    );
}

#[tokio::test]
async fn upload_edocument_is_a_post_that_spoofs_put() {
    let server = route_server(
        "POST",
        "/edocument/upload",
        json!({ "message": "Processing...." }),
    )
    .await;

    let got = client(&server)
        .upload_edocument(vec![pdf("einvoice.xml")])
        .await
        .expect("edocument upload should be accepted");
    assert_eq!(got.message, "Processing....");

    let sent = &server.received_requests().await.unwrap()[0];
    assert_eq!(
        sent.method.as_str(),
        "POST",
        "the route is a PUT, but a real PUT drops the files"
    );
    let body = body_text(sent);
    assert!(body.contains("name=\"_method\"\r\n\r\nPUT"), "body: {body}");
    assert!(
        body.contains("name=\"documents[]\"; filename=\"einvoice.xml\""),
        "body: {body}"
    );
}

#[tokio::test]
async fn edocument_uploads_are_never_replayed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    client(&server)
        .with_retry_policy(quick_retry())
        .upload_edocument(vec![pdf("einvoice.xml")])
        .await
        .expect_err("500");
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        1,
        "a multipart body is not cloneable, so it gets one attempt"
    );
}

// --- e-invoicing --------------------------------------------------------

fn discovered() -> serde_json::Value {
    json!({ "discovered": true })
}

extra_test!(
    validate_einvoice_posts_to_validate_entity,
    "POST",
    "/einvoice/validateEntity",
    json!({ "passes": true, "invoices": [], "clients": [] }),
    |c| c.validate_einvoice(&json!({ "entity": "invoices", "entity_id": "i1" })),
    |got| assert_eq!(got["passes"], true)
);

/// A failed check is a 422, so it arrives as an error rather than an `Ok`
/// carrying the failures. Pinned because it is surprising.
#[tokio::test]
async fn a_failing_einvoice_check_is_an_error_not_a_result() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/einvoice/validateEntity"))
        .respond_with(
            ResponseTemplate::new(422).set_body_json(json!({ "passes": false, "invoices": ["x"] })),
        )
        .mount(&server)
        .await;

    let err = client(&server)
        .validate_einvoice(&json!({ "entity": "invoices" }))
        .await
        .expect_err("a failed validation is a 422");
    assert!(
        matches!(&err, Error::Api { status, .. } if status.as_u16() == 422),
        "{err:?}"
    );
}

extra_test!(
    einvoice_configurations_posts_the_settings,
    "POST",
    "/einvoice/configurations",
    json!({}),
    |c| c.einvoice_configurations(&json!({ "payment_means": [] })),
    |got| assert!(got.is_object())
);

extra_test!(
    einvoice_quota_reads_the_remaining_count,
    "GET",
    "/einvoice/quota",
    json!({ "quota": 42 }),
    |c| c.einvoice_quota(),
    |got| assert_eq!(got["quota"], 42)
);

extra_test!(
    einvoice_delivery_map_reads_the_static_map,
    "GET",
    "/einvoice/delivery_map",
    json!({ "DE": { "routable": true } }),
    |c| c.einvoice_delivery_map(),
    |got| assert_eq!(got["DE"]["routable"], true)
);

extra_test!(
    peppol_discovery_posts_the_identifier,
    "POST",
    "/einvoice/peppol/discovery",
    discovered(),
    |c| c.peppol_discovery(&json!({ "identifier": "0088:123" })),
    |got| assert_eq!(got["discovered"], true)
);

extra_test!(
    peppol_legal_entity_is_a_post_despite_being_a_read,
    "POST",
    "/einvoice/peppol/legal_entity",
    json!({ "id": 7, "party_name": "Acme" }),
    |c| c.peppol_legal_entity(&json!({})),
    |got| assert_eq!(got["id"], 7)
);

extra_test!(
    peppol_disconnect_posts_and_tolerates_an_empty_array,
    "POST",
    "/einvoice/peppol/disconnect",
    json!([]),
    |c| c.peppol_disconnect(&json!({})),
    |got| assert!(got.is_array())
);

extra_test!(
    peppol_update_legal_entity_is_a_put,
    "PUT",
    "/einvoice/peppol/update",
    json!([]),
    |c| c.peppol_update_legal_entity(&json!({ "party_name": "Acme" })),
    |got| assert!(got.is_array())
);

extra_test!(
    peppol_add_legal_identifier_returns_the_ok_message,
    "POST",
    "/einvoice/peppol/add_additional_legal_identifier",
    message(),
    |c| c.peppol_add_legal_identifier(&json!({ "identifier": "0088:1" })),
    |got| assert_eq!(got.message, "ok")
);

// A `DELETE` that carries a body, which no other method in the crate does.
extra_test!(
    peppol_remove_legal_identifier_is_a_delete_with_a_body,
    "DELETE",
    "/einvoice/peppol/remove_additional_legal_identifier",
    json!([]),
    |c| c.peppol_remove_legal_identifier(&json!({ "identifier": "0088:1" })),
    |got| assert!(got.is_array())
);

extra_test!(
    peppol_retry_send_returns_the_queued_message,
    "POST",
    "/einvoice/peppol/send",
    json!({ "message": "trying...." }),
    |c| c.peppol_retry_send(&json!({ "entity_id": "i1" })),
    |got| assert_eq!(got.message, "trying....")
);

extra_test!(
    peppol_sg_c5_activate_posts_to_the_nested_path,
    "POST",
    "/einvoice/peppol/sg/c5/activate",
    message(),
    |c| c.peppol_sg_c5_activate(&json!({})),
    |got| assert_eq!(got.message, "ok")
);

extra_test!(
    peppol_sg_c5_deactivate_posts_to_the_nested_path,
    "POST",
    "/einvoice/peppol/sg/c5/deactivate",
    message(),
    |c| c.peppol_sg_c5_deactivate(&json!({})),
    |got| assert_eq!(got.message, "ok")
);

extra_test!(
    peppol_sg_c5_cancel_posts_to_the_nested_path,
    "POST",
    "/einvoice/peppol/sg/c5/cancel",
    message(),
    |c| c.peppol_sg_c5_cancel(&json!({})),
    |got| assert_eq!(got.message, "ok")
);

/// `setup` answers either a URL to send the user to, or a bare 204 when the
/// registration completed outright. Both are successes, so the 204 is `None`
/// rather than a decode error.
#[tokio::test]
async fn peppol_setup_maps_a_204_to_none_and_a_body_to_some() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/einvoice/peppol/setup"))
        .respond_with(ResponseTemplate::new(204))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    assert_eq!(
        client(&server).peppol_setup(&json!({})).await.unwrap(),
        None,
        "an empty 204 is a completed setup, not a failure"
    );

    let server = route_server(
        "POST",
        "/einvoice/peppol/setup",
        json!({ "corppass_url": "https://x.test/corppass" }),
    )
    .await;
    let got = client(&server).peppol_setup(&json!({})).await.unwrap();
    assert_eq!(
        got.unwrap()["corppass_url"],
        "https://x.test/corppass",
        "a body means a further authorisation step"
    );
}

/// The token rotation answers 204 and keeps the new token server-side.
#[tokio::test]
async fn update_einvoice_token_discards_the_empty_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/einvoice/token/update"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    client(&server)
        .update_einvoice_token(&json!({}))
        .await
        .expect("a 204 is a success");
}

#[tokio::test]
async fn einvoice_health_check_accepts_an_empty_object() {
    let server = route_server("GET", "/einvoice/health_check", json!({})).await;
    client(&server)
        .einvoice_health_check()
        .await
        .expect("an empty object is a pass");
}

// --- quickbooks, stripe, calendar, bank feeds ---------------------------

/// The three 204 routes. A JSON decode attempt on an empty body would fail,
/// so these must go through the discarding helper.
#[tokio::test]
async fn the_quickbooks_204_routes_discard_their_bodies() {
    for (route, call) in ["sync", "action", "disconnect"].iter().zip(0..3) {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!("/quickbooks/{route}")))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&server)
            .await;

        let c = client(&server);
        let body = json!({});
        let result = match call {
            0 => c.quickbooks_sync(&body).await,
            1 => c.quickbooks_action(&body).await,
            _ => c.quickbooks_disconnect(&body).await,
        };
        result.unwrap_or_else(|e| panic!("quickbooks/{route} should succeed: {e:?}"));
    }
}

extra_test!(
    quickbooks_reconnect_url_returns_the_one_time_link,
    "POST",
    "/quickbooks/reconnect_url",
    json!({ "reconnect_url": "https://x.test/r", "requires_reconnect": true }),
    |c| c.quickbooks_reconnect_url(),
    |got| assert_eq!(got["reconnect_url"], "https://x.test/r")
);

extra_test!(
    stripe_update_payment_methods_queues_the_refresh,
    "POST",
    "/stripe/update_payment_methods",
    json!({ "message": "Processing" }),
    |c| c.stripe_update_payment_methods(),
    |got| assert_eq!(got.message, "Processing")
);

extra_test!(
    stripe_import_customers_queues_the_import,
    "POST",
    "/stripe/import_customers",
    json!({ "message": "Processing" }),
    |c| c.stripe_import_customers(),
    |got| assert_eq!(got.message, "Processing")
);

extra_test!(
    stripe_verify_passes_the_driver_response_through,
    "POST",
    "/stripe/verify",
    json!({ "verified": true }),
    |c| c.stripe_verify(),
    |got| assert_eq!(got["verified"], true)
);

extra_test!(
    stripe_disconnect_escapes_the_gateway_id,
    "POST",
    "/stripe/disconnect/cg1",
    json!({ "disconnected": true }),
    |c| c.stripe_disconnect("cg1"),
    |got| assert_eq!(got["disconnected"], true)
);

/// The stripe routes are password-protected, so the header has to reach them.
#[tokio::test]
async fn the_stripe_routes_send_the_api_password() {
    let server = route_server(
        "POST",
        "/stripe/import_customers",
        json!({ "message": "Processing" }),
    )
    .await;
    client(&server)
        .with_api_password("secret")
        .unwrap()
        .stripe_import_customers()
        .await
        .unwrap();

    let headers = &server.received_requests().await.unwrap()[0].headers;
    assert_eq!(headers["X-API-PASSWORD"], "secret");
}

extra_test!(
    calendar_connection_unwraps_the_data_envelope,
    "GET",
    "/calendar_connection",
    json!({ "data": { "provider": "google" } }),
    |c| c.calendar_connection(),
    |got| assert_eq!(got["provider"], "google")
);

extra_test!(
    complete_calendar_connection_escapes_the_provider,
    "POST",
    "/calendar_connection/google/complete",
    json!({ "data": { "provider": "google" } }),
    |c| c.complete_calendar_connection("google", &json!({ "code": "x" })),
    |got| assert_eq!(got["provider"], "google")
);

extra_test!(
    calendar_connection_calendars_lists_the_available_ones,
    "GET",
    "/calendar_connection/calendars",
    json!({ "data": { "calendars": [{ "id": "c1" }] } }),
    |c| c.calendar_connection_calendars(),
    |got| assert_eq!(got["calendars"][0]["id"], "c1")
);

extra_test!(
    update_calendar_connection_calendars_is_a_put,
    "PUT",
    "/calendar_connection/calendars",
    json!({ "data": { "calendar_connection": { "id": "x" } } }),
    |c| c.update_calendar_connection_calendars(&json!({ "calendar_ids": ["c1"] })),
    |got| assert_eq!(got["calendar_connection"]["id"], "x")
);

#[tokio::test]
async fn calendar_connection_events_sends_both_dates_as_query_params() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/calendar_connection/events"))
        .and(query_param("from", "2026-01-01"))
        .and(query_param("to", "2026-01-31"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "data": { "events": [{ "id": "e1" }] } })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let got = client(&server)
        .calendar_connection_events("2026-01-01", "2026-01-31")
        .await
        .expect("events should decode");
    assert_eq!(got["events"][0]["id"], "e1");
}

#[tokio::test]
async fn delete_calendar_connection_discards_the_null_payload() {
    let server = route_server(
        "DELETE",
        "/calendar_connection",
        json!({ "data": { "calendar_connection": null } }),
    )
    .await;
    client(&server)
        .delete_calendar_connection()
        .await
        .expect("the disconnect should succeed");
}

extra_test!(
    yodlee_account_status_escapes_the_account_number,
    "POST",
    "/yodlee/status/acc%2F1",
    json!({ "account_status": "ok" }),
    |c| c.yodlee_account_status("acc/1"),
    |got| assert_eq!(got["account_status"], "ok")
);

extra_test!(
    nordigen_institutions_returns_the_bare_array,
    "GET",
    "/nordigen/institutions",
    json!([{ "id": "BANK_X" }]),
    |c| c.nordigen_institutions(),
    |got| assert_eq!(got[0]["id"], "BANK_X")
);

// --- logout, passkeys, login --------------------------------------------

extra_test!(
    logout_posts_and_returns_the_message,
    "POST",
    "/logout",
    message(),
    |c| c.logout(),
    |got| assert_eq!(got.message, "ok")
);

extra_test!(
    list_passkeys_decodes_the_data_envelope,
    "GET",
    "/settings/passkeys",
    json!({ "data": [
        { "id": "p1", "name": "Work laptop", "created_at": "2026-01-01T00:00:00.000000Z", "last_used_at": 1700000000 },
        { "id": "p2", "name": "Phone", "created_at": "2026-02-01T00:00:00.000000Z", "last_used_at": null },
    ] }),
    |c| c.list_passkeys(),
    |got| {
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].last_used_at, Some(1700000000));
        assert_eq!(got[1].last_used_at, None);
    }
);

/// The server answers a delete with `{"message": ...}`, not an empty body,
/// but `delete_passkey` discards it like every other `delete_*`.
#[tokio::test]
async fn delete_passkey_discards_the_message_body() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/settings/passkeys/p1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(message()))
        .expect(1)
        .mount(&server)
        .await;
    client(&server).delete_passkey("p1").await.unwrap();
}

/// `login` has no client yet, so it is an associated function taking the
/// base url directly rather than a method.
#[tokio::test]
async fn login_posts_credentials_and_decodes_the_company_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/login"))
        .and(wiremock::matchers::body_json(json!({
            "email": "me@example.com",
            "password": "correct-horse-battery-staple",
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [
                {
                    "is_owner": true,
                    "is_admin": true,
                    "token": { "id": "t1", "token": "new-api-token" },
                    "company": { "id": "c1" },
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let login = InvoiceNinjaClient::login(
        &server.uri(),
        "me@example.com",
        "correct-horse-battery-staple",
        None,
    )
    .await
    .expect("login should succeed");

    assert_eq!(login.data.len(), 1);
    let token = login.data[0].token.as_ref().expect("token include present");
    assert_eq!(token.token, "new-api-token");
    // The nested company blob is not modelled, but it is not dropped either.
    assert!(login.data[0].extra.contains_key("company"));
}

/// An optional one-time password goes out alongside the credentials when
/// the caller supplies one, and is left out entirely otherwise.
#[tokio::test]
async fn login_sends_one_time_password_only_when_given() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/login"))
        .and(wiremock::matchers::body_json(json!({
            "email": "me@example.com",
            "password": "pw",
            "one_time_password": "123456",
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": [] })))
        .expect(1)
        .mount(&server)
        .await;

    InvoiceNinjaClient::login(&server.uri(), "me@example.com", "pw", Some("123456"))
        .await
        .expect("login should succeed");
}

/// A failed login surfaces the server's validation message like any other
/// non-2xx response.
#[tokio::test]
async fn login_failure_is_an_api_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/login"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "message": "These credentials do not match our records."
        })))
        .mount(&server)
        .await;

    let err = InvoiceNinjaClient::login(&server.uri(), "me@example.com", "wrong", None)
        .await
        .expect_err("400");
    assert!(
        matches!(&err, Error::Api { status, .. } if status.as_u16() == 400),
        "{err:?}"
    );
}
