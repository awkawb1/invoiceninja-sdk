# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Wrappers for every one of the API's 34 `Route::resource` groups, plus
  `users` — 265 methods. See `COVERAGE.md` for the per-endpoint inventory.
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

### Fixed

- `list_all_*` silently truncated results. The server caps page size at
  `min(abs(per_page), 5000)`, so a page shorter than requested did not mean
  the last page — the walk stopped early and returned a partial list as
  though it were complete. `per_page` is now clamped to the server maximum so
  the fallback heuristic holds.
- `list_all_*` with `per_page(0)` never terminated: the check `got < 0` has
  no solution. It now falls back to the default page size, and a `MAX_PAGES`
  backstop raises `Error::PaginationLimit` rather than quietly truncating.
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
