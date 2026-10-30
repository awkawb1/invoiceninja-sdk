# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

An async Rust client for the [Invoice Ninja](https://www.invoiceninja.com/) v5 API.

**`GOALS.md` states what the project is for, what it deliberately is not,
and what is still undecided.** Read it before making a design call — several
choices in here look like oversights until you know the reasoning (the
curated model subset, the untyped `chart`, not retrying `POST`, the
divergences from the official PHP SDK). If a change would contradict a
stated goal or settle an open question, update `GOALS.md` in the same
change.

## Development environment

This project uses Nix for toolchain management (Rust via `rust-overlay`):

```sh
nix develop
```

`flake.nix` also has `packages.default`: `nix build` compiles with the
pinned dev toolchain and runs the full test suite, including doctests,
inside the Nix sandbox. The crate has no `[[bin]]` target (see the "No
CLI" non-goal in `GOALS.md`), so the resulting store path is empty —
`cargoInstallHook` has no binary to copy out of `target/release`. Treat
`nix build` as a build-and-test gate, not a source of an installable
artifact.

### Rust version

Two versions are pinned, and **three places must agree** when either moves:

| | Version | Declared in |
| --- | --- | --- |
| Dev toolchain | 1.99.1 | `devVersion` in `flake.nix`, `test` job in `ci.yml` |
| MSRV | 1.88 | `msrvVersion` in `flake.nix`, `rust-version` in `Cargo.toml`, `msrv` job in `ci.yml` |

CI has a third job, `stable`, which tracks latest stable and runs weekly
(Mondays, via `schedule`) as well as on push and PR. It exists so a new
Rust release that breaks the build or fires a new clippy lint shows up
before the pin moves. Check it when bumping `devVersion` — if it is red,
the bump needs work first. Nothing needs to stay in sync with it.

**It is advisory, but it reports red.** There is deliberately no
`continue-on-error`, because a green check with an annotation is a signal
nobody reads. That means **it must stay out of the required status checks**
in branch protection — that repo setting, not anything in `ci.yml`, is what
keeps it from blocking merges. Expect it red occasionally through no fault
of the change under test; that is the tradeoff for it being visible.

The pinned `test` and `msrv` jobs skip scheduled runs (`if:
github.event_name != 'schedule'`), since a pinned toolchain against a
locked dependency graph cannot change on a timer.

The MSRV is set by the **dependency graph**, not by this crate's code:
`reqwest` pulls in `url` → `idna` → `idna_adapter` → `icu_*`, and the `icu`
crates require 1.88. Recheck the floor after a dependency bump with:

```sh
cargo metadata --format-version 1 \
  | jq -r '.packages[] | select(.rust_version) | "\(.rust_version)\t\(.name)"' \
  | sort -V | tail -5
```

Verify a candidate MSRV actually builds before declaring it — the number
above was confirmed by running the suite, not inferred:

```sh
CARGO_TARGET_DIR=/tmp/msrv-target nix develop .#msrv --command cargo test
```

Don't reach for a newer std API without checking it against the MSRV; a
convenience like `Option::is_none_or` (1.82) is rarely worth raising the
floor for.

## Common commands

```sh
cargo build
cargo test
cargo test <test_name>       # run a single test
cargo check
cargo clippy
cargo fmt

# Run the example (requires IN_API_TOKEN, optionally IN_BASE_URL)
IN_API_TOKEN=... cargo run --example list_clients
```

Tests live in `tests/` (integration-style, no network access required);
`src/` has no `#[test]` modules. Four files:

- `model_roundtrip.rs` — the serde layer, from hand-written payloads.
- `client_requests.rs` — the request plumbing (pagination, path escaping,
  headers, timeouts, retry, error mapping, one test per endpoint extra)
  against a `wiremock` server on loopback.
- `decode_tolerance.rs` — for every model, serialises `Default`, then nulls
  and removes each field in turn and requires the result to still decode.
  A new model belongs in its list; a new non-`Option` field without
  `null_to_default` fails it.
- `real_shapes.rs` — decodes `tests/fixtures/*.json`, which are the *shapes*
  (keys and JSON types, no values) of responses captured from a real server,
  and checks a decode + re-serialise loses no key. It also refuses fixtures
  that hold any real value, and fixtures with no test.

No credentials or internet needed for any of them.

**Why the fixtures matter:** twice, mocks written from the upstream source
passed while a real server disagreed (system-log timestamps; the
`/subscriptions/steps` shape), and `frequency_id` and several empty-string
fields were wrong the same way. When the smoke test finds a divergence,
refresh the fixtures so it stays caught:

```sh
# 1. Fill a DISPOSABLE instance with one row of each kind (writes!).
IN_SMOKE_WRITE_HOST=localhost IN_BASE_URL=... IN_API_TOKEN=... \
  cargo run --example seed_sample_data
# 2. Capture shapes (GET only). Review `git diff tests/fixtures` before committing.
IN_BASE_URL=... IN_API_TOKEN=... cargo run --example capture_shapes
```

A resource with no rows is skipped, so the fixture set only covers what the
instance held. Bank integrations, company gateways, client gateway tokens,
subscriptions, task schedulers, and recurring quotes are not captured: they
need external accounts, or (recurring quotes on 5.13.50) the server's own
insert fails.

**Fields omitted when empty.** The server runs `ConvertEmptyStringsToNull`,
so an unset `String` serialised as `""` becomes `NULL`, which a non-null
column or an `integer` rule then rejects. `Location.country_id`,
`PurchaseOrder.status_id` and `Webhook.format` therefore carry
`skip_serializing_if = "String::is_empty"`. If a create from a `Default`
model 500s or 422s on an empty field, do the same — and add the path to
`OMITTED_WHEN_EMPTY` in `tests/real_shapes.rs` and `examples/smoke_test.rs`,
which is deliberately a list of decisions rather than a blanket exemption.

### Testing against a real server

Everything above is checked against mock servers written from the upstream
source, so it cannot catch a place where the real server disagrees with that
reading. `examples/smoke_test.rs` can:

```sh
# Read-only (GET requests only). IN_BASE_URL has no default, on purpose.
IN_BASE_URL=http://localhost:8080/api/v1 IN_API_TOKEN=... \
  cargo run --example smoke_test

# Adds a write tier: create a throwaway client, upload files to it, delete it.
IN_SMOKE_WRITE_HOST=localhost IN_BASE_URL=... IN_API_TOKEN=... \
  cargo run --example smoke_test
```

**Never point the write tier at a production server.** The guard is that
`IN_SMOKE_WRITE_HOST` must equal the host in `IN_BASE_URL`, so a write only
reaches a server that was named explicitly. Even after its delete the write
tier leaves a gap in the client number sequence and a soft-deleted row (a
real purge needs the account password), and it may fire webhooks. Use a
disposable instance — a local VM or container — for it.

Beyond "does it decode", the read tier fetches the same rows raw and fails
if a typed row lost a key (`keys kept: <resource>`, names only, never
values), and the write tier creates a purchase order, location, webhook and
recurring invoice from bare `Default` models, since those are what broke.
The read tier also downloads an invoice by invitation key, which is there to
re-check the claim that those routes are token-authenticated; the matching
`download_e_*` is left out, since it renders from e-invoice settings a test
instance need not have.
With `IN_API_PASSWORD` set as well, a password tier merges two throwaway
clients and purges the survivor (a real, permanent delete); it was run
against a disposable 5.13.50 instance on 2026-10-24 and all four checks
pass. `.github/workflows/smoke.yml` runs it on demand (`workflow_dispatch` only,
needing `IN_BASE_URL` and `IN_API_TOKEN` secrets); the write tier runs only if
the `write_host` input names the instance's host.

It prints PASS/FAIL per check and never the contents of a record. The upload
wire format (`POST` plus `_method=PUT`) was read from the framework source,
then confirmed by running the write tier against a disposable Invoice Ninja
5.13.50 instance (2026-10-24): single and multi-file uploads both attached.

## Changelog

`CHANGELOG.md` follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
and semantic versioning. **Update it in the same change that alters
behaviour** — add an entry under `## [Unreleased]`, in the right section
(`Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`), creating
the section if it is not there yet. Entries describe the effect on a user of
the crate, not the implementation: say what broke or changed for a caller and
why, not which function was edited. Purely internal work (refactors, test-only
changes, doc tweaks) does not need an entry.

## Architecture

- `src/client.rs` — `InvoiceNinjaClient` holds a `reqwest::Client` (with the
  `X-Api-Token` auth header baked in) and a `base_url`. Low-level helpers
  (`get_list`, `get_one`, `post_one`, `put_one`, `delete_one`, `post_list`)
  handle request dispatch and decode `ListResponse<T>` / `ItemResponse<T>`
  envelopes, mapping non-2xx responses through `api_error` into `Error::Api`.
- Not every endpoint answers with a JSON envelope, so there are three other
  helper families: `get_raw`/`post_raw` return the body as bytes (PDF streams
  and CSV report exports), `get_unwrapped`/`post_unwrapped` decode a bare
  JSON body with no `data` wrapper (`/statics`, `/charts/*`, `/search`), and
  `get_discard` just checks the status (`/ping`).
- Per-resource methods (`list_clients`, `get_invoice`, `create_product`, etc.)
  are generated by the `resource_methods!` macro at the bottom of
  `client.rs`, which expands to
  `list`/`list_all`/`get`/`create`/`update`/`delete`/`bulk` for a given REST
  path and model type. **To add a new resource**: add a model under
  `src/models/`, re-export it from `src/models/mod.rs`, and add one
  `resource_methods!` invocation in `client.rs`. Note the macro takes seven
  method names before the path and model — adding a resource means passing
  all seven.
- `resource_methods!` delegates the six CRUD methods to `crud_methods!` and
  adds `bulk` on top. **Use `crud_methods!` directly for resources with no
  `/bulk` route** — currently `client_gateway_tokens`, `companies`, and
  `system_logs`. Check `routes/api.php` before assuming a resource has one;
  a generated `bulk_*` against a missing route just 405s.
- Requests go through two chokepoints: `req(method, path)` builds them and is
  the single place the optional `X-API-PASSWORD` header and the per-request
  timeout are attached, and `execute(builder)` dispatches them and is the
  single place retrying happens. **A new helper must use both** — calling
  `self.http` directly would skip the header, the timeout, and the retry
  policy. Only `req` and `execute` should touch `self.http`.
- **The request timeout is per attempt, not per call.** `RetryPolicy::
  max_elapsed` is what bounds a call overall; without it, four attempts at
  the 60s default would take four minutes. Keep both in mind when changing
  either.
- **Both credentials are stored as `HeaderValue`s with
  `set_sensitive(true)`.** `reqwest`'s `Client` Debug prints its default
  headers, so that flag is the only thing keeping the API token out of any
  `{:?}` log. Don't build either header without it; a test pins this.
- `src/retry.rs` — `RetryPolicy` plus the rules for what may be replayed.
  **The method matters, not just the status.** A 429 means the server
  refused the request without processing it, so any method is safe to
  retry; a 5xx or timeout gives no such guarantee, so only idempotent
  methods are. **Don't extend retrying to `POST` on 5xx** — this is a
  billing API, and a replayed create would duplicate an invoice or payment.
- **Interpolate ids into paths with `seg(id)?`, never raw.** It escapes the
  value into a single path segment and rejects ids that are empty or
  dot-only. Both halves matter: escaping stops a `/` or `?` in an id from
  retargeting the request, but escaping *alone* is not enough, because the
  WHATWG rules the `url` crate follows decode `%2E` back to `.` before
  resolving dot segments — so an escaped `..` still collapses the path. The
  exceptions are `report` and `chart`, which take a multi-segment tail by
  design (`chart("project_burnup/<id>", …)`) and must stay unescaped.
- `entity_action_methods!` generates three methods — `{x}_action` and
  `download_{x}` for the `GET /{resource}/{id}/{action}` route, plus
  `download_{plural}` for the bulk merged-PDF action. **Only invoices,
  quotes, credits, and purchase orders have the action route** — don't add it
  elsewhere without checking `routes/api.php`. In particular the official PHP
  SDK puts bulk `download` on *every* resource, which is wrong; only
  PDF-rendering entities support it.
- `invitation_download_methods!` generates `download_{x}_by_invitation_key`
  and `download_e_{x}_by_invitation_key` for the same four resources. Two
  traps, both pinned by tests: the entity segment is **singular**
  (`invoice/{key}/download`, the only route family in the API that is), and
  the routes are **authenticated** despite being the link a client follows —
  they sit in `routes/api.php`'s `api/v1` group under `token_auth`. The
  genuinely token-less invitation downloads are the separate client-portal
  routes in `routes/client.php`, which are not under `/api/v1` and are not
  wrapped. `COVERAGE.md` used to list the `/api/v1` eight as unwrapped for
  that reason, having conflated the two families; don't reintroduce the
  claim.
- `upload_methods!` generates `upload_{x}` for the 16 resources with a
  `/{id}/upload` route; documents can only be created this way, because
  `DocumentController::store` is an empty stub upstream (hence no
  `create_document`). Three things are easy to get wrong, all verified
  against the upstream source: the request is a **`POST` with
  `_method=PUT`** (PHP does not parse multipart on a real `PUT`, and the
  spec's `_method=POST` would 405); `is_public` goes out as **`1`/`0`**
  (Laravel's strict `boolean` rule rejects the string `"true"`); and uploads
  are **never retried**, because reqwest's multipart body is not cloneable
  and `execute` gives an un-cloneable request exactly one attempt — a test
  pins this, since nothing in the types would notice if it changed. Don't
  "fix" any of the three.
- Non-CRUD endpoints (documents, reports, charts, search, statics, emails,
  refunds, ping/health) are hand-written in the last `impl` block rather than
  macro-generated, because their request and response shapes are all
  different.
- **The import, e-invoicing and integration blocks return
  `serde_json::Value` on purpose.** They are pass-throughs from Storecove,
  Stripe, Yodlee or GoCardless, or arrays built for Invoice Ninja's React
  client; none has a documented schema and they move on the provider's
  release schedule. Don't "finish the job" by typing them — the same
  reasoning that keeps `chart` generic applies, and `COVERAGE.md` records it
  as the crate's largest deliberate untyped surface. `ExportJob` and
  `ImportPreview` are typed because their one useful field (the URL, the
  hash) is the whole point of the call.
- Three dispatch helpers exist only for that surface: `send_unwrapped` for a
  `PUT` or `DELETE` that carries a body (`peppol_remove_legal_identifier` is
  the crate's only `DELETE` with one), `post_unwrapped_opt` for a route that
  answers either JSON or a bare `204` (`peppol_setup`), and
  `multipart_unwrapped` for the import uploads, whose form field is
  `files[<entity_type>]` rather than `documents[]` and whose reply has no
  `data` envelope. All three go through `req`/`execute` like everything else.
- `list_*` returns one page; `list_all_*` goes through `get_all_pages`, which
  terminates on `meta.pagination.total_pages` and only falls back to the
  short-page heuristic when the server sends no metadata. **The heuristic is
  not sound on its own**: `BaseController` caps `per_page` at
  `min(abs(per_page), 5000)`, so a page can be shorter than requested while
  records remain — which is why `per_page` is clamped to 5000 before the
  walk starts. `per_page(0)` falls back to the default size (it would make
  the short-page check `got < 0`, which never terminates), the page counter
  advances locally rather than trusting the echoed `current_page`, an empty
  page always ends the walk, and a `MAX_PAGES` backstop raises
  `Error::PaginationLimit` rather than quietly returning a truncated list.
- `src/models/` — one file per resource, plus `common.rs` for shared
  envelope/pagination types (`ListParams`, `ListResponse`, `ItemResponse`,
  `Meta`, `Pagination`).
- Several endpoints share a schema, so some models are type aliases rather
  than separate structs: Invoice Ninja keeps quotes and credits in the
  invoices table (an `entity_type` field distinguishes them), so `Quote` and
  `Credit` alias `Invoice` (`src/models/invoice.rs`); likewise
  `RecurringQuote` aliases `RecurringInvoice`
  (`src/models/recurring_invoice.rs`), whose fields are a strict superset.
  Each alias still gets its own path via the macro.
- Every model struct has a `#[serde(flatten)] pub extra: Map<String, Value>`
  field. This preserves any API fields not explicitly modeled, so a `get`
  followed by an `update` round-trips without silently dropping data. Keep
  this pattern when adding fields or new models.
- **Every non-`Option` field carries
  `#[serde(default, deserialize_with = "super::common::null_to_default")]`,
  not a bare `#[serde(default)]`.** `default` alone only covers a *missing*
  key; a key present with an explicit `null` fails with `invalid type: null,
  expected a string`. Invoice Ninja returns `null` for many
  nominally-string fields, so a bare `default` would break decoding of the
  whole response. `Option<T>` fields keep the plain `#[serde(default)]` —
  they already map `null` to `None`, and that distinction matters (an unset
  `archived_at` should stay `None`, not become `Some(0)`).
- Models deliberately cover a curated subset of each schema and let `extra`
  carry the rest — `custom_value1..4` and `custom_surcharge1..4` are
  intentionally unmodeled. Fields whose JSON type is ambiguous or
  inconsistent in the upstream spec (e.g. `Subscription.currency_id`,
  `webhook_configuration`, `due_date_days`) are also left to `extra`, since a
  wrong concrete type is a hard deserialize error whereas `extra` is lossless.
- Field names and types come from Invoice Ninja's own OpenAPI spec
  (`openapi/api-docs.yaml` on the `v5-stable` branch of the invoiceninja
  repo). Check it before adding or retyping a field; the same name can differ
  in type across resources (`frequency_id` is a number on recurring invoices
  and expenses but a string on subscriptions).
- **The published spec is incomplete for routes**: it documents `/{id}` and
  action paths only for clients, locations, and tags, and omits them for
  invoices and everything else. `routes/api.php` in the same repo is the
  authoritative route list, and the controllers are authoritative for what a
  given action returns. Use the spec for *schemas*, the routes and
  controllers for *endpoints*.
- `src/error.rs` — `Error` enum (`thiserror`) covering transport, JSON
  decode, API error responses (with per-field validation `errors` map), and
  client construction errors. `ApiErrorBody` mirrors Invoice Ninja's error
  JSON shape.
- **A missing record is a `400`, not a `404`.** `Handler.php` maps Laravel's
  `ModelNotFoundException` to a 400 carrying `No query results for model
  [App\Models\X].`, and keeps 404 for a route that does not exist or a method
  it does not support. So don't write a not-found check against 404 — the
  smoke test's post-purge assertion did, and passed only because it had
  never been run. Match the status together with the message.

## Coverage

**All 34 of the API's `Route::resource` groups are wrapped**, plus `users`
(which isn't one). 375 methods: 30 resources with CRUD + bulk, 3 with CRUD
only, 4 with entity actions, 4 with invitation-key downloads, 16 with
document uploads, and 111 hand-written.

Plus the non-CRUD surface: documents, entity actions and PDF downloads for
the four action-capable resources, their invitation-key PDF and e-document
downloads, `report`, `chart`, `search`, `statics`, `send_email`,
`refund_payment`, `ping`, `health_check`, import/export, the 17 e-invoicing
routes, the QuickBooks, Stripe, calendar, Yodlee and GoCardless
integrations, `logout`, passkey list/delete, and a standalone `login`.

**`GOALS.md`'s roadmap is closed** — every route an API-token client can
reach is wrapped. What is left out is left out for a reason, not pending:
passkey registration and passkey login (WebAuthn ceremonies only a browser
can drive), SSO (`oauth_login`, `oidc/*`) and `/login/precheck`,
`/refresh`/`/refresh_react` (the bulk sync-dump shape parked as its own open
question in `GOALS.md`, not an auth concept), `/connected_account` (Gmail
OAuth linking — an integration, not login/logout), the client-portal family
and the OAuth and provider-webhook routes (all outside `/api/v1`, so they
don't fit the client's `base_url` or its token), `POST /quickbooks/settings`
(a registered route with no controller method, so it 500s), and a few
company and user extras.

**`COVERAGE.md` is the detailed inventory** — every wrapped endpoint, every
gap, and the reason for each gap.

Keep `COVERAGE.md` current when adding or removing an endpoint: it names
exact counts and route groups, so it goes stale quietly. The three coverage
summaries — here, `README.md`, and `COVERAGE.md` — should agree;
`COVERAGE.md` is the one with the specifics, the other two summarize and
link to it.

## Relationship to the official PHP SDK

[invoiceninja/sdk-php](https://github.com/invoiceninja/sdk-php) is the
first-party wrapper and a useful cross-check, but it is thinner than this
crate — each endpoint class is just a URI over a shared `BaseEntity`. See the
comparison table in `COVERAGE.md` for what each side covers.

Two divergences are deliberate and shouldn't be "fixed" to match it:

- **We send JSON; it sends `form_params`.** Form encoding bracket-encodes
  nested structures, which is bad for payloads like an invoice's `line_items`.
  The API accepts both.
- **We keep the whole `errors` map on `Error::Api`;** it surfaces only the
  first validation message.

And two of its behaviours are bugs not to copy: it sends `stream: false` in
download payloads (no controller reads a `stream` input), and it exposes
`download` on every resource even though only PDF-rendering entities support
that action.
