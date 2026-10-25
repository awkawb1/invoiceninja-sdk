//! Goal 2, checked mechanically: every model must absorb a field that is
//! `null` or absent.
//!
//! Invoice Ninja returns `null` for many nominally-string fields and omits
//! others, so a model that fails on either takes the whole response down with
//! it. Rather than trusting each field to carry the right serde attribute,
//! this serialises `Default` (which emits every non-optional field), then for
//! each field in turn nulls it and, separately, removes it, and requires the
//! result to still decode.
//!
//! A wrong *type* is deliberately not covered: that is meant to be a hard
//! error, so it is not something to absorb.

use invoiceninjars::models::*;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

/// Returns one line per mutation that failed to decode.
fn failures<T: Serialize + DeserializeOwned + Default>(name: &str) -> Vec<String> {
    let base = serde_json::to_value(T::default()).expect("default serialises");
    let object = base
        .as_object()
        .unwrap_or_else(|| panic!("{name}: default did not serialise to an object"));

    let mut out = Vec::new();
    for key in object.keys() {
        let mut nulled = base.clone();
        nulled[key] = Value::Null;
        if let Err(e) = serde_json::from_value::<T>(nulled) {
            out.push(format!("{name}.{key} = null: {e}"));
        }

        let mut removed = base.clone();
        removed.as_object_mut().unwrap().remove(key);
        if let Err(e) = serde_json::from_value::<T>(removed) {
            out.push(format!("{name}.{key} absent: {e}"));
        }
    }
    out
}

macro_rules! tolerant {
    ($($ty:ty),+ $(,)?) => {{
        let mut all = Vec::new();
        $( all.extend(failures::<$ty>(stringify!($ty))); )+
        all
    }};
}

#[test]
fn every_model_absorbs_a_null_or_missing_field() {
    let all = tolerant![
        Activity,
        ActivityFeedEntry,
        ActivityRef,
        BankIntegration,
        BankTransaction,
        BankTransactionRule,
        Client,
        ClientContact,
        ClientGatewayToken,
        Company,
        CompanyGateway,
        CompanyToken,
        Design,
        Document,
        Expense,
        ExpenseCategory,
        GroupSetting,
        Invoice,
        InvoiceItem,
        Location,
        Payment,
        PaymentInvoice,
        PaymentTerm,
        Product,
        Project,
        PurchaseOrder,
        RecurringExpense,
        RecurringInvoice,
        Subscription,
        SubscriptionStep,
        SystemLog,
        Tag,
        Task,
        TaskScheduler,
        TaskStatus,
        TaxRate,
        User,
        Vendor,
        VendorContact,
        Webhook,
    ];

    assert!(
        all.is_empty(),
        "{} field(s) break decoding on null/absent:\n{}",
        all.len(),
        all.join("\n")
    );
}

/// A guard on the test itself: if `Default` stopped emitting fields, the loop
/// above would pass vacuously.
#[test]
fn the_tolerance_test_actually_exercises_fields() {
    let client = serde_json::to_value(Client::default()).unwrap();
    assert!(
        client.as_object().unwrap().len() > 10,
        "Client::default() should serialise many fields"
    );
}

/// The mutation really does bite: a field without `null_to_default` fails.
#[test]
fn a_bare_non_option_field_would_be_caught() {
    #[derive(serde::Serialize, serde::Deserialize, Default)]
    struct Naive {
        #[serde(default)]
        name: String,
    }
    assert_eq!(
        failures::<Naive>("Naive").len(),
        1,
        "only the null case fails"
    );
}
