use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A document (file) attached to another entity.
///
/// Documents are created by uploading to the owning entity
/// (`PUT /{resource}/{id}/upload`, multipart) rather than by posting to
/// `/documents`, so this crate exposes read, update, delete, and download
/// but no `create_document`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Document {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub user_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub assigned_user_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub project_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub vendor_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub name: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub url: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub preview: String,
    /// File extension, e.g. `pdf`.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub r#type: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub disk: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub hash: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub is_deleted: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub is_default: bool,
    #[serde(default)]
    pub created_at: Option<i64>,
    #[serde(default)]
    pub updated_at: Option<i64>,
    #[serde(default)]
    pub deleted_at: Option<i64>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
