use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A connected bank account feed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BankIntegration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub user_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub provider_bank_name: String,
    /// Returned as an integer rather than a hashed id.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub bank_account_id: i64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub bank_account_name: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub bank_account_number: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub bank_account_status: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub bank_account_type: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub balance: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub currency: String,
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
