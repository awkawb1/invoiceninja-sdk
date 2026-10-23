use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// An API token. Served from `/tokens`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompanyToken {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub name: String,
    /// Only returned in full when the token is first created.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub token: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub is_system: bool,
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
