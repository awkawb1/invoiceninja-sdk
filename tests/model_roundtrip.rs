use invoiceninja_sdk::models::{
    BankIntegration, BankTransaction, BankTransactionRule, BulkRequest, Client, ClientGatewayToken,
    Company, CompanyGateway, CompanyToken, Credit, Design, Document, Expense, ExpenseCategory,
    GroupSetting, HealthCheck, Invoice, ListParams, ListResponse, Location, LoginResponse, Passkey,
    PaymentTerm, Project, PurchaseOrder, Quote, RecurringExpense, RecurringInvoice, RecurringQuote,
    SearchResults, Statics, Subscription, SystemLog, Tag, Task, TaskScheduler, TaskStatus, TaxRate,
    User, Vendor, Webhook,
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

#[test]
fn recurring_invoice_decodes_recurrence_fields() {
    // frequency_id / remaining_cycles come back as JSON numbers, and
    // remaining_cycles is -1 for an indefinite schedule.
    let payload = json!({
        "id": "ri1",
        "client_id": "abc123",
        "frequency_id": 5,
        "remaining_cycles": -1,
        "auto_bill": "off",
        "next_send_date": "2026-11-01",
        "line_items": [{"product_key": "widget", "cost": 9.5, "quantity": 2.0}],
        "custom_value1": "unmodeled field",
    });

    let ri: RecurringInvoice = serde_json::from_value(payload).unwrap();
    assert_eq!(ri.frequency_id, 5);
    assert_eq!(ri.remaining_cycles, -1);
    assert_eq!(ri.next_send_date, "2026-11-01");
    assert_eq!(ri.line_items.len(), 1);
    assert_eq!(ri.line_items[0].product_key, "widget");

    let round_tripped = serde_json::to_value(&ri).unwrap();
    assert_eq!(round_tripped["custom_value1"], "unmodeled field");
}

/// `RecurringQuote` aliases `RecurringInvoice`; the API's RecurringQuote
/// fields are a strict subset, so a quote payload must decode without the
/// invoice-only fields present.
#[test]
fn recurring_quote_decodes_without_invoice_only_fields() {
    let payload = json!({
        "id": "rq1",
        "client_id": "abc123",
        "frequency_id": 2,
        "remaining_cycles": 12,
    });

    let rq: RecurringQuote = serde_json::from_value(payload).unwrap();
    assert_eq!(rq.frequency_id, 2);
    assert_eq!(rq.remaining_cycles, 12);
    // Invoice-only fields fall back to their defaults rather than failing.
    assert_eq!(rq.auto_bill, "");
    assert!(!rq.auto_bill_enabled);
}

#[test]
fn purchase_order_round_trip_preserves_unknown_fields() {
    let payload = json!({
        "id": "po1",
        "vendor_id": "ven1",
        "number": "PO-0001",
        "amount": 250.0,
        "custom_surcharge1": 7.25,
    });

    let po: PurchaseOrder = serde_json::from_value(payload).unwrap();
    assert_eq!(po.vendor_id, "ven1");
    assert_eq!(po.number, "PO-0001");
    assert_eq!(po.amount, 250.0);

    let round_tripped = serde_json::to_value(&po).unwrap();
    assert_eq!(round_tripped["custom_surcharge1"], 7.25);
}

#[test]
fn recurring_expense_round_trip_preserves_unknown_fields() {
    let payload = json!({
        "id": "rex1",
        "vendor_id": "ven1",
        "frequency_id": 4,
        "remaining_cycles": 6,
        "amount": 19.99,
        "custom_value2": "unmodeled field",
    });

    let rex: RecurringExpense = serde_json::from_value(payload).unwrap();
    assert_eq!(rex.frequency_id, 4);
    assert_eq!(rex.remaining_cycles, 6);
    assert_eq!(rex.amount, 19.99);

    let round_tripped = serde_json::to_value(&rex).unwrap();
    assert_eq!(round_tripped["custom_value2"], "unmodeled field");
}

/// `Subscription.frequency_id` is a string even though the recurring-invoice
/// field of the same name is numeric; this pins that asymmetry.
#[test]
fn subscription_frequency_id_is_a_string() {
    let payload = json!({
        "id": "sub1",
        "product_id": "prod1",
        "frequency_id": "5",
        "trial_enabled": true,
        "trial_duration": 14,
        "promo_discount": 10.0,
    });

    let sub: Subscription = serde_json::from_value(payload).unwrap();
    assert_eq!(sub.frequency_id, "5");
    assert!(sub.trial_enabled);
    assert_eq!(sub.trial_duration, 14);
    assert_eq!(sub.promo_discount, 10.0);
}

#[test]
fn small_lookup_resources_decode() {
    let tax_rate: TaxRate = serde_json::from_value(json!({
        "id": "tr1", "name": "GST", "rate": 10.0
    }))
    .unwrap();
    assert_eq!(tax_rate.name, "GST");
    assert_eq!(tax_rate.rate, 10.0);

    let category: ExpenseCategory = serde_json::from_value(json!({
        "id": "ec1", "name": "Accounting"
    }))
    .unwrap();
    assert_eq!(category.name, "Accounting");

    let status: TaskStatus = serde_json::from_value(json!({
        "id": "ts1", "name": "Backlog", "color": "#000000", "task_status_order": 4
    }))
    .unwrap();
    assert_eq!(status.task_status_order, 4);
    assert_eq!(status.color, "#000000");

    let term: PaymentTerm = serde_json::from_value(json!({
        "id": "pt1", "name": "NET 30", "num_days": 30
    }))
    .unwrap();
    assert_eq!(term.num_days, 30);

    let webhook: Webhook = serde_json::from_value(json!({
        "id": "wh1", "event_id": "1", "target_url": "https://example.com/hook", "format": "JSON"
    }))
    .unwrap();
    assert_eq!(webhook.target_url, "https://example.com/hook");
    assert_eq!(webhook.format, "JSON");
}

/// Bank transactions are the one resource whose ids come back as integers
/// rather than hashed strings.
#[test]
fn bank_transaction_decodes_integer_ids() {
    let payload = json!({
        "id": "bt1",
        "transaction_id": 9876543210i64,
        "bank_account_id": 42,
        "category_id": 7,
        "amount": 123.45,
        "base_type": "CREDIT",
        "date": "2026-10-01",
    });

    let bt: BankTransaction = serde_json::from_value(payload).unwrap();
    assert_eq!(bt.transaction_id, 9876543210);
    assert_eq!(bt.bank_account_id, 42);
    assert_eq!(bt.category_id, 7);
    assert_eq!(bt.base_type, "CREDIT");
}

/// A newly constructed model must not serialize an `id`, so `create_*` calls
/// do not send `"id": null` to the API.
#[test]
fn default_models_omit_id_on_serialize() {
    let body = serde_json::to_value(RecurringInvoice::default()).unwrap();
    assert!(body.get("id").is_none());

    let body = serde_json::to_value(TaxRate::default()).unwrap();
    assert!(body.get("id").is_none());
}

// --- explicit-null tolerance -------------------------------------------
//
// Invoice Ninja returns `null` for plenty of nominally-string fields.
// `#[serde(default)]` only covers a *missing* key, so these payloads used to
// fail to decode entirely; the models pair `default` with a
// null-to-default deserializer.

#[test]
fn null_strings_numbers_and_bools_decode_as_defaults() {
    let payload = json!({
        "id": "abc123",
        "name": null,
        "address1": null,
        "balance": null,
        "is_deleted": null,
        "contacts": null,
    });

    let client: Client = serde_json::from_value(payload).unwrap();
    assert_eq!(client.name, "");
    assert_eq!(client.address1, "");
    assert_eq!(client.balance, 0.0);
    assert!(!client.is_deleted);
    assert!(client.contacts.is_empty());
}

#[test]
fn null_line_items_and_nested_nulls_decode() {
    let payload = json!({
        "id": "inv1",
        "number": null,
        "public_notes": null,
        "line_items": [{
            "product_key": null,
            "notes": null,
            "cost": null,
            "quantity": 3.0,
        }],
    });

    let invoice: Invoice = serde_json::from_value(payload).unwrap();
    assert_eq!(invoice.number, "");
    assert_eq!(invoice.public_notes, "");
    assert_eq!(invoice.line_items.len(), 1);
    assert_eq!(invoice.line_items[0].product_key, "");
    assert_eq!(invoice.line_items[0].cost, 0.0);
    assert_eq!(invoice.line_items[0].quantity, 3.0);
}

#[test]
fn null_integer_fields_decode_across_resources() {
    let ri: RecurringInvoice = serde_json::from_value(json!({
        "id": "ri1", "frequency_id": null, "remaining_cycles": null, "auto_bill": null
    }))
    .unwrap();
    assert_eq!(ri.frequency_id, 0);
    assert_eq!(ri.remaining_cycles, 0);
    assert_eq!(ri.auto_bill, "");

    let status: TaskStatus = serde_json::from_value(json!({
        "id": "ts1", "name": null, "task_status_order": null
    }))
    .unwrap();
    assert_eq!(status.task_status_order, 0);

    let bt: BankTransaction = serde_json::from_value(json!({
        "id": "bt1", "transaction_id": null, "bank_account_id": null, "base_type": null
    }))
    .unwrap();
    assert_eq!(bt.transaction_id, 0);
    assert_eq!(bt.bank_account_id, 0);
}

/// Nullable timestamps stay `None` rather than collapsing to `Some(0)`, so
/// "never archived" is still distinguishable from "archived at epoch".
#[test]
fn null_timestamps_remain_none() {
    let client: Client = serde_json::from_value(json!({
        "id": "abc123", "archived_at": null, "created_at": null, "updated_at": 1700000000i64
    }))
    .unwrap();
    assert_eq!(client.archived_at, None);
    assert_eq!(client.created_at, None);
    assert_eq!(client.updated_at, Some(1700000000));
}

/// A null value for an *unmodeled* key still round-trips through `extra`.
#[test]
fn null_unknown_fields_round_trip_through_extra() {
    let payload = json!({"id": "abc123", "name": "Acme Inc", "custom_value1": null});

    let client: Client = serde_json::from_value(payload).unwrap();
    assert!(client.extra.get("custom_value1").unwrap().is_null());

    let round_tripped = serde_json::to_value(&client).unwrap();
    assert!(round_tripped["custom_value1"].is_null());
}

// --- non-CRUD endpoint plumbing ----------------------------------------

#[test]
fn bulk_request_serializes_action_and_ids() {
    let body = BulkRequest::new("archive", &["abc123", "def456"]);

    assert_eq!(
        serde_json::to_value(&body).unwrap(),
        json!({"action": "archive", "ids": ["abc123", "def456"]})
    );
}

/// `BulkRequest::new` takes anything string-like, so ids pulled off models
/// (which are `Option<String>`) don't need converting at the call site.
#[test]
fn bulk_request_accepts_owned_and_borrowed_ids() {
    let owned: Vec<String> = vec!["abc123".to_string(), "def456".to_string()];
    let from_owned = BulkRequest::new("delete", &owned);
    let from_borrowed = BulkRequest::new("delete", &["abc123", "def456"]);

    assert_eq!(from_owned.ids, from_borrowed.ids);
    assert_eq!(from_owned.action, "delete");
}

/// A bulk action whose `ids` match nothing answers HTTP 200 with
/// `{"message": "No Invoices Found"}` — no `data` key at all. That has to
/// read as an empty result, not a decode error.
#[test]
fn list_envelope_without_data_key_decodes_as_empty() {
    let body: ListResponse<Invoice> =
        serde_json::from_value(json!({"message": "No Invoices Found"})).unwrap();

    assert!(body.data.is_empty());
    assert!(body.meta.is_none());
}

#[test]
fn list_envelope_still_decodes_data_and_pagination() {
    let body: ListResponse<Invoice> = serde_json::from_value(json!({
        "data": [{"id": "inv1", "number": "INV-0001"}],
        "meta": {"pagination": {
            "total": 1, "count": 1, "per_page": 50, "current_page": 1, "total_pages": 1
        }},
    }))
    .unwrap();

    assert_eq!(body.data.len(), 1);
    assert_eq!(body.data[0].number, "INV-0001");
    let pagination = body.meta.unwrap().pagination.unwrap();
    assert_eq!(pagination.total, 1);
    assert_eq!(pagination.per_page, 50);
}

#[test]
fn document_round_trip_preserves_unknown_fields() {
    // `type` is a Rust keyword, so the field is `r#type` but must still map
    // to the plain JSON key.
    let payload = json!({
        "id": "doc1",
        "name": "contract.pdf",
        "type": "pdf",
        "hash": "abc",
        "is_default": false,
        "custom_value1": "unmodeled field",
    });

    let doc: Document = serde_json::from_value(payload).unwrap();
    assert_eq!(doc.name, "contract.pdf");
    assert_eq!(doc.r#type, "pdf");

    let round_tripped = serde_json::to_value(&doc).unwrap();
    assert_eq!(round_tripped["type"], "pdf");
    assert_eq!(round_tripped["custom_value1"], "unmodeled field");
}

// --- list params ------------------------------------------------------

/// The typed fields serialize; `custom` is skipped here because it is
/// applied as a separate query pass (serde_urlencoded can't flatten a map).
#[test]
fn list_params_skips_custom_in_struct_serialization() {
    let params = ListParams::new().per_page(25).param("client_id", "abc123");

    assert_eq!(
        serde_json::to_value(&params).unwrap(),
        json!({"per_page": 25})
    );
    assert_eq!(
        params.custom,
        vec![("client_id".to_string(), "abc123".to_string())]
    );
}

#[test]
fn list_params_custom_pairs_accumulate_and_allow_repeats() {
    let params = ListParams::new()
        .param("client_id", "abc123")
        .param("status", "active")
        .param("status", "paid");

    assert_eq!(params.custom.len(), 3);
    assert_eq!(
        params.custom,
        vec![
            ("client_id".to_string(), "abc123".to_string()),
            ("status".to_string(), "active".to_string()),
            ("status".to_string(), "paid".to_string()),
        ]
    );
}

/// Both halves of the query must survive urlencoding, since the client
/// serializes the typed struct and the custom pairs in two passes.
#[test]
fn list_params_both_halves_urlencode() {
    let params = ListParams::new().per_page(25).sort("created_at|desc");

    let typed = serde_urlencoded::to_string(&params).unwrap();
    assert!(typed.contains("per_page=25"));
    assert!(typed.contains("sort=created_at%7Cdesc"));

    let custom = serde_urlencoded::to_string(
        &ListParams::new()
            .param("client_id", "abc123")
            .param("is_deleted", "false")
            .custom,
    )
    .unwrap();
    assert_eq!(custom, "client_id=abc123&is_deleted=false");
}

// --- newly added resource groups ---------------------------------------

#[test]
fn company_keeps_settings_untyped_and_preserves_toggles() {
    // Company has well over a hundred fields; only the structural ones are
    // modeled, and `settings` stays untyped.
    let payload = json!({
        "id": "co1",
        "company_key": "abc",
        "settings": {"currency_id": "1", "timezone_id": "15"},
        "enabled_modules": 32767,
        "show_product_cost": true,
        "markdown_enabled": true,
    });

    let company: Company = serde_json::from_value(payload).unwrap();
    assert_eq!(company.company_key, "abc");
    assert_eq!(company.enabled_modules, 32767);
    assert!(company.markdown_enabled);
    assert_eq!(company.settings["currency_id"], "1");
    // An unmodeled feature toggle still round-trips.
    let round_tripped = serde_json::to_value(&company).unwrap();
    assert_eq!(round_tripped["show_product_cost"], true);
    assert_eq!(round_tripped["settings"]["timezone_id"], "15");
}

/// The spec says system-log timestamps are date strings; a real server sends
/// integers, as it does everywhere else.
#[test]
fn system_log_timestamps_are_integers() {
    let log: SystemLog = serde_json::from_value(json!({
        "id": "sl1",
        "event_id": 30,
        "category_id": 1,
        "type_id": 300,
        "log": {"message": "gateway declined"},
        "created_at": 1792869488,
        "updated_at": 1792869489,
    }))
    .unwrap();

    assert_eq!(log.event_id, 30);
    assert_eq!(log.created_at, 1792869488);
    assert_eq!(log.updated_at, 1792869489);
    assert_eq!(log.log["message"], "gateway declined");
}

#[test]
fn new_resource_models_decode() {
    let integration: BankIntegration = serde_json::from_value(json!({
        "id": "bi1", "bank_account_id": 7788, "balance": 1234.56, "currency": "USD"
    }))
    .unwrap();
    assert_eq!(integration.bank_account_id, 7788);
    assert_eq!(integration.balance, 1234.56);

    let rule: BankTransactionRule = serde_json::from_value(json!({
        "id": "btr1",
        "name": "Coffee",
        "matches_on_all": true,
        "rules": [{"search_key": "description", "operator": "contains", "value": "cafe"}],
    }))
    .unwrap();
    assert_eq!(rule.rules.len(), 1);
    assert_eq!(rule.rules[0]["operator"], "contains");

    let gateway: CompanyGateway = serde_json::from_value(json!({
        "id": "cg1", "gateway_key": "d14dd26a", "accepted_credit_cards": 32, "config": "encrypted"
    }))
    .unwrap();
    assert_eq!(gateway.accepted_credit_cards, 32);
    assert_eq!(gateway.config, "encrypted");

    let tag: Tag = serde_json::from_value(json!({
        "id": "t1", "name": "urgent", "color": "#ff0000", "entity_type": "invoice"
    }))
    .unwrap();
    assert_eq!(tag.name, "urgent");

    let location: Location = serde_json::from_value(json!({
        "id": "loc1", "name": "Warehouse", "city": "Brisbane", "is_shipping_location": true
    }))
    .unwrap();
    assert!(location.is_shipping_location);

    let scheduler: TaskScheduler = serde_json::from_value(json!({
        "id": "ts1", "job": "email_report", "repeat_every": "WEEK",
        "start_from": 1700000000i64, "report_keys": ["date", "amount"],
    }))
    .unwrap();
    assert_eq!(scheduler.report_keys, vec!["date", "amount"]);

    let token: CompanyToken = serde_json::from_value(json!({
        "id": "tok1", "name": "CI token", "token": "secret", "is_system": false
    }))
    .unwrap();
    assert_eq!(token.name, "CI token");

    let gw_token: ClientGatewayToken = serde_json::from_value(json!({
        "id": "cgt1", "client_id": "abc123", "token": "card_x", "is_default": true
    }))
    .unwrap();
    assert!(gw_token.is_default);

    let group: GroupSetting = serde_json::from_value(json!({
        "id": "gs1", "name": "Wholesale", "settings": {"currency_id": "2"}
    }))
    .unwrap();
    assert_eq!(group.settings["currency_id"], "2");

    let design: Design = serde_json::from_value(json!({
        "id": "d1", "name": "Clean", "is_custom": false, "is_active": true
    }))
    .unwrap();
    assert!(design.is_active);

    let user: User = serde_json::from_value(json!({
        "id": "u1", "first_name": "Ada", "email": "ada@example.com", "passkey_count": 2
    }))
    .unwrap();
    assert_eq!(user.first_name, "Ada");
    assert_eq!(user.passkey_count, 2);
}

/// `Design.design` is typed as a string upstream but returned as a nested
/// object, so it is left unmodeled and must survive in `extra`.
#[test]
fn design_body_round_trips_through_extra() {
    let payload = json!({
        "id": "d1",
        "name": "Custom",
        "design": {"header": "<h1>", "body": "<div>", "footer": "<p>"},
    });

    let design: Design = serde_json::from_value(payload).unwrap();
    let round_tripped = serde_json::to_value(&design).unwrap();
    assert_eq!(round_tripped["design"]["header"], "<h1>");
}

#[test]
fn statics_round_trip_preserves_unmodeled_tables_and_fields() {
    let payload = json!({
        "currencies": [{ "id": 1, "name": "US Dollar", "code": "USD", "precision": 2 }],
        "countries": [{ "id": 840, "name": "United States", "iso_3166_3": "USA" }],
        "gateways": [{ "id": 1, "name": "Stripe" }],
        "some_future_table": [{ "x": 1 }]
    });

    let statics: Statics = serde_json::from_value(payload).unwrap();
    let back = serde_json::to_value(&statics).unwrap();

    assert_eq!(back["countries"][0]["iso_3166_3"], "USA");
    assert_eq!(back["some_future_table"][0]["x"], 1);
    assert_eq!(back["gateways"][0]["name"], "Stripe");
    assert_eq!(
        back["currencies"][0]["id"], "1",
        "ids are normalised to strings on the way back out"
    );
}

#[test]
fn search_results_round_trip_preserves_unknown_buckets() {
    let payload = json!({
        "clients": [{ "name": "Acme", "type": "/client", "id": "a1", "path": "/clients/a1" }],
        "future_bucket": [{ "name": "x" }]
    });

    let results: SearchResults = serde_json::from_value(payload).unwrap();
    assert_eq!(results.clients[0].r#type, "/client");

    let back = serde_json::to_value(&results).unwrap();
    assert_eq!(back["future_bucket"][0]["name"], "x");
    assert_eq!(back["clients"][0]["type"], "/client");
}

#[test]
fn health_check_round_trip_preserves_unknown_fields() {
    let payload = json!({
        "system_health": false,
        "api_version": "5.8.56",
        "queue_data": { "pending": 3 },
        "brand_new_probe": true
    });

    let health: HealthCheck = serde_json::from_value(payload).unwrap();
    assert_eq!(health.system_health, Some(false));

    let back = serde_json::to_value(&health).unwrap();
    assert_eq!(back["brand_new_probe"], true);
    assert_eq!(back["queue_data"]["pending"], 3);
}

#[test]
fn statics_ids_accept_both_strings_and_numbers() {
    let payload = json!({
        "payment_types": [{ "id": "1", "name": "Bank Transfer" }],
        "industries": [{ "id": 3, "name": "Advertising" }],
        "sizes": [{ "id": null, "name": "Unknown" }]
    });

    let statics: Statics = serde_json::from_value(payload).unwrap();
    assert_eq!(statics.payment_types[0].id, "1");
    assert_eq!(statics.industries[0].id, "3");
    assert_eq!(statics.sizes[0].id, "", "a null id decodes as empty");
}

#[test]
fn login_response_types_the_token_and_preserves_the_rest() {
    let payload = json!({
        "data": [{
            "is_owner": true,
            "is_admin": true,
            "token": { "id": "t1", "token": "abc" },
            "company": { "id": "c1", "settings": {} },
            "user": { "id": "u1", "email": "me@example.com" }
        }]
    });

    let login: LoginResponse = serde_json::from_value(payload).unwrap();
    assert!(login.data[0].is_owner);
    let token = login.data[0].token.as_ref().unwrap();
    assert_eq!(token.token, "abc");

    let back = serde_json::to_value(&login).unwrap();
    assert_eq!(back["data"][0]["company"]["id"], "c1");
    assert_eq!(back["data"][0]["user"]["email"], "me@example.com");
}

#[test]
fn passkey_round_trip_preserves_last_used_at() {
    let used = json!({
        "id": "p1", "name": "Laptop", "created_at": "2026-01-01T00:00:00.000000Z",
        "last_used_at": 1700000000
    });
    let never_used = json!({
        "id": "p2", "name": "Phone", "created_at": "2026-02-01T00:00:00.000000Z",
        "last_used_at": null
    });

    let used: Passkey = serde_json::from_value(used).unwrap();
    let never_used: Passkey = serde_json::from_value(never_used).unwrap();
    assert_eq!(used.last_used_at, Some(1700000000));
    assert_eq!(never_used.last_used_at, None);
}
