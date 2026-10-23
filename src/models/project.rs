use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Project {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub client_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub name: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub number: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub task_rate: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub due_date: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub budgeted_hours: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub color: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub public_notes: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub private_notes: String,
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
