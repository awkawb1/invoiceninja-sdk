# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Import and export, five methods: `export`, `preimport`, `import`,
  `import_json`, and `upload_edocument`. A CSV import is two calls —
  `preimport` uploads the files as `files[<entity_type>]` and returns the
  column mapping the server suggests, then `import` starts the ingest from
  the returned `hash`. The route's one-shot form (files and mapping in a
  single multipart request) is deliberately not wrapped: a mapping is nested,
  and bracket-encoding nested structures is what this crate avoids by sending
  JSON. `upload_edocument` imports expenses from e-documents and is not a
  document attachment, which is why it is not an `upload_*` method. Adds
  `ExportJob` (the export's collection URL expires in 1–5 hours) and
  `ImportPreview`. All of these queue work, so a success means accepted, not
  done.
- Invitation-key downloads, eight methods:
  `download_invoice_by_invitation_key` and
  `download_e_invoice_by_invitation_key`, plus the same pair for quotes,
  credits, and purchase orders. These take the per-contact invitation key
  from a client-portal link rather than a record id, and return the PDF or
  the XML e-document as bytes. Prefer them over the id-based `download_*`
  when the document is for a particular contact: the server renders in that
  contact's locale, so a client with contacts in two languages gets a
  different document per key. An unknown key is a `400` carrying `no record
  found`. `COVERAGE.md` previously listed these as out of scope on the
  grounds that they need no auth token; that was a misreading — they are
  ordinary authenticated `/api/v1` routes.
- Document uploads: `upload_client`, `upload_invoice`, `upload_expense`, and
  thirteen more, one for each resource with an `/{id}/upload` route. They
  take a `Vec<FileUpload>` and an optional `is_public`, and return the
  refreshed record. Uploads are never retried, since a replay could attach
  the file twice. This is the only way the API creates a document, so there
  is no `create_document`. Enables reqwest's `multipart` feature, which adds
  `mime`, `mime_guess`, and `unicase` and leaves the MSRV at 1.88.
- Per-resource extras, nineteen methods: `retry_webhook`, `sort_tasks`,
  `set_default_design`, `set_default_client_gateway_token`,
  `subscription_steps`, `check_subscription_steps`, the company-gateway
  `test`/`clone`/`import_customers` trio, four bank-integration and
  bank-transaction routes, and six client routes (`update_client_tax_data`,
  `client_show_settings`, `client_peppol_discovery`, `check_client_vat`,
  `client_vat_status`, `client_documents`). Each is a `POST` unless noted in
  `COVERAGE.md`, so none is retried on a 5xx. Adds `MessageResponse` for the
  `{"message": ...}` replies and `SubscriptionStep`.
- Password-protected actions, eight methods: `purge_client`, `merge_clients`,
  `merge_vendors`, `purge_company`, `purge_company_save_settings`,
  `invite_user`, `purge_user`, and `new_user_template`. They need
  `with_api_password`, most cannot be undone, and none is retried on a 5xx.
  A merge keeps the first id and folds the second into it. Checked against
  the upstream controllers and mock servers; not yet against a real instance.
- Activity log, email history and previews, fourteen methods:
  `list_activities`, `list_all_activities`, `activity_feed`,
  `entity_activity`, `add_activity_note`, `download_activity_entity`,
  `client_email_history`, `entity_email_history`, and the previews `preview`,
  `preview_html`, `live_preview`, `live_design`, `preview_purchase_order` and
  `live_preview_purchase_order`. Adds `Activity`, `ActivityFeedEntry` and
  `ActivityRef`. Email history is returned as raw JSON. See `COVERAGE.md` for
  which were confirmed against a real server and the easy mistakes (a
  singular entity name in one route, a plural in the next).
- `FileUpload` and `Error::InvalidUpload`. An upload with no files, a nameless
  file, or an unparseable MIME type is refused locally, because the server
  answers an empty upload with a success that changed nothing.
- A declared minimum supported Rust version of **1.88**, verified by running
  the test suite against it. The floor comes from the dependency graph
  (`reqwest` → `url` → `idna` → `icu_*`), not from this crate's own code.
  CI builds on both the MSRV and the pinned dev toolchain, plus a weekly
  advisory job on latest stable that reports breakage from a new Rust
  release. The advisory job fails visibly rather than silently; it is kept
  non-blocking through branch protection rather than `continue-on-error`.
- Wrappers for every one of the API's 34 `Route::resource` groups, plus
  `users` — 325 methods (including document uploads, per-resource extras, password-protected actions, and activity, email-history and preview routes). See `COVERAGE.md` for the per-endpoint inventory.
- `COVERAGE.md`, a detailed record of what is wrapped, what is not, and why,
  checked against `routes/api.php` on the `v5-stable` branch.
- Retry with full-jitter exponential backoff, configurable through
  `RetryPolicy` and `InvoiceNinjaClient::with_retry_policy`. Defaults to 3
  retries. `Retry-After` is honoured when the server sends it.
- `InvoiceNinjaClient::with_timeout` and a `DEFAULT_TIMEOUT` of 60 seconds
  applied to every request.
- Typed responses for previously untyped endpoints: `Statics`, `StaticItem`,
  `Currency`, `DateFormat`, `SearchResults`, `SearchHit`, `HealthCheck`, and
  `PhpVersion`.
- `statics_as`, `search_as`, and `health_check_as` as generic escape hatches
  for callers who need to pin a shape the typed models leave in `extra`.
- `tests/client_requests.rs`: 50 tests covering the request plumbing against
  a loopback `wiremock` server. No network or credentials required.
- `RetryPolicy::max_elapsed`, a ceiling on the time one call may spend across
  all attempts, defaulting to 120 seconds. The request timeout is per
  attempt, so without this the default policy could spend four minutes on a
  call that times out repeatedly.

### Changed

- `statics`, `search`, and `health_check` return concrete types instead of
  being generic over `T: DeserializeOwned`.
- `send_email` now unwraps the `data` envelope, so `T` is the entity
  (`Invoice`, `Quote`, …) rather than a wrapper the caller had to model. It
  stays generic because the endpoint serves all four entity types.
- `chart` remains generic, deliberately: the eleven chart endpoints share no
  common response shape, so there is nothing to narrow to.
- The base URL is parsed and validated when the client is constructed. A
  missing scheme used to produce a 404 on every call instead of one error.
- `list_all_*` terminates on `meta.pagination.total_pages` when the server
  sends it, falling back to the short-page heuristic only when it does not.
- `Error` is now `#[non_exhaustive]`, so later variants are not a breaking
  change. Matches on it need a catch-all arm.
- `Error::Json` is now a struct variant carrying an `excerpt` of the body
  that failed to decode. The serde message alone ("expected value at line 1
  column 1") does not reveal that the response was, say, an HTML error page
  from a proxy or a login redirect from a wrong base URL.
- API errors with an empty or message-less body now report the status reason
  ("Not Found") instead of rendering blank.
- `Error::Http` now renders its whole cause chain. `reqwest`'s own message
  stops at "error sending request for url (...)", leaving out the part that
  says why — "Connection refused", a DNS failure, a TLS handshake error — so
  anyone printing the error had no way to tell which it was.

### Fixed

- `list_all_*` silently truncated results. The server caps page size at
  `min(abs(per_page), 5000)`, so a page shorter than requested did not mean
  the last page — the walk stopped early and returned a partial list as
  though it were complete. `per_page` is now clamped to the server maximum so
  the fallback heuristic holds.
- `list_all_*` with `per_page(0)` never terminated: the check `got < 0` has
  no solution. It now falls back to the default page size, and a `MAX_PAGES`
  backstop raises `Error::PaginationLimit` rather than quietly truncating.
- Creating a recurring invoice or recurring expense failed after the record
  had been created: a real server returns `frequency_id` as the string `"5"`
  and the field was a strict integer. It now accepts a number or a numeric
  string.
- Creating a purchase order, location, or webhook from a default model was
  rejected by a real server. An unset `PurchaseOrder.status_id`,
  `Location.country_id`, or `Webhook.format` was sent as `""`, which the
  server turns into `NULL` and then refuses (a 500 for the purchase order and
  webhook, a 422 for the location). They are now left out of the request when
  empty.
- `with_api_password` refused a password containing a control character (a
  newline, say) with a misleading `InvalidToken` error. Such a password is
  now sent base64-encoded as `X-API-PASSWORD-BASE64`, the form the server's
  middleware accepts for exactly this.
- A payment whose `invoices` entry had a `null` or missing `invoice_id` or
  `amount` failed to decode the whole payment. Both now default.
- `list_system_logs` failed to decode against a real server. `SystemLog`'s
  `created_at` and `updated_at` were typed as strings, following the OpenAPI
  spec, but the server sends integer timestamps. They are now `i64`, like
  every other model.
- Requests could hang indefinitely. `reqwest` applies no timeout of its own
  and none was configured.
- Ids were interpolated into request paths unescaped, so a `/`, `?`, or `#`
  in an id changed which endpoint was addressed.

### Security

- Ids used as URL path segments are escaped, and ids that are empty or
  consist only of dots are rejected. Escaping alone is insufficient: the
  WHATWG rules the `url` crate implements decode `%2E` back to `.` before
  resolving dot segments, so an escaped `..` still collapsed the path. An
  empty id would have addressed the collection route, making
  `delete_client("")` mean `DELETE /clients`.
- `POST` is never replayed on a 5xx or timeout, since the server may have
  applied it before the response was lost and a replay would duplicate an
  invoice, payment, or email. A 429 is the exception — the server states it
  refused the request, so no work was done.
- The API token and password are held as headers marked sensitive, so
  neither appears in `Debug` output. `reqwest`'s own `Client` debug prints
  its default headers, so this is the only thing keeping the token out of
  logs; there is now a regression test pinning it.

## [0.1.0] - 2026-10-23

### Added

- Initial release: async client for the Invoice Ninja v5 API covering
  clients, invoices, quotes, credits, payments, products, expenses,
  projects, tasks, and vendors.
