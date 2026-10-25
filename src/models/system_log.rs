use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A server-side log entry (gateway responses, mail failures, and so on).
///
/// The OpenAPI spec declares `created_at` and `updated_at` as date strings,
/// but a real server (5.13.50) sends integer timestamps like every other
/// resource.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemLog {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub user_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub client_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub event_id: i64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub category_id: i64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub type_id: i64,
    /// The logged payload; shape depends on the event.
    #[serde(default)]
    pub log: Value,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub created_at: i64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub updated_at: i64,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
