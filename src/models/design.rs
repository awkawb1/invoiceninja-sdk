use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A PDF template.
///
/// The `design` field itself (the HTML section bodies) is left to
/// [`extra`](Self::extra): the spec types it as a string, but the API
/// returns a nested object of `header`/`body`/`footer` sections.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Design {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub name: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub is_custom: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub is_active: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub is_deleted: bool,
    #[serde(default)]
    pub archived_at: Option<i64>,
    #[serde(default)]
    pub created_at: Option<i64>,
    #[serde(default)]
    pub updated_at: Option<i64>,
    #[serde(default)]
    pub deleted_at: Option<i64>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
