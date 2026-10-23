use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaymentTerm {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Human-readable form of the term, e.g. `NET 30`.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub name: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub num_days: i64,
    #[serde(default)]
    pub archived_at: Option<i64>,
    #[serde(default)]
    pub created_at: Option<i64>,
    #[serde(default)]
    pub updated_at: Option<i64>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
