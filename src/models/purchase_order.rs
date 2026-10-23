use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::invoice::InvoiceItem;

/// A purchase order. Structurally close to an invoice, but addressed to a
/// vendor rather than a client.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PurchaseOrder {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub vendor_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub client_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub invoice_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub project_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub number: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub po_number: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub quote_number: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub status_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub date: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub due_date: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub partial_due_date: String,
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
    pub currency_id: String,
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
