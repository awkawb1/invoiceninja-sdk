//! Records the *shape* of real Invoice Ninja responses as test fixtures.
//!
//! Mock servers written from the upstream source have been wrong about the
//! real thing before (system-log timestamps, `/subscriptions/steps`), so the
//! offline suite benefits from fixtures that came from a real server. This
//! writes them without writing any data: every string becomes `""`, every
//! number `0`, every bool `false`, and an array keeps only its first element.
//! What survives is the key names and the JSON types, which is what decoding
//! depends on. `tests/real_shapes.rs` refuses to pass if a fixture holds
//! anything else.
//!
//! GET requests only, so it is safe against any instance, but it still reads
//! real records in memory. Review the diff before committing.
//!
//! ```sh
//! IN_BASE_URL=http://localhost:8080/api/v1 IN_API_TOKEN=... \
//!     cargo run --example capture_shapes
//! ```
//!
//! A resource with no rows on the instance writes nothing and is reported as
//! skipped, since an empty list says nothing about a row's shape. Populate a
//! disposable instance and re-run to capture more.

use std::fs;
use std::path::Path;

use serde_json::{json, Value};

/// Resources captured as a one-row list. The name doubles as the fixture
/// filename.
const LISTS: &[&str] = &[
    "activities",
    "bank_integrations",
    "bank_transaction_rules",
    "bank_transactions",
    "client_gateway_tokens",
    "clients",
    "companies",
    "company_gateways",
    "credits",
    "designs",
    "documents",
    "expense_categories",
    "expenses",
    "group_settings",
    "invoices",
    "locations",
    "payment_terms",
    "payments",
    "products",
    "projects",
    "purchase_orders",
    "quotes",
    "recurring_expenses",
    "recurring_invoices",
    "recurring_quotes",
    "subscriptions",
    "system_logs",
    "tags",
    "task_schedulers",
    "task_statuses",
    "tasks",
    "tax_rates",
    "tokens",
    "users",
    "vendors",
    "webhooks",
];

/// Endpoints with no `data` envelope, captured whole.
const BARE: &[(&str, &str)] = &[
    ("statics", "statics"),
    ("health_check", "health_check"),
    ("subscription_steps", "subscriptions/steps"),
    ("activity_feed", "activities?rows=1&reactv2=1"),
];

/// Replaces every value with a placeholder of the same JSON type.
fn shape(value: &Value) -> Value {
    match value {
        Value::Null => Value::Null,
        Value::Bool(_) => json!(false),
        Value::Number(n) if n.is_f64() => json!(0.0),
        Value::Number(_) => json!(0),
        Value::String(_) => json!(""),
        // Only the first element: a list's rows share a shape, and keeping
        // all of them would just be size.
        Value::Array(items) => Value::Array(items.first().map(shape).into_iter().collect()),
        Value::Object(map) => {
            Value::Object(map.iter().map(|(k, v)| (k.clone(), shape(v))).collect())
        }
    }
}

#[tokio::main]
async fn main() {
    let base = std::env::var("IN_BASE_URL").expect("set IN_BASE_URL");
    let token = std::env::var("IN_API_TOKEN").expect("set IN_API_TOKEN");
    let base = base.trim_end_matches('/');
    let out = Path::new("tests/fixtures");
    fs::create_dir_all(out).expect("create tests/fixtures");

    let http = reqwest::Client::new();
    let get = |path: String| {
        let http = http.clone();
        let token = token.clone();
        let url = format!("{base}/{path}");
        async move {
            let response = http
                .get(url)
                .header("X-Api-Token", token)
                .header("Accept", "application/json")
                .header("X-Requested-With", "XMLHttpRequest")
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let status = response.status();
            let body = response.text().await.map_err(|e| e.to_string())?;
            if !status.is_success() {
                return Err(format!("HTTP {status}"));
            }
            serde_json::from_str::<Value>(&body).map_err(|e| e.to_string())
        }
    };

    for name in LISTS {
        // Deleted and archived rows included: on a disposable instance they
        // are often all there is.
        // `system_logs` answers the status filter with a 500, so fall back to
        // the plain list when the filtered one fails.
        let filtered = format!("{name}?per_page=1&status=active,archived,deleted");
        let result = match get(filtered).await {
            Ok(body) => Ok(body),
            Err(_) => get(format!("{name}?per_page=1")).await,
        };
        match result {
            Ok(body) => match body
                .get("data")
                .and_then(Value::as_array)
                .and_then(|d| d.first())
            {
                Some(row) => {
                    write(out, name, &json!({ "data": [shape(row)] }));
                    println!("  captured  {name}");
                }
                None => println!("  skipped   {name}: no rows on this instance"),
            },
            Err(e) => println!("  FAILED    {name}: {e}"),
        }
    }

    for (name, path) in BARE {
        match get((*path).to_string()).await {
            Ok(body) => {
                write(out, name, &shape(&body));
                println!("  captured  {name}");
            }
            Err(e) => println!("  FAILED    {name}: {e}"),
        }
    }
}

fn write(dir: &Path, name: &str, value: &Value) {
    let mut text = serde_json::to_string_pretty(value).expect("serialise fixture");
    text.push('\n');
    fs::write(dir.join(format!("{name}.json")), text).expect("write fixture");
}
