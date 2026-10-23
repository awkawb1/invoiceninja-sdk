# invoiceninjars

An async Rust client for the [Invoice Ninja](https://www.invoiceninja.com/) v5 API.

Requires Rust 1.88 or newer.

## Development

Enter the dev shell (Rust toolchain via `rust-overlay`):

```sh
nix develop
```

The dev toolchain is pinned in `flake.nix` rather than tracking latest
stable, so it only moves deliberately. To check the crate still builds on
the declared minimum of 1.88:

```sh
nix develop .#msrv --command cargo test
```

## Usage

```rust
use invoiceninjars::{InvoiceNinjaClient, models::ListParams};

// Hosted app:
let client = InvoiceNinjaClient::hosted(std::env::var("IN_API_TOKEN")?)?;

// Self-hosted — note the base URL must include the `/api/v1` suffix:
let client = InvoiceNinjaClient::new(
    "https://your-domain.example/api/v1",
    std::env::var("IN_API_TOKEN")?,
)?;

let clients = client.list_clients(&ListParams::new().per_page(50)).await?;
```

`ListParams` has typed setters for the parameters every list endpoint
accepts, plus `param` for the per-resource filters:

```rust
let unpaid = client
    .list_invoices(
        &ListParams::new()
            .per_page(100)
            .sort("date|desc")
            .param("client_id", "abc123")
            .param("client_status", "unpaid"),
    )
    .await?;
```

List endpoints are paginated. `list_*` returns a single page; `list_all_*`
follows pagination and returns every matching record:

```rust
let all_clients = client.list_all_clients(&ListParams::new()).await?;
```

Every request carries a 60-second deadline by default, since `reqwest`
applies none of its own, and transient failures are retried up to 3 times
with exponential backoff. Both are configurable:

```rust
use std::time::Duration;
use invoiceninjars::RetryPolicy;

let client = InvoiceNinjaClient::hosted(token)?
    .with_timeout(Duration::from_secs(300))
    .with_retry_policy(RetryPolicy::new(5));
```

Note that the timeout applies to each attempt, not to the call as a whole;
`RetryPolicy::max_elapsed` (120 seconds by default) is what bounds the total.

Retrying is deliberately conservative about `POST`. A `POST` that fails with
a 5xx or times out is **not** replayed, because the server may have applied
it before the response was lost and a repeat would create a second invoice
or payment. A 429 is retried for any method, since the server states it
refused the request without processing it. Pass `RetryPolicy::none()` to
disable retrying entirely.

Run the example (requires `IN_API_TOKEN`, optionally `IN_BASE_URL`):

```sh
IN_API_TOKEN=... cargo run --example list_clients
```

## Coverage

Every one of the API's 34 resource groups, plus users — 265 methods. Each
resource has `list`, `list_all`, `get`, `create`, `update`, `delete`, and
(where the API offers one) `bulk`:

| Area | Resources |
| --- | --- |
| Billing | Clients, Invoices, Quotes, Credits, Payments, Products, Purchase Orders |
| Recurring | Recurring Invoices, Recurring Quotes, Recurring Expenses, Subscriptions |
| Work | Projects, Tasks, Task Statuses, Task Schedulers |
| Spend | Expenses, Expense Categories, Vendors, Bank Transactions |
| Banking | Bank Integrations, Bank Transaction Rules, Company Gateways, Client Gateway Tokens |
| Admin | Companies, Users, Group Settings, Tokens, System Logs |
| Settings | Tax Rates, Payment Terms, Webhooks, Designs, Locations, Tags |

Plus documents, entity actions and PDF downloads, reports, charts, search,
statics, email sending, refunds, and health checks.

**See [COVERAGE.md](COVERAGE.md) for the full endpoint-by-endpoint
inventory**, including everything that is deliberately *not* wrapped and why.

Models cover a curated subset of each schema; any field the API returns that
isn't modeled explicitly (including `custom_value1..4` and
`custom_surcharge1..4`) is preserved in that model's `extra` map, so a `get`
followed by an `update` round-trips without dropping data.

### Beyond CRUD

Bulk actions return the affected records. They're available on every
resource the API gives a `/bulk` route — all but companies, system logs, and
client gateway tokens:

```rust
let archived = client.bulk_invoices("archive", &["abc123", "def456"]).await?;
```

Some endpoints — reading or writing an individual user, `/users/bulk`,
purging a client or company — sit behind the API's `password_protected`
middleware and need the account password on top of a valid token:

```rust
let client = InvoiceNinjaClient::hosted(std::env::var("IN_API_TOKEN")?)?
    .with_api_password(std::env::var("IN_API_PASSWORD")?)?;

let user = client.get_user("abc123").await?;
```

Invoices, quotes, credits, and purchase orders also support per-entity
actions and PDF rendering:

```rust
let invoice = client.mark_invoice_sent("abc123").await?;
let pdf: Vec<u8> = client.download_invoice("abc123").await?;

// The bulk `download` action streams one merged PDF. Available for quotes,
// credits, and purchase orders too.
let merged: Vec<u8> = client.download_invoices(&["abc123", "def456"]).await?;
```

Reports return their raw export body (CSV for most of them), and the
schemaless endpoints decode into any type you choose:

```rust
use serde_json::{json, Value};

let csv: Vec<u8> = client.report("invoices", &json!({"date_range": "this_year"})).await?;
let totals: Value = client.chart("totals_v2", &json!({})).await?;
let statics: Value = client.statics().await?;
```

Add more resources by adding a model under `src/models/` and a
`resource_methods!` invocation in `src/client.rs`.
