//! Fills a **disposable** Invoice Ninja instance with one record of each
//! common kind, so `capture_shapes` has real rows to take shapes from.
//!
//! ```sh
//! IN_SMOKE_WRITE_HOST=localhost IN_BASE_URL=http://localhost:8080/api/v1 \
//!     IN_API_TOKEN=... cargo run --example seed_sample_data
//! ```
//!
//! **Never run this against production.** It creates records, advances
//! number sequences, and may fire webhooks. It uses the smoke test's guard:
//! `IN_SMOKE_WRITE_HOST` must equal the host in `IN_BASE_URL`. Nothing is
//! deleted afterwards; the instance is meant to be thrown away.
//!
//! Each step is independent: one that fails (a validation rule this script
//! did not anticipate) is reported and the rest carry on, with anything that
//! needed it skipped.

use std::future::Future;
use std::process::ExitCode;

use invoiceninjars::{Error, InvoiceNinjaClient};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

fn model<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).expect("seed body matches the model")
}

fn describe(error: &Error) -> String {
    match error {
        Error::Api { errors, .. } if !errors.is_empty() => format!("{error} {errors:?}"),
        _ => error.to_string(),
    }
}

/// Runs one creation, reporting the outcome and returning the new id.
async fn step<T>(name: &str, call: impl Future<Output = invoiceninjars::Result<T>>) -> Option<T> {
    match call.await {
        Ok(created) => {
            println!("  created  {name}");
            Some(created)
        }
        Err(e) => {
            println!("  FAILED   {name}: {}", describe(&e));
            None
        }
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let base_url = std::env::var("IN_BASE_URL").expect("set IN_BASE_URL");
    let token = std::env::var("IN_API_TOKEN").expect("set IN_API_TOKEN");
    let host = reqwest::Url::parse(&base_url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string))
        .unwrap_or_default();
    if std::env::var("IN_SMOKE_WRITE_HOST").as_deref() != Ok(host.as_str()) {
        eprintln!("refusing to write: set IN_SMOKE_WRITE_HOST={host} to confirm this instance is disposable");
        return ExitCode::FAILURE;
    }
    let c = InvoiceNinjaClient::new(&base_url, token).expect("client");
    println!("seeding {host}");

    let line =
        json!([{ "product_key": "seed-product", "quantity": 1, "cost": 10.0, "notes": "seed" }]);

    let client_id = step(
        "client",
        c.create_client(&model(json!({
            "name": "seed-client",
            "contacts": [{ "first_name": "Seed", "last_name": "Contact", "email": "seed@example.com" }],
        }))),
    )
    .await
    .and_then(|x| x.id);
    let vendor_id = step(
        "vendor",
        c.create_vendor(&model(json!({ "name": "seed-vendor" }))),
    )
    .await
    .and_then(|x| x.id);

    step(
        "product",
        c.create_product(&model(
            json!({ "product_key": "seed-product", "price": 10.0 }),
        )),
    )
    .await;
    step(
        "tax_rate",
        c.create_tax_rate(&model(json!({ "name": "seed-tax", "rate": 5.0 }))),
    )
    .await;
    step(
        "payment_term",
        c.create_payment_term(&model(json!({ "name": "seed-term", "num_days": 15 }))),
    )
    .await;
    step(
        "expense_category",
        c.create_expense_category(&model(json!({ "name": "seed-category" }))),
    )
    .await;
    step(
        "tag",
        c.create_tag(&model(
            json!({ "name": "seed-tag", "entity_type": "client" }),
        )),
    )
    .await;
    step(
        "group_setting",
        c.create_group_setting(&model(json!({ "name": "seed-group" }))),
    )
    .await;
    step(
        "webhook",
        c.create_webhook(&model(
            json!({ "event_id": "1", "target_url": "https://example.com/seed-hook" }),
        )),
    )
    .await;
    step(
        "expense",
        c.create_expense(&model(
            json!({ "amount": 5.0, "public_notes": "seed", "vendor_id": vendor_id }),
        )),
    )
    .await;
    step(
        "recurring_expense",
        c.create_recurring_expense(&model(
            json!({ "amount": 5.0, "frequency_id": 5, "vendor_id": vendor_id }),
        )),
    )
    .await;

    if let Some(vendor_id) = &vendor_id {
        step(
            "purchase_order",
            c.create_purchase_order(&model(
                json!({ "vendor_id": vendor_id, "line_items": line }),
            )),
        )
        .await;
    }

    let Some(client_id) = client_id else {
        println!("no client, so the client-dependent records are skipped");
        return ExitCode::FAILURE;
    };

    step(
        "location",
        c.create_location(&model(
            json!({ "name": "seed-location", "client_id": client_id, "address1": "1 Seed St" }),
        )),
    )
    .await;
    step(
        "project",
        c.create_project(&model(
            json!({ "name": "seed-project", "client_id": client_id }),
        )),
    )
    .await;
    step(
        "task",
        c.create_task(&model(
            json!({ "description": "seed task", "client_id": client_id }),
        )),
    )
    .await;
    step(
        "quote",
        c.create_quote(&model(
            json!({ "client_id": client_id, "line_items": line }),
        )),
    )
    .await;
    step(
        "credit",
        c.create_credit(&model(
            json!({ "client_id": client_id, "line_items": line }),
        )),
    )
    .await;
    step(
        "recurring_invoice",
        c.create_recurring_invoice(&model(
            json!({ "client_id": client_id, "frequency_id": 5, "line_items": line }),
        )),
    )
    .await;
    // Expected to 500 on 5.13.50: the `recurring_quotes` table has no
    // `design_id` column, so the server's own insert fails. Not an SDK bug.
    step(
        "recurring_quote",
        c.create_recurring_quote(&model(
            json!({ "client_id": client_id, "frequency_id": 5, "line_items": line }),
        )),
    )
    .await;

    let invoice = step(
        "invoice",
        c.create_invoice(&model(
            json!({ "client_id": client_id, "line_items": line }),
        )),
    )
    .await;
    if let Some(id) = invoice.and_then(|i| i.id) {
        // A payment can only be applied to an invoice that has been sent.
        if step("invoice mark_sent", c.mark_invoice_sent(&id))
            .await
            .is_some()
        {
            step(
                "payment",
                c.create_payment(&model(json!({
                    "client_id": client_id,
                    "amount": 5.0,
                    "invoices": [{ "invoice_id": id, "amount": 5.0 }],
                }))),
            )
            .await;
        }
    }
    ExitCode::SUCCESS
}
