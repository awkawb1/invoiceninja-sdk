use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A server-side log entry (gateway responses, mail failures, and so on).
///
/// Note that unlike every other resource, this one's `created_at` and
/// `updated_at` are returned as date **strings**, not integer timestamps.
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
    pub created_at: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub updated_at: String,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
