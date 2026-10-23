use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Product {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub product_key: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub notes: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub cost: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub price: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub quantity: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub tax_name1: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub tax_rate1: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub tax_name2: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub tax_rate2: f64,
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
