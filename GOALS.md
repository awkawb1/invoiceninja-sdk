# Goals

What this crate is trying to be, what it is deliberately not, and what is
still open. The point of writing this down is that several decisions here
look like oversights until you know the reasoning — this is where the
reasoning lives.

## What this is for

A complete, honest async Rust client for the Invoice Ninja v5 API.

*Complete* in that every endpoint the API exposes should eventually be
reachable through it. *Honest* in that it should not quietly lose data,
quietly truncate results, or quietly succeed when it did not.

## Goals

### 1. Never lose data in a round trip

A `get` followed by an `update` must not drop fields the crate does not know
about. Every model carries `#[serde(flatten)] extra`, and the modelled
fields are a curated subset rather than an attempt at the whole schema.

This is why `custom_value1..4`, `custom_surcharge1..4`, and fields with
ambiguous upstream types are intentionally unmodelled — `extra` carries them
losslessly, whereas a wrong concrete type is a hard decode error that breaks
the entire response.

### 2. Tolerate upstream inconsistency, fail on real problems

Invoice Ninja returns `null` for many nominally-string fields, and the same
field name can differ in type across resources (`frequency_id` is a number on
recurring invoices and a string on subscriptions; `/statics` returns
`payment_types.id` as a string and every other table's as a number).

Decoding should absorb that — hence `null_to_default` and `id_as_string` —
without becoming so permissive that genuine errors pass silently.

### 3. Prefer a loud failure to a quiet wrong answer

The bug worth remembering: `list_all_*` used to stop when a page came back
shorter than requested, which the server's `per_page` cap made wrong, so it
returned a partial list indistinguishable from a complete one. Silent
truncation is the worst failure mode this crate can have.

Concretely: pagination terminates on the paginator's own `total_pages`, a
runaway walk raises `Error::PaginationLimit` rather than returning what it
has, and an id that cannot safely be a path segment is rejected instead of
being mangled into a request against a different endpoint.

### 4. Be safe by default where the API handles money

`POST` is never replayed on a 5xx or a timeout, because the server may have
applied it before the response was lost and a retry would create a second
invoice, payment, or email. A 429 is retried for any method, because the
server states it refused the request without processing it.

When a safety default and a convenience conflict here, safety wins, and the
reasoning goes in a comment so it is not later optimised away.

### 5. Make failures diagnosable

An error should say enough to act on. Decode failures carry an excerpt of
the body that actually arrived, because "expected value at line 1 column 1"
does not reveal that a proxy returned an HTML error page. API errors keep
the whole per-field `errors` map, not just the first message.

### 6. Verify against the source, not against assumptions

Field names and types come from Invoice Ninja's `openapi/api-docs.yaml`; the
set of endpoints comes from `routes/api.php` and the controllers, because
the published spec is incomplete for routes. Claims in `COVERAGE.md` are
checked against the upstream repository rather than inferred.

### 7. Stay testable without credentials

The whole suite runs offline: model decoding against fixtures, request
plumbing against a `wiremock` server on loopback. Nobody should need an
Invoice Ninja account to contribute.

## Non-goals

- **Not a port of the official PHP SDK.** It is a useful cross-check, but
  thinner, and two of its behaviours are bugs not to copy. See the
  comparison in `COVERAGE.md`. The divergences — JSON instead of
  `form_params`, keeping the whole validation `errors` map, exposing bulk
  `download` only on PDF-rendering entities — are deliberate.
- **No sync/blocking API.** Async only.
- **No identity map, lazy relationship loading, or transparent entity
  caching.** Three separate refusals with one theme — the client stays
  stateless and the models stay plain data:
  - *Lazy loading.* An `Invoice` that could fetch its own client would have
    to hold a client handle, so it would stop being a `Deserialize` struct.
    That breaks `Default`-constructibility, which every `create_*` relies
    on, and it breaks the `extra` round-trip model.
  - *Dirty tracking.* The same problem from the other side: you cannot
    meaningfully diff a flattened `Map` of fields the crate does not model,
    and carrying those fields is the whole point of `extra` (goal 1).
  - *Entity caching.* The shared response path in `BaseController` sends no
    `ETag`, `If-None-Match`, or `Cache-Control`, so a cached entity cannot
    be cheaply revalidated — every hit is a gamble. Serving a stale invoice
    balance is a correctness hazard, not a missed optimisation.

  An identity map would also mean shared mutable state; `InvoiceNinjaClient`
  is currently cheap to `Clone` across tasks, and a lock would change that.
  Server-side eager loading via `include` already covers the common case in
  one round trip.

  Note this does **not** rule out typed `include` helpers or an opt-in cache
  for the static lookup tables — see the open questions.
- **No modelling of every field.** See goal 1; breadth of endpoints is a
  goal, breadth of typed fields is not.
- **No retrying of non-idempotent requests.** See goal 4.
- **No API stability before 1.0.** See below.

## Publication and compatibility

Published on crates.io so it is installable, with **no stability promise
before 1.0**. Pin an exact version; expect breaking changes in minor
releases. `CHANGELOG.md` records them, and breaking changes should be
legible there rather than discovered at compile time.

**Gitea is the source of truth; GitHub is a one-way mirror.** The canonical
repository is `gitea.awkawb.cloud/awkawb/invoiceninja-sdk`. A Gitea push mirror
copies every commit to <https://github.com/awkawb1/invoiceninja-sdk>, which
is what `Cargo.toml` points at. Nothing should be written to the GitHub
copy: the next sync overwrites it, so a merged pull request there would
simply disappear. Contributions go through Gitea.

`Error` is `#[non_exhaustive]` so that adding variants is not itself a
breaking change.

**MSRV is 1.88**, set by the dependency graph rather than by this crate's
code. It may rise when dependencies require it; that is a changelog entry,
not a blocker. It should never rise because a convenience API was used
without checking — see the note in `CLAUDE.md`.

## Roadmap: closing the coverage gaps

All 34 `Route::resource` groups are wrapped. Full coverage is the target, so
`COVERAGE.md`'s gap list is a to-do list, not a permanent boundary. Roughly
in order of value over effort:

**Done:** multipart document uploads, for the 16 resources that have the
route (`upload_client`, `upload_invoice`, …). This earlier claimed it would
also make `create_document` possible, which was wrong: upstream
`DocumentController::store` is an empty stub, and an upload is the *only*
way a document is created. There is no `create_document` and there will not
be one. `edocument/upload` was left out because it imports an expense from
an e-document rather than attaching a file, so it belongs with item 6.

**Done:** the per-resource extras — webhook retry, task sort, default
design, default gateway token, subscription steps, and the client,
bank-integration, and company-gateway routes (19 methods). The numbering
below keeps its original item numbers. Running the smoke test against a real
server overturned one reading of the source here: `/subscriptions/steps`
answers with an object keyed by class name, not the array the controller
suggests. That is the case for keeping the real-server tier.

**Done:** the password-protected actions — client purge and merge, vendor
merge, company purge, user invite and purge, and the blank-user template (8
methods). They are verified against the upstream controllers and mocks only;
the smoke test has a password tier for the first real-server run
(`IN_API_PASSWORD`), which has not happened yet. The claim here that
`with_api_password` already made them reachable was not quite true: a
password with a control character was refused locally.

**Done:** activities, email history and previews (14 methods, two new
models). The email history stays raw JSON, since no instance here had sent
mail to take a shape from; promoting it to a model needs a sample. Several
previews are unverified against a real server (see `COVERAGE.md`).

Still to do:

5. **Public invitation-key downloads** — eight routes that take no auth
   token, so they do not fit the authenticated request path and need a
   different entry point.
6. **Import/export and the integrations** — `/import`, `/export`,
   `edocument/upload`, Stripe, QuickBooks, einvoice (17 routes), Yodlee,
   Nordigen, calendar. The largest block and the least SDK-shaped; worth
   doing last and possibly in pieces.

Keep `COVERAGE.md` current as these land — it names exact counts, so it goes
stale quietly.

## Open questions

These are genuinely undecided. Deciding one is a change to this file.

- **Auth routes** (`/login`, `/logout`, `/refresh`, passkeys) are in the gap
  list, but they are a different authentication model from the API token
  this client is built around. Including them may mean a second client type
  rather than more methods on this one. Not obviously in scope.
- **`chart` is still generic** over `T: DeserializeOwned`, because the
  eleven chart endpoints share no response shape. Per-chart structs would
  be eleven small models of unstable payloads; unclear whether that earns
  its keep.
- **Large bodies are buffered entirely into memory.** PDF downloads and CSV
  report exports come back as `Vec<u8>`, and an uploaded `FileUpload` holds
  its whole contents until the request finishes. Streaming would be better
  for big files in both directions but changes those signatures — and for
  uploads it would also change the retry story, which currently depends on
  the multipart body being un-cloneable.
- **`Error::PaginationLimit` has no test**, because reaching it needs
  10,001 mock round trips. Making `MAX_PAGES` injectable purely for a test
  may be worse than the gap.
- **Typed `include` helpers.** Eager loading is a stringly parameter today
  (`.include("client,payments")`), so a typo is a silent no-op rather than a
  compile error. An enum-backed builder per resource would fix that, and it
  conflicts with nothing above — it is just not done. The cost is knowing
  which relationships each resource actually accepts, which the spec does
  not reliably document.
- **An opt-in cache for `/statics`.** The payload is large and near-constant
  and the docs already say to cache it, but the crate offers no hook. A
  small trait the caller implements would stay clear of the entity-caching
  hazard above, since the lookup tables are not money. Whether that beats
  "cache it yourself" is genuinely unclear.

  Related, and deliberately parked: the API accepts an `updated_at` query
  parameter meaning "everything changed since T", which is the shape a local
  mirror would need. That is a sync engine rather than a cache, and a
  different product from this crate — recorded so the option is not
  forgotten rather than because it is planned.
- **When to go 1.0**, and what the API needs to settle first.
