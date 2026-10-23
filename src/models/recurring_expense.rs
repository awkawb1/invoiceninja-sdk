use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A recurring expense template: an expense plus the recurrence fields
/// (`frequency_id`, `remaining_cycles`, `next_send_date`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecurringExpense {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub client_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub vendor_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub invoice_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub project_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub category_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub number: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub amount: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub foreign_amount: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub exchange_rate: f64,
    /// Integer enum matching [`RecurringInvoice::frequency_id`].
    ///
    /// [`RecurringInvoice::frequency_id`]: super::RecurringInvoice::frequency_id
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub frequency_id: i64,
    /// Number of cycles left; `-1` means it recurs indefinitely.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub remaining_cycles: i64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub date: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub payment_date: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub last_sent_date: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub next_send_date: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub public_notes: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub private_notes: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub transaction_reference: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub tax_name1: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub tax_rate1: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub tax_name2: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub tax_rate2: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub tax_name3: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub tax_rate3: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub should_be_invoiced: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub uses_inclusive_taxes: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub is_deleted: bool,
    #[serde(default)]
    pub archived_at: Option<i64>,
    #[serde(default)]
    pub created_at: Option<i64>,
    #[serde(default)]
    pub updated_at: Option<i64>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
