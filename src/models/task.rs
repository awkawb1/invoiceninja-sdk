use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Task {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub invoice_id: String,
    #[serde(default)]
    pub project_id: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub rate: f64,
    #[serde(default)]
    pub status_id: String,
    #[serde(default)]
    pub is_running: bool,
    #[serde(default)]
    pub is_deleted: bool,
    /// JSON-encoded array of `[start_timestamp, end_timestamp]` pairs, as returned by the API.
    #[serde(default)]
    pub time_log: String,
    #[serde(default)]
    pub archived_at: Option<i64>,
    #[serde(default)]
    pub created_at: Option<i64>,
    #[serde(default)]
    pub updated_at: Option<i64>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
