use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// An outbound webhook registration (the API calls these "subscriptions"
/// internally, but they are served from `/webhooks`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Webhook {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The event this hook fires on, as an integer-enum string.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub event_id: String,
    /// The URL Invoice Ninja posts to.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub target_url: String,
    /// Payload format: `JSON` or `UBL`.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub format: String,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
