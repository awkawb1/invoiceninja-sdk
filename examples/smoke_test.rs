//! Smoke test against a real Invoice Ninja instance.
//!
//! The rest of the suite runs against mock servers written from the upstream
//! source, so it cannot catch a place where the real server disagrees with
//! that reading. This can. It prints PASS/FAIL per check, never the contents
//! of any record, and exits non-zero if anything failed.
//!
//! ```sh
//! # Read-only: GET requests only.
//! IN_BASE_URL=http://localhost:8080/api/v1 IN_API_TOKEN=... \
//!     cargo run --example smoke_test
//!
//! # Adds the write tier. Naming the host is the guard: it must equal the
//! # host in IN_BASE_URL, so a write cannot reach a server you did not name.
//! IN_SMOKE_WRITE_HOST=localhost IN_BASE_URL=... IN_API_TOKEN=... \
//!     cargo run --example smoke_test
//! ```
//!
//! **Do not point the write tier at a production server.** It creates a
//! client, attaches files to it, and deletes it. Even after the delete that
//! leaves a gap in the client number sequence and a soft-deleted row (a real
//! purge needs the account password), and it may fire webhooks.
//!
//! `IN_BASE_URL` has no default, unlike the other example, because defaulting
//! to the hosted app is the wrong failure for a tool that can write.

use std::fmt::Display;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use invoiceninjars::models::{Client, ListParams};
use invoiceninjars::{Error, FileUpload, InvoiceNinjaClient};
use serde_json::json;

#[derive(Default)]
struct Report {
    passed: u32,
    failed: u32,
}

impl Report {
    fn pass(&mut self, name: &str, detail: impl Display) {
        println!("  PASS  {name:<34} {detail}");
        self.passed += 1;
    }

    fn fail(&mut self, name: &str, error: impl Display) {
        println!("  FAIL  {name:<34} {error}");
        self.failed += 1;
    }

    fn check(&mut self, name: &str, ok: bool, detail: impl Display) {
        if ok {
            self.pass(name, detail);
        } else {
            self.fail(name, detail);
        }
    }
}

/// Renders an error with its per-field validation map, which is usually what
/// explains a 422.
fn describe(error: &Error) -> String {
    match error {
        Error::Api { errors, .. } if !errors.is_empty() => format!("{error} {errors:?}"),
        _ => error.to_string(),
    }
}

macro_rules! list {
    ($report:expr, $name:literal, $call:expr) => {
        match $call.await {
            Ok(rows) => $report.pass($name, format!("{} row(s) decoded", rows.len())),
            Err(e) => $report.fail($name, describe(&e)),
        }
    };
}

#[tokio::main]
async fn main() -> ExitCode {
    let base_url = std::env::var("IN_BASE_URL").expect("set IN_BASE_URL (no default, on purpose)");
    let token = std::env::var("IN_API_TOKEN").expect("set IN_API_TOKEN");

    let host = host_of(&base_url);
    let client = match InvoiceNinjaClient::new(&base_url, token) {
        Ok(client) => client,
        Err(e) => {
            eprintln!("could not build client: {e}");
            return ExitCode::FAILURE;
        }
    };

    println!("target: {host}");
    let mut report = Report::default();

    println!("\nread-only checks");
    read_checks(&client, &mut report).await;

    match std::env::var("IN_SMOKE_WRITE_HOST") {
        Ok(allowed) if allowed == host => {
            println!("\nwrite checks (IN_SMOKE_WRITE_HOST matches {host})");
            write_checks(&client, &mut report).await;
        }
        Ok(allowed) => {
            println!(
                "\nwrite checks SKIPPED: IN_SMOKE_WRITE_HOST is `{allowed}` but the target is `{host}`"
            );
        }
        Err(_) => println!("\nwrite checks skipped (set IN_SMOKE_WRITE_HOST to enable)"),
    }

    println!("\n{} passed, {} failed", report.passed, report.failed);
    if report.failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn host_of(base_url: &str) -> String {
    reqwest::Url::parse(base_url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
        .unwrap_or_default()
}

async fn read_checks(client: &InvoiceNinjaClient, report: &mut Report) {
    let small = ListParams::new().per_page(2);

    match client.ping().await {
        Ok(()) => report.pass("ping", "authenticated"),
        Err(e) => report.fail("ping", describe(&e)),
    }

    // Hosted returns a bare message instead of a report; both must decode.
    match client.health_check().await {
        Ok(health) => report.pass(
            "health_check",
            match health.system_health {
                Some(ok) => format!("self-hosted report, system_health={ok}"),
                None => "bare message (no report on this deployment)".to_string(),
            },
        ),
        Err(e) => report.fail("health_check", describe(&e)),
    }

    match client.statics().await {
        Ok(s) => report.check(
            "statics",
            !s.currencies.is_empty() && !s.countries.is_empty(),
            format!(
                "{} currencies, {} countries, {} unmodeled key(s) kept in extra",
                s.currencies.len(),
                s.countries.len(),
                s.extra.len()
            ),
        ),
        Err(e) => report.fail("statics", describe(&e)),
    }

    match client.search(&json!({ "search": "a" })).await {
        Ok(results) => report.pass(
            "search",
            format!(
                "{} client hit(s), {} unmodeled bucket(s)",
                results.clients.len(),
                results.extra.len()
            ),
        ),
        Err(e) => report.fail("search", describe(&e)),
    }

    list!(report, "list_clients", client.list_clients(&small));
    list!(report, "list_invoices", client.list_invoices(&small));
    list!(report, "list_quotes", client.list_quotes(&small));
    list!(report, "list_credits", client.list_credits(&small));
    list!(report, "list_payments", client.list_payments(&small));
    list!(report, "list_products", client.list_products(&small));
    list!(report, "list_expenses", client.list_expenses(&small));
    list!(report, "list_projects", client.list_projects(&small));
    list!(report, "list_tasks", client.list_tasks(&small));
    list!(report, "list_vendors", client.list_vendors(&small));
    list!(
        report,
        "list_recurring_invoices",
        client.list_recurring_invoices(&small)
    );
    list!(
        report,
        "list_recurring_quotes",
        client.list_recurring_quotes(&small)
    );
    list!(
        report,
        "list_recurring_expenses",
        client.list_recurring_expenses(&small)
    );
    list!(
        report,
        "list_purchase_orders",
        client.list_purchase_orders(&small)
    );
    list!(report, "list_tax_rates", client.list_tax_rates(&small));
    list!(
        report,
        "list_expense_categories",
        client.list_expense_categories(&small)
    );
    list!(
        report,
        "list_task_statuses",
        client.list_task_statuses(&small)
    );
    list!(
        report,
        "list_payment_terms",
        client.list_payment_terms(&small)
    );
    list!(
        report,
        "list_subscriptions",
        client.list_subscriptions(&small)
    );
    list!(
        report,
        "list_bank_transactions",
        client.list_bank_transactions(&small)
    );
    list!(report, "list_webhooks", client.list_webhooks(&small));
    list!(
        report,
        "list_bank_integrations",
        client.list_bank_integrations(&small)
    );
    list!(
        report,
        "list_bank_transaction_rules",
        client.list_bank_transaction_rules(&small)
    );
    list!(
        report,
        "list_company_gateways",
        client.list_company_gateways(&small)
    );
    list!(report, "list_designs", client.list_designs(&small));
    list!(
        report,
        "list_group_settings",
        client.list_group_settings(&small)
    );
    list!(report, "list_locations", client.list_locations(&small));
    list!(report, "list_tags", client.list_tags(&small));
    list!(
        report,
        "list_task_schedulers",
        client.list_task_schedulers(&small)
    );
    list!(report, "list_tokens", client.list_tokens(&small));
    list!(
        report,
        "list_client_gateway_tokens",
        client.list_client_gateway_tokens(&small)
    );
    list!(report, "list_companies", client.list_companies(&small));
    list!(report, "list_system_logs", client.list_system_logs(&small));
    list!(report, "list_documents", client.list_documents(&small));
    list!(report, "list_users", client.list_users(&small));

    // The walk follows `meta.pagination.total_pages`, which only a real server
    // can show we are reading correctly. Compare it to one oversized request.
    let everything = ListParams::new().per_page(5000);
    match (
        client
            .list_all_clients(&ListParams::new().per_page(2))
            .await,
        client.list_clients(&everything).await,
    ) {
        (Ok(walked), Ok(single)) if single.len() < 5000 => report.check(
            "pagination walk == single request",
            walked.len() == single.len(),
            format!(
                "walked {} at per_page=2, single request returned {}",
                walked.len(),
                single.len()
            ),
        ),
        (Ok(_), Ok(_)) => report.pass("pagination walk", "skipped: 5000+ clients"),
        (Err(e), _) | (_, Err(e)) => report.fail("pagination walk", describe(&e)),
    }

    // The server caps per_page. Ask for far more than the cap and see how many
    // rows come back, which is the fact the clamp in `get_all_pages` rests on.
    match client
        .list_clients(&ListParams::new().per_page(100_000))
        .await
    {
        Ok(rows) => report.pass(
            "per_page cap",
            format!("asked for 100000, got {} row(s) (cap is 5000)", rows.len()),
        ),
        Err(e) => report.fail("per_page cap", describe(&e)),
    }

    // A single fetch by id, to exercise `get_one` and `seg`.
    match client.list_clients(&small).await {
        Ok(rows) => match rows.first().and_then(|c| c.id.clone()) {
            Some(id) => match client.get_client(&id).await {
                Ok(c) => report.pass(
                    "get_client",
                    format!("fetched one, {} unmodeled field(s) in extra", c.extra.len()),
                ),
                Err(e) => report.fail("get_client", describe(&e)),
            },
            None => report.pass("get_client", "skipped: no clients on this instance"),
        },
        Err(e) => report.fail("get_client", describe(&e)),
    }
}

async fn write_checks(client: &InvoiceNinjaClient, report: &mut Report) {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let name = format!("sdk-smoke-{nanos}");

    let created = match client
        .create_client(&Client {
            name: name.clone(),
            ..Default::default()
        })
        .await
    {
        Ok(created) => created,
        Err(e) => {
            report.fail("create_client", describe(&e));
            return;
        }
    };
    let Some(id) = created.id.clone() else {
        report.fail("create_client", "response carried no id");
        return;
    };
    report.pass("create_client", format!("created `{name}`"));

    upload_checks(client, report, &id).await;
    round_trip_checks(client, report, &id, created.extra.len()).await;

    // Always runs, including after a failed check above.
    match client.delete_client(&id).await {
        Ok(()) => report.pass(
            "delete_client",
            "soft-deleted (a purge needs the account password)",
        ),
        Err(e) => report.fail(
            "delete_client",
            format!("{} -- remove `{name}` by hand", describe(&e)),
        ),
    }
}

async fn upload_checks(client: &InvoiceNinjaClient, report: &mut Report, id: &str) {
    let text = |label: &str| {
        FileUpload::new(
            format!("{label}.txt"),
            format!("smoke {label}").into_bytes(),
        )
        .mime("text/plain")
    };
    let count_documents = |c: &Client| {
        c.extra
            .get("documents")
            .and_then(|d| d.as_array())
            .map(Vec::len)
    };

    // The wire format is the least-verified part: POST + `_method=PUT`, read
    // from the framework source rather than observed.
    match client
        .upload_client(id, vec![text("first")], Some(false))
        .await
    {
        Ok(c) => report.check(
            "upload_client (1 file, private)",
            count_documents(&c) == Some(1),
            format!("documents on record: {:?}", count_documents(&c)),
        ),
        Err(e) => report.fail("upload_client (1 file, private)", describe(&e)),
    }

    match client
        .upload_client(id, vec![text("second"), text("third")], Some(true))
        .await
    {
        Ok(c) => report.check(
            "upload_client (2 files, public)",
            count_documents(&c) == Some(3),
            format!(
                "documents on record: {:?} (expected 3)",
                count_documents(&c)
            ),
        ),
        Err(e) => report.fail("upload_client (2 files, public)", describe(&e)),
    }
}

async fn round_trip_checks(
    client: &InvoiceNinjaClient,
    report: &mut Report,
    id: &str,
    extra_keys_at_create: usize,
) {
    // `get` then `update` must not drop fields the crate does not model.
    let fetched = match client.get_client(id).await {
        Ok(c) => c,
        Err(e) => {
            report.fail("get_client (round trip)", describe(&e));
            return;
        }
    };
    let before = fetched.extra.len();

    let mut edited = fetched;
    edited.private_notes = "edited by smoke test".into();
    let updated = match client.update_client(id, &edited).await {
        Ok(c) => c,
        Err(e) => {
            report.fail("update_client", describe(&e));
            return;
        }
    };

    report.check(
        "update_client preserves extra",
        updated.extra.len() >= before.min(extra_keys_at_create),
        format!(
            "extra keys: {extra_keys_at_create} at create, {before} on get, {} after update",
            updated.extra.len()
        ),
    );
    report.check(
        "update_client applied the edit",
        updated.private_notes == "edited by smoke test",
        "private_notes round-tripped",
    );
}
