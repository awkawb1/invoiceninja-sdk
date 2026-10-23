use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Expense {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default)]
    pub vendor_id: String,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub invoice_id: String,
    #[serde(default)]
    pub amount: f64,
    #[serde(default)]
    pub exchange_rate: f64,
    #[serde(default)]
    pub public_notes: String,
    #[serde(default)]
    pub private_notes: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub payment_date: String,
    #[serde(default)]
    pub transaction_reference: String,
    #[serde(default)]
    pub category_id: String,
    #[serde(default)]
    pub tax_name1: String,
    #[serde(default)]
    pub tax_rate1: f64,
    #[serde(default)]
    pub tax_name2: String,
    #[serde(default)]
    pub tax_rate2: f64,
    #[serde(default)]
    pub should_be_invoiced: bool,
    #[serde(default)]
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
