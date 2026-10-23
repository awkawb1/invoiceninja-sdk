# invoiceninjars

An async Rust client for the [Invoice Ninja](https://www.invoiceninja.com/) v5 API.

## Development

Enter the dev shell (Rust toolchain via `rust-overlay`):

```sh
nix develop
```

## Usage

```rust
use invoiceninjars::{InvoiceNinjaClient, models::ListParams};

let client = InvoiceNinjaClient::new(
    "https://invoicing.co/api/v1", // or your self-hosted base URL
    std::env::var("IN_API_TOKEN")?,
)?;

let clients = client.list_clients(&ListParams::new().per_page(50)).await?;
```

List endpoints are paginated. `list_*` returns a single page; `list_all_*`
follows pagination and returns every matching record:

```rust
let all_clients = client.list_all_clients(&ListParams::new()).await?;
```

Run the example (requires `IN_API_TOKEN`, optionally `IN_BASE_URL`):

```sh
IN_API_TOKEN=... cargo run --example list_clients
```

## Coverage

Clients, Invoices, Quotes, Credits, Payments, Products, Expenses, Projects,
Tasks, and Vendors, each with `list`, `list_all`, `get`, `create`, `update`,
and `delete`. Add more resources by adding a model under `src/models/` and a
`resource_methods!` invocation in `src/client.rs`.
