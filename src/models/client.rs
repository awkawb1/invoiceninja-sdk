use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Client {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub name: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub id_number: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub vat_number: String,
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
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub country_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub phone: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub website: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub private_notes: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub balance: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub paid_to_date: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub credit_balance: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub is_deleted: bool,
    #[serde(default)]
    pub archived_at: Option<i64>,
    #[serde(default)]
    pub created_at: Option<i64>,
    #[serde(default)]
    pub updated_at: Option<i64>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub contacts: Vec<ClientContact>,

    /// Any fields returned by the API that aren't modeled above are preserved here,
    /// so a `get` followed by an `update` round-trips without data loss.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientContact {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub first_name: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub last_name: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub email: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub phone: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub is_primary: bool,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
