use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Invoice {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub client_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub number: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub date: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub due_date: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub po_number: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub status_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub amount: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub balance: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub discount: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub is_amount_discount: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub public_notes: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub private_notes: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub terms: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub footer: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub is_deleted: bool,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InvoiceItem {
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub product_key: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub notes: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub cost: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub quantity: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub discount: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub tax_name1: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub tax_rate1: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub tax_name2: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub tax_rate2: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub line_total: f64,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Invoice Ninja stores quotes in the same table/schema as invoices
/// (an `entity_type` distinguishes them), so the `/quotes` endpoint
/// returns the same JSON shape as `/invoices`.
pub type Quote = Invoice;

/// Likewise, credits share the invoices table/schema, so the `/credits`
/// endpoint returns the same JSON shape as `/invoices`.
pub type Credit = Invoice;
