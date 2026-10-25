use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Payment {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub client_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub amount: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub applied: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub refunded: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub date: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub transaction_reference: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub type_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub private_notes: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub is_deleted: bool,
    #[serde(default)]
    pub archived_at: Option<i64>,
    #[serde(default)]
    pub created_at: Option<i64>,
    #[serde(default)]
    pub updated_at: Option<i64>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub invoices: Vec<PaymentInvoice>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaymentInvoice {
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub invoice_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub amount: f64,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
