//! Decodes fixtures captured from a real server.
//!
//! `tests/fixtures/*.json` are written by `examples/capture_shapes.rs` and
//! hold only key names and JSON types (strings are `""`, numbers `0`, bools
//! `false`). Mock responses written from the upstream source have been wrong
//! about the real thing before, so these pin the shapes a server actually
//! sent. To refresh them, seed a disposable instance with
//! `examples/seed_sample_data.rs` and re-run the capture.
//!
//! Three properties are checked: every fixture decodes into its model
//! (goal 2), a decode followed by a re-serialise loses no key (goal 1), and no
//! fixture contains a real value, so a careless capture cannot leak data.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use invoiceninjars::models::*;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn load(name: &str) -> Value {
    let path = dir().join(format!("{name}.json"));
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The captured row of a list fixture.
fn row(name: &str) -> Value {
    load(name)["data"][0].clone()
}

/// Key paths present in `raw` but absent from `out`. Arrays are compared
/// through their first element, which is all a fixture keeps.
fn lost_keys(raw: &Value, out: &Value, prefix: &str, into: &mut Vec<String>) {
    match (raw, out) {
        (Value::Object(raw), Value::Object(out)) => {
            for (key, raw_value) in raw {
                match out.get(key) {
                    Some(out_value) => {
                        lost_keys(raw_value, out_value, &format!("{prefix}.{key}"), into)
                    }
                    None => into.push(format!("{prefix}.{key}")),
                }
            }
        }
        (Value::Array(raw), Value::Array(out)) => {
            if let (Some(raw), Some(out)) = (raw.first(), out.first()) {
                lost_keys(raw, out, &format!("{prefix}[]"), into);
            }
        }
        _ => {}
    }
}

/// Decodes `raw` as `T`, re-serialises it, and returns problems.
fn check<T: DeserializeOwned + Serialize>(name: &str, raw: &Value) -> Vec<String> {
    let typed: T = match serde_json::from_value(raw.clone()) {
        Ok(typed) => typed,
        Err(e) => return vec![format!("{name}: does not decode: {e}")],
    };
    let out = serde_json::to_value(&typed).expect("serialises");
    let mut lost = Vec::new();
    lost_keys(raw, &out, name, &mut lost);
    lost.into_iter()
        .filter(|path| !OMITTED_WHEN_EMPTY.contains(&path.as_str()))
        .map(|path| format!("{path}: key lost on a decode/re-serialise"))
        .collect()
}

/// Fields that are deliberately left out of a request when empty, because the
/// server rejects the empty value (`""` becomes `NULL` on a non-null column,
/// or fails an `integer` rule). A fixture holds `""` placeholders, so these
/// show up as absent after a re-serialise. Adding to this list is a decision.
const OMITTED_WHEN_EMPTY: &[&str] = &[
    "locations.country_id",
    "purchase_orders.status_id",
    "webhooks.format",
];

/// One fixture per `(file, type)`; the macro also records which files it
/// covered so a new capture cannot sit untested.
macro_rules! list_fixtures {
    ($($file:literal => $ty:ty),+ $(,)?) => {
        const COVERED_LISTS: &[&str] = &[$($file),+];

        fn check_lists() -> Vec<String> {
            let mut problems = Vec::new();
            $( problems.extend(check::<$ty>($file, &row($file))); )+
            problems
        }
    };
}

list_fixtures! {
    "activities" => Activity,
    "clients" => Client,
    "companies" => Company,
    "credits" => Credit,
    "designs" => Design,
    "documents" => Document,
    "expense_categories" => ExpenseCategory,
    "expenses" => Expense,
    "group_settings" => GroupSetting,
    "invoices" => Invoice,
    "locations" => Location,
    "payment_terms" => PaymentTerm,
    "payments" => Payment,
    "products" => Product,
    "projects" => Project,
    "purchase_orders" => PurchaseOrder,
    "quotes" => Quote,
    "recurring_expenses" => RecurringExpense,
    "recurring_invoices" => RecurringInvoice,
    "system_logs" => SystemLog,
    "tags" => Tag,
    "task_statuses" => TaskStatus,
    "tasks" => Task,
    "tax_rates" => TaxRate,
    "tokens" => CompanyToken,
    "users" => User,
    "vendors" => Vendor,
    "webhooks" => Webhook,
}

/// Fixtures that are not a one-row list.
const COVERED_OTHER: &[&str] = &[
    "statics",
    "health_check",
    "subscription_steps",
    "activity_feed",
];

#[test]
fn every_real_row_decodes_and_loses_no_key() {
    let problems = check_lists();
    assert!(
        problems.is_empty(),
        "{} problem(s):\n{}",
        problems.len(),
        problems.join("\n")
    );
}

#[test]
fn bare_endpoint_fixtures_decode() {
    let mut problems = Vec::new();
    problems.extend(check::<Statics>("statics", &load("statics")));
    problems.extend(check::<HealthCheck>("health_check", &load("health_check")));
    problems.extend(check::<ActivityFeedEntry>(
        "activity_feed",
        &load("activity_feed")["data"][0],
    ));

    // Keyed by class name on a real server; see `subscription_steps`.
    let steps: Result<BTreeMap<String, SubscriptionStep>, _> =
        serde_json::from_value(load("subscription_steps"));
    match steps {
        Ok(steps) => assert!(!steps.is_empty(), "the captured steps object is empty"),
        Err(e) => problems.push(format!("subscription_steps: {e}")),
    }

    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn every_fixture_file_is_covered_by_a_test() {
    let mut on_disk: Vec<String> = fs::read_dir(dir())
        .expect("tests/fixtures exists")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()? == "json").then(|| path.file_stem()?.to_str().map(String::from))?
        })
        .collect();
    on_disk.sort();

    let mut covered: Vec<String> = COVERED_LISTS
        .iter()
        .chain(COVERED_OTHER)
        .map(|s| s.to_string())
        .collect();
    covered.sort();

    assert_eq!(
        on_disk, covered,
        "a captured fixture has no test (or a test has no fixture)"
    );
}

/// The capture tool replaces every value; this refuses to pass if any
/// survived, so real data cannot be committed by accident.
#[test]
fn fixtures_hold_shapes_and_never_values() {
    fn walk(value: &Value, at: &str, bad: &mut Vec<String>) {
        match value {
            Value::String(s) if !s.is_empty() => bad.push(format!("{at}: string value")),
            Value::Number(n) if n.as_f64() != Some(0.0) => bad.push(format!("{at}: number value")),
            Value::Bool(true) => bad.push(format!("{at}: true")),
            Value::Array(items) => {
                assert!(items.len() <= 1, "{at}: a fixture keeps one array element");
                items.iter().for_each(|v| walk(v, &format!("{at}[]"), bad));
            }
            Value::Object(map) => map
                .iter()
                .for_each(|(k, v)| walk(v, &format!("{at}.{k}"), bad)),
            _ => {}
        }
    }

    let mut bad = Vec::new();
    for entry in fs::read_dir(dir()).expect("tests/fixtures exists") {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "json") {
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            walk(&load(&name), &name, &mut bad);
        }
    }
    assert!(
        bad.is_empty(),
        "fixtures contain values:\n{}",
        bad.join("\n")
    );
}
