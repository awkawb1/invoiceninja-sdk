use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A stored payment method for a client.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientGatewayToken {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub client_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub token: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub routing_number: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub company_gateway_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub is_default: bool,
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
