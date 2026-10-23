use invoiceninjars::models::{
    Client, Credit, Expense, Invoice, ListParams, Project, Quote, Task, Vendor,
};
use serde_json::json;

#[test]
fn client_round_trip_preserves_unknown_fields() {
    let payload = json!({
        "id": "abc123",
        "name": "Acme Inc",
        "custom_value1": "unmodeled field",
    });

    let client: Client = serde_json::from_value(payload).unwrap();
    assert_eq!(client.name, "Acme Inc");
    assert_eq!(
        client.extra.get("custom_value1").unwrap(),
        "unmodeled field"
    );

    let round_tripped = serde_json::to_value(&client).unwrap();
    assert_eq!(round_tripped["custom_value1"], "unmodeled field");
}

#[test]
fn invoice_round_trip_preserves_unknown_fields() {
    let payload = json!({
        "id": "inv1",
        "client_id": "abc123",
        "number": "INV-0001",
        "custom_surcharge1": 12.5,
    });

    let invoice: Invoice = serde_json::from_value(payload).unwrap();
    assert_eq!(invoice.number, "INV-0001");
    assert_eq!(invoice.extra.get("custom_surcharge1").unwrap(), 12.5);

    let round_tripped = serde_json::to_value(&invoice).unwrap();
    assert_eq!(round_tripped["custom_surcharge1"], 12.5);
}

#[test]
fn list_params_omits_unset_fields() {
    let params = ListParams::new().per_page(25).sort("created_at|desc");

    assert_eq!(
        serde_json::to_value(&params).unwrap(),
        json!({"per_page": 25, "sort": "created_at|desc"})
    );
}

#[test]
fn vendor_round_trip_preserves_nested_contact_fields() {
    let payload = json!({
        "id": "ven1",
        "name": "Supply Co",
        "custom_value2": "unmodeled vendor field",
        "contacts": [{
            "id": "vc1",
            "first_name": "Ada",
            "email": "ada@supply.example",
            "is_primary": true,
            "custom_value1": "unmodeled contact field",
        }],
    });

    let vendor: Vendor = serde_json::from_value(payload).unwrap();
    assert_eq!(vendor.name, "Supply Co");
    assert_eq!(vendor.contacts.len(), 1);
    assert_eq!(vendor.contacts[0].first_name, "Ada");
    assert!(vendor.contacts[0].is_primary);

    let round_tripped = serde_json::to_value(&vendor).unwrap();
    assert_eq!(round_tripped["custom_value2"], "unmodeled vendor field");
    assert_eq!(
        round_tripped["contacts"][0]["custom_value1"],
        "unmodeled contact field"
    );
}

#[test]
fn expense_round_trip_preserves_unknown_fields() {
    let payload = json!({
        "id": "exp1",
        "vendor_id": "ven1",
        "amount": 42.75,
        "should_be_invoiced": true,
        "custom_value3": "unmodeled field",
    });

    let expense: Expense = serde_json::from_value(payload).unwrap();
    assert_eq!(expense.vendor_id, "ven1");
    assert_eq!(expense.amount, 42.75);
    assert!(expense.should_be_invoiced);

    let round_tripped = serde_json::to_value(&expense).unwrap();
    assert_eq!(round_tripped["custom_value3"], "unmodeled field");
}

#[test]
fn task_round_trip_preserves_unknown_fields() {
    let payload = json!({
        "id": "task1",
        "project_id": "proj1",
        "description": "Design review",
        "time_log": "[[1700000000,1700003600]]",
        "custom_value1": "unmodeled field",
    });

    let task: Task = serde_json::from_value(payload).unwrap();
    assert_eq!(task.project_id, "proj1");
    assert_eq!(task.time_log, "[[1700000000,1700003600]]");

    let round_tripped = serde_json::to_value(&task).unwrap();
    assert_eq!(round_tripped["custom_value1"], "unmodeled field");
}

#[test]
fn project_round_trip_preserves_unknown_fields() {
    let payload = json!({
        "id": "proj1",
        "client_id": "abc123",
        "name": "Website rebuild",
        "budgeted_hours": 120.0,
        "custom_value4": "unmodeled field",
    });

    let project: Project = serde_json::from_value(payload).unwrap();
    assert_eq!(project.name, "Website rebuild");
    assert_eq!(project.budgeted_hours, 120.0);

    let round_tripped = serde_json::to_value(&project).unwrap();
    assert_eq!(round_tripped["custom_value4"], "unmodeled field");
}

/// `Quote` and `Credit` are type aliases for `Invoice`; this pins that the
/// aliased endpoints decode the shared invoice shape.
#[test]
fn quote_and_credit_share_the_invoice_shape() {
    let payload = json!({
        "id": "q1",
        "number": "QUO-0001",
        "entity_type": "quote",
    });

    let quote: Quote = serde_json::from_value(payload.clone()).unwrap();
    assert_eq!(quote.number, "QUO-0001");
    assert_eq!(quote.extra.get("entity_type").unwrap(), "quote");

    let credit: Credit = serde_json::from_value(payload).unwrap();
    assert_eq!(credit.number, "QUO-0001");
}
