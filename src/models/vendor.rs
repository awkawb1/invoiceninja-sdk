use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Vendor {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub id_number: String,
    #[serde(default)]
    pub vat_number: String,
    #[serde(default)]
    pub address1: String,
    #[serde(default)]
    pub address2: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub postal_code: String,
    #[serde(default)]
    pub country_id: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub website: String,
    #[serde(default)]
    pub private_notes: String,
    #[serde(default)]
    pub is_deleted: bool,
    #[serde(default)]
    pub archived_at: Option<i64>,
    #[serde(default)]
    pub created_at: Option<i64>,
    #[serde(default)]
    pub updated_at: Option<i64>,
    #[serde(default)]
    pub contacts: Vec<VendorContact>,

    /// Any fields returned by the API that aren't modeled above are preserved here,
    /// so a `get` followed by an `update` round-trips without data loss.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VendorContact {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub is_primary: bool,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
