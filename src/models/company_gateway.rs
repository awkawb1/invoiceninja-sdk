use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A configured payment gateway.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompanyGateway {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub gateway_key: String,
    /// Bitmask of accepted card types.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub accepted_credit_cards: i64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub require_billing_address: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub require_shipping_address: bool,
    /// An encrypted JSON *string*, not an object — decrypted server-side, so
    /// it is opaque here.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub config: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub update_details: bool,
    /// Per-card fee and limit rules; shape varies by gateway.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub fees_and_limits: Vec<Value>,
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
