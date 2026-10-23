use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Response of `GET /health_check`.
///
/// Two different payloads arrive here depending on the deployment. A
/// self-hosted install returns the full `SystemHealth::check()` report
/// modelled below; the hosted app returns only `{"message": "", "errors":
/// []}`, since the check is not meaningful there. Every field defaults, so
/// both decode — test [`system_health`](Self::system_health) rather than
/// assuming the detail fields are populated.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Overall verdict. Absent on hosted, where no check runs.
    #[serde(default)]
    pub system_health: Option<bool>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub api_version: String,
    #[serde(default)]
    pub php_version: Option<PhpVersion>,
    /// Required PHP extensions and whether each was found.
    #[serde(default)]
    pub extensions: Value,
    #[serde(default)]
    pub env_writable: Option<bool>,
    #[serde(default)]
    pub simple_db_check: Option<bool>,
    #[serde(default)]
    pub cache_enabled: Option<bool>,
    #[serde(default)]
    pub exec: Option<bool>,
    #[serde(default)]
    pub open_basedir: Option<bool>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub mail_mailer: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub pdf_engine: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub queue: String,
    /// Queue depth and the most recent failure, when the queue is readable.
    #[serde(default)]
    pub queue_data: Value,
    #[serde(default)]
    pub trailing_slash: Option<bool>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub file_permissions: String,
    #[serde(default)]
    pub exchange_rate_api_not_configured: Option<bool>,
    #[serde(default)]
    pub is_docker: Option<bool>,
    #[serde(default)]
    pub pending_migrations: Option<bool>,

    /// Set on the hosted app, which returns a bare message instead of a
    /// report.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub message: String,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// The PHP version sub-report of a health check.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhpVersion {
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub current_php_version: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub minimum_php_version: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub current_php_cli_version: String,
    #[serde(default)]
    pub is_okay: Option<bool>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub memory_limit: String,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
