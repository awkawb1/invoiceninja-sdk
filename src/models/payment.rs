use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Payment {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub amount: f64,
    #[serde(default)]
    pub applied: f64,
    #[serde(default)]
    pub refunded: f64,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub transaction_reference: String,
    #[serde(default)]
    pub type_id: String,
    #[serde(default)]
    pub private_notes: String,
    #[serde(default)]
    pub is_deleted: bool,
    #[serde(default)]
    pub archived_at: Option<i64>,
    #[serde(default)]
    pub created_at: Option<i64>,
    #[serde(default)]
    pub updated_at: Option<i64>,
    #[serde(default)]
    pub invoices: Vec<PaymentInvoice>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaymentInvoice {
    pub invoice_id: String,
    pub amount: f64,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
