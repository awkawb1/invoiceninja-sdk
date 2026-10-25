use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::invoice::InvoiceItem;

/// A recurring invoice template. Shares the invoices table with [`Invoice`],
/// plus the recurrence fields (`frequency_id`, `remaining_cycles`,
/// `next_send_date`, auto-billing).
///
/// [`Invoice`]: super::Invoice
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecurringInvoice {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub client_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub vendor_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub project_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub number: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub po_number: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub status_id: String,
    /// How often the invoice recurs. Invoice Ninja returns this as an integer
    /// enum (1 = daily, 2 = weekly, … ), not a human-readable string. A real
    /// server sends it as a numeric string; both decode.
    #[serde(default, deserialize_with = "super::common::int_or_numeric_string")]
    pub frequency_id: i64,
    /// Number of cycles left to send; `-1` means it recurs indefinitely.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub remaining_cycles: i64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub auto_bill: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub auto_bill_enabled: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub date: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub due_date: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub next_send_date: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub last_sent_date: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub amount: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub balance: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub paid_to_date: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub discount: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub partial: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub exchange_rate: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub is_amount_discount: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub uses_inclusive_taxes: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub public_notes: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub private_notes: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub terms: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub footer: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub design_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub is_deleted: bool,
    #[serde(default)]
    pub last_viewed: Option<i64>,
    #[serde(default)]
    pub archived_at: Option<i64>,
    #[serde(default)]
    pub created_at: Option<i64>,
    #[serde(default)]
    pub updated_at: Option<i64>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub line_items: Vec<InvoiceItem>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Recurring quotes share the recurring-invoice schema (the API's
/// `RecurringQuote` fields are a strict subset of `RecurringInvoice`), so the
/// `/recurring_quotes` endpoint returns the same JSON shape.
pub type RecurringQuote = RecurringInvoice;
