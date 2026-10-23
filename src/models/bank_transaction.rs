use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A transaction imported from a connected bank account.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BankTransaction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The upstream transaction identifier, returned as an integer rather
    /// than a hashed id.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub transaction_id: i64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub bank_account_id: i64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub amount: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub currency_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub account_type: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub description: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub category_id: i64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub category_type: String,
    /// Either `CREDIT` or `DEBIT`.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub base_type: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub date: String,
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
