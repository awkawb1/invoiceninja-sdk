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
//! The read tier also fetches the same rows raw and fails if a typed row lost
//! a key. The write tier creates a client, attaches files to it, creates a
//! purchase order, location, webhook, and recurring invoice from default
//! models, and deletes what it made.
//!
//! With `IN_API_PASSWORD` also set, a password tier runs inside the write
//! tier: it merges two throwaway clients and purges the survivor, which
//! really removes the row. Take the password from the environment, never from
//! a command line or chat.
//!
//! **Do not point the write tier at a production server.** Even after the delete that
//! leaves a gap in the client number sequence and a soft-deleted row (a real
//! purge needs the account password), and it may fire webhooks.
//!
//! `IN_BASE_URL` has no default, unlike the other example, because defaulting
//! to the hosted app is the wrong failure for a tool that can write.

use std::fmt::Display;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use invoiceninjars::models::{
    Client, ListParams, Location, PurchaseOrder, RecurringInvoice, Vendor, Webhook,
};
use invoiceninjars::{Error, FileUpload, InvoiceNinjaClient};
use serde::Serialize;
use serde_json::{json, Value};

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

/// Plain GETs for the raw JSON, so a typed result can be compared with what
/// the server actually sent. Decoding successfully proves little: a model
/// that quietly dropped a field would still decode.
struct Raw {
    http: reqwest::Client,
    base: String,
    token: String,
}

impl Raw {
    async fn rows(&self, path: &str) -> Option<Vec<Value>> {
        let body: Value = self
            .http
            .get(format!("{}/{path}?per_page=2", self.base))
            .header("X-Api-Token", &self.token)
            .header("Accept", "application/json")
            .header("X-Requested-With", "XMLHttpRequest")
            .send()
            .await
            .ok()?
            .json()
            .await
            .ok()?;
        body.get("data")?.as_array().cloned()
    }
}

/// Keys that are left out of a request when empty on purpose (the server
/// rejects the empty value), so an empty one is not a lost key.
const OMITTED_WHEN_EMPTY: &[&str] = &["country_id", "status_id", "format"];

/// Key paths in `raw` that `typed` no longer has.
fn lost_keys(raw: &Value, typed: &Value, prefix: &str, into: &mut Vec<String>) {
    match (raw, typed) {
        (Value::Object(raw), Value::Object(typed)) => {
            for (key, raw_value) in raw {
                match typed.get(key) {
                    Some(typed_value) => {
                        lost_keys(raw_value, typed_value, &format!("{prefix}.{key}"), into)
                    }
                    None if raw_value == "" && OMITTED_WHEN_EMPTY.contains(&key.as_str()) => {}
                    None => into.push(format!("{prefix}.{key}")),
                }
            }
        }
        (Value::Array(raw), Value::Array(typed)) => {
            if let (Some(raw), Some(typed)) = (raw.first(), typed.first()) {
                lost_keys(raw, typed, &format!("{prefix}[]"), into);
            }
        }
        _ => {}
    }
}

/// Fetches the same rows raw and requires the typed ones to keep every key.
/// Only key names are printed, never values.
async fn key_loss(report: &mut Report, raw: &Raw, path: &str, typed: &[impl Serialize]) {
    let Some(raw_rows) = raw.rows(path).await else {
        report.fail(
            &format!("keys kept: {path}"),
            "could not fetch the raw rows",
        );
        return;
    };
    let mut lost = Vec::new();
    for (raw_row, typed_row) in raw_rows.iter().zip(typed) {
        let typed_row = serde_json::to_value(typed_row).unwrap_or(Value::Null);
        lost_keys(raw_row, &typed_row, path, &mut lost);
    }
    lost.sort();
    lost.dedup();
    report.check(
        &format!("keys kept: {path}"),
        lost.is_empty(),
        if lost.is_empty() {
            format!("{} row(s), no key lost", raw_rows.len().min(typed.len()))
        } else {
            format!("lost {}", lost.join(", "))
        },
    );
}

#[tokio::main]
async fn main() -> ExitCode {
    let base_url = std::env::var("IN_BASE_URL").expect("set IN_BASE_URL (no default, on purpose)");
    let token = std::env::var("IN_API_TOKEN").expect("set IN_API_TOKEN");

    let host = host_of(&base_url);
    let raw = Raw {
        http: reqwest::Client::new(),
        base: base_url.trim_end_matches('/').to_string(),
        token: token.clone(),
    };
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
    read_checks(&client, &raw, &mut report).await;

    // Empty counts as unset: CI passes the variable through even when blank.
    match std::env::var("IN_SMOKE_WRITE_HOST")
        .ok()
        .filter(|v| !v.is_empty())
    {
        Some(allowed) if allowed == host => {
            println!("\nwrite checks (IN_SMOKE_WRITE_HOST matches {host})");
            write_checks(&client, &mut report).await;

            match std::env::var("IN_API_PASSWORD")
                .ok()
                .filter(|v| !v.is_empty())
            {
                Some(password) => {
                    println!("\npassword checks (IN_API_PASSWORD is set)");
                    password_checks(&client, &mut report, password).await;
                }
                None => println!("\npassword checks skipped (set IN_API_PASSWORD to enable)"),
            }
        }
        Some(allowed) => {
            println!(
                "\nwrite checks SKIPPED: IN_SMOKE_WRITE_HOST is `{allowed}` but the target is `{host}`"
            );
        }
        None => println!("\nwrite checks skipped (set IN_SMOKE_WRITE_HOST to enable)"),
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

async fn read_checks(client: &InvoiceNinjaClient, raw: &Raw, report: &mut Report) {
    let small = ListParams::new().per_page(2);

    // Decodes one page, then (if there are rows) checks none of their keys
    // were lost. The REST path is the method name without `list_`.
    macro_rules! list {
        ($report:expr, $name:literal, $call:expr) => {
            match $call.await {
                Ok(rows) => {
                    $report.pass($name, format!("{} row(s) decoded", rows.len()));
                    if !rows.is_empty() {
                        key_loss($report, raw, $name.trim_start_matches("list_"), &rows).await;
                    }
                }
                Err(e) => $report.fail($name, describe(&e)),
            }
        };
    }

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

    match client.subscription_steps().await {
        Ok(steps) => report.check(
            "subscription_steps",
            !steps.is_empty() && steps.iter().all(|s| !s.id.is_empty()),
            format!("{} step(s) decoded", steps.len()),
        ),
        Err(e) => report.fail("subscription_steps", describe(&e)),
    }

    match client.activity_feed(Some(5)).await {
        Ok(feed) => report.check(
            "activity_feed",
            feed.iter().all(|e| e.activity_type_id > 0),
            format!("{} entr(ies), numeric type ids", feed.len()),
        ),
        Err(e) => report.fail("activity_feed", describe(&e)),
    }

    // A POST, but it only renders: nothing is stored. Needs an invoice to
    // render, so it is skipped on an empty instance.
    match client.list_invoices(&small).await {
        Ok(rows) => match rows
            .first()
            .and_then(|i| i.id.clone().map(|id| (id, i.client_id.clone())))
        {
            Some((id, client_id)) => {
                let body = json!({ "entity": "invoice", "entity_id": id, "client_id": client_id });
                match client.live_preview(&body).await {
                    Ok(pdf) => report.check(
                        "live_preview",
                        pdf.starts_with(b"%PDF"),
                        format!("{} byte(s), starts with the PDF magic", pdf.len()),
                    ),
                    Err(e) => report.fail("live_preview", describe(&e)),
                }
            }
            None => report.pass("live_preview", "skipped: no invoices on this instance"),
        },
        Err(e) => report.fail("live_preview", describe(&e)),
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
    list!(report, "list_activities", client.list_activities(&small));

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
    extras_checks(client, report, &id).await;
    creation_checks(client, report, &id).await;
    activity_checks(client, report, &id).await;

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

    // POST + `_method=PUT`: read from the framework source, then observed
    // working on 5.13.50.
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

/// Per-client extras that only read or reshape data already on the record.
/// `updateTaxData` and `check_vat` are left out: they call external services.
async fn extras_checks(client: &InvoiceNinjaClient, report: &mut Report, id: &str) {
    match client.client_show_settings(id).await {
        Ok(settings) => report.check(
            "client_show_settings",
            settings.is_object(),
            format!("{} setting(s)", settings.as_object().map_or(0, |m| m.len())),
        ),
        Err(e) => report.fail("client_show_settings", describe(&e)),
    }

    match client.client_vat_status(id).await {
        Ok(status) => report.pass(
            "client_vat_status",
            format!("message of {} char(s)", status.message.len()),
        ),
        Err(e) => report.fail("client_vat_status", describe(&e)),
    }

    // The two uploads in `upload_checks` should be visible here.
    match client.client_documents(id, &json!({})).await {
        Ok(docs) => report.check(
            "client_documents",
            docs.len() == 3,
            format!("{} document(s) (expected 3)", docs.len()),
        ),
        Err(e) => report.fail("client_documents", describe(&e)),
    }
}

/// Creates from bare `Default` models the records that broke against a real
/// server: an empty `status_id`, `country_id` or `format` was rejected, and a
/// string `frequency_id` failed to decode after the record existed. Each is
/// deleted again straight away.
async fn creation_checks(client: &InvoiceNinjaClient, report: &mut Report, client_id: &str) {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    let vendor = match client
        .create_vendor(&Vendor {
            name: format!("sdk-smoke-vendor-{stamp}"),
            ..Default::default()
        })
        .await
    {
        Ok(vendor) => vendor,
        Err(e) => {
            report.fail("create_vendor", describe(&e));
            return;
        }
    };
    let vendor_id = vendor.id.clone().unwrap_or_default();

    match client
        .create_purchase_order(&PurchaseOrder {
            vendor_id: vendor_id.clone(),
            ..Default::default()
        })
        .await
    {
        Ok(po) => {
            report.pass("create_purchase_order (default)", "empty status_id omitted");
            if let Some(id) = po.id {
                let _ = client.delete_purchase_order(&id).await;
            }
        }
        Err(e) => report.fail("create_purchase_order (default)", describe(&e)),
    }

    match client
        .create_location(&Location {
            name: format!("sdk-smoke-location-{stamp}"),
            client_id: client_id.to_string(),
            ..Default::default()
        })
        .await
    {
        Ok(location) => {
            report.pass("create_location (default)", "empty country_id omitted");
            if let Some(id) = location.id {
                let _ = client.delete_location(&id).await;
            }
        }
        Err(e) => report.fail("create_location (default)", describe(&e)),
    }

    match client
        .create_webhook(&Webhook {
            event_id: "1".into(),
            target_url: "https://example.com/sdk-smoke".into(),
            ..Default::default()
        })
        .await
    {
        Ok(hook) => {
            report.pass("create_webhook (default)", "empty format omitted");
            if let Some(id) = hook.id {
                let _ = client.delete_webhook(&id).await;
            }
        }
        Err(e) => report.fail("create_webhook (default)", describe(&e)),
    }

    // The create succeeds either way; decoding the reply is what failed.
    match client
        .create_recurring_invoice(&RecurringInvoice {
            client_id: client_id.to_string(),
            frequency_id: 5,
            ..Default::default()
        })
        .await
    {
        Ok(recurring) => {
            report.check(
                "create_recurring_invoice",
                recurring.frequency_id == 5,
                format!("frequency_id decoded as {}", recurring.frequency_id),
            );
            if let Some(id) = recurring.id {
                let _ = client.delete_recurring_invoice(&id).await;
            }
        }
        Err(e) => report.fail("create_recurring_invoice", describe(&e)),
    }

    if let Err(e) = client.delete_vendor(&vendor_id).await {
        report.fail(
            "delete_vendor",
            format!("{} -- remove the vendor by hand", describe(&e)),
        );
    }
}

/// Password tier: needs `IN_API_PASSWORD`, and only runs inside the write
/// tier's host guard. Merges two throwaway clients, then purges the survivor,
/// which unlike `delete_client` really removes the row. Users, companies and
/// vendors are left alone: inviting a user sends mail, and a company purge
/// takes data that was not created here.
async fn password_checks(client: &InvoiceNinjaClient, report: &mut Report, password: String) {
    let client = match client.clone().with_api_password(password) {
        Ok(client) => client,
        Err(e) => {
            report.fail("with_api_password", describe(&e));
            return;
        }
    };

    match client.new_user_template().await {
        Ok(user) => report.check(
            "new_user_template",
            user.id.as_deref().unwrap_or("").is_empty(),
            "blank user, nothing stored",
        ),
        Err(e) => report.fail("new_user_template", describe(&e)),
    }

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut ids = Vec::new();
    for label in ["keep", "drop"] {
        match client
            .create_client(&Client {
                name: format!("sdk-smoke-merge-{label}-{stamp}"),
                ..Default::default()
            })
            .await
        {
            Ok(created) => ids.extend(created.id),
            Err(e) => {
                report.fail("create_client (merge)", describe(&e));
                return;
            }
        }
    }
    let [keep, drop] = [ids[0].clone(), ids[1].clone()];

    match client.merge_clients(&keep, &drop).await {
        Ok(merged) => report.check(
            "merge_clients",
            merged.id.as_deref() == Some(keep.as_str()),
            "the first id is the survivor",
        ),
        Err(e) => report.fail("merge_clients", describe(&e)),
    }

    match client.purge_client(&keep).await {
        Ok(message) => report.pass("purge_client", format!("`{}`", message.message)),
        Err(e) => report.fail(
            "purge_client",
            format!(
                "{} -- remove the sdk-smoke-merge clients by hand",
                describe(&e)
            ),
        ),
    }

    // A purge is permanent, so the record must now be gone, not just flagged.
    match client.get_client(&keep).await {
        Err(Error::Api { status, .. }) if status.as_u16() == 404 => {
            report.pass("get_client after purge", "404, the row is gone")
        }
        Err(e) => report.fail("get_client after purge", describe(&e)),
        Ok(_) => report.fail("get_client after purge", "still readable after a purge"),
    }
}

/// Notes, the per-record feed and email history, against the throwaway
/// client. Nothing here sends mail: the history is read, and is expected to
/// be empty because no mail has gone out for this client.
async fn activity_checks(client: &InvoiceNinjaClient, report: &mut Report, id: &str) {
    let note = "sdk smoke note";
    match client.add_activity_note("clients", id, note).await {
        Ok(activity) => report.check(
            "add_activity_note",
            activity.notes == note && activity.client_id == id,
            "note stored against the client",
        ),
        Err(e) => report.fail("add_activity_note", describe(&e)),
    }

    match client.entity_activity("client", id, Some(10)).await {
        Ok(feed) => report.check(
            "entity_activity",
            feed.iter().any(|e| e.notes == note),
            format!("{} entr(ies), including the note", feed.len()),
        ),
        Err(e) => report.fail("entity_activity", describe(&e)),
    }

    match client.client_email_history(id).await {
        Ok(history) => report.check(
            "client_email_history",
            history.is_empty(),
            "no mail sent, so no history",
        ),
        Err(e) => report.fail("client_email_history", describe(&e)),
    }
}
