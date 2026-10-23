use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A scheduled job (recurring report emails, and similar).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskScheduler {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub paused: bool,
    /// Frequency, e.g. `DAY`, `WEEK`, `MONTH`.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub repeat_every: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub start_from: i64,
    /// The job to run, e.g. `email_report`.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub job: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub date_range: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub date_key: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub start_date: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub end_date: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub report_keys: Vec<String>,
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
