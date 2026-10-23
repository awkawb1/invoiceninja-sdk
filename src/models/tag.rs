use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Tag {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub entity_type: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub name: String,
    /// Hex colour value.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub color: String,
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
