use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A rule for auto-categorising imported bank transactions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BankTransactionRule {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub user_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub name: String,
    /// Match clauses. Left untyped: the shape is a nested
    /// `search_key`/`operator`/`value` object the spec does not define.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub rules: Vec<Value>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub auto_convert: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub matches_on_all: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub applies_to: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub client_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub vendor_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub category_id: String,
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
