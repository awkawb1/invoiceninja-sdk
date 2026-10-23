use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A named settings group that clients can be assigned to.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GroupSetting {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub user_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub name: String,
    /// The settings override blob. Left untyped — Invoice Ninja's
    /// `CompanySettings` object has well over a hundred fields.
    #[serde(default)]
    pub settings: Value,
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
