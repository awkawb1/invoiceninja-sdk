use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A physical address attached to a client or vendor.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Location {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub name: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub address1: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub address2: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub city: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub state: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub postal_code: String,
    /// Omitted when empty: the server validates it as an integer, so an unset
    /// `""` is rejected with a 422 and a default `Location` could not be
    /// created.
    #[serde(
        default,
        deserialize_with = "super::common::null_to_default",
        skip_serializing_if = "String::is_empty"
    )]
    pub country_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub user_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub assigned_user_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub client_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub vendor_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub is_shipping_location: bool,
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
