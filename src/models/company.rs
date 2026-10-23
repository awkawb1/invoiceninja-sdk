use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A company (tenant).
///
/// The API returns well over a hundred fields here; this models the
/// identifying and structural ones and leaves the rest — including every
/// `enable_*`/`show_*` feature toggle — to [`extra`](Self::extra), which
/// still round-trips them on update.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Company {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub company_key: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub size_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub industry_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub portal_mode: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub subdomain: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub portal_domain: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub logo: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub db: String,
    /// The company settings blob. Left untyped — `CompanySettings` is the
    /// largest object in the API.
    #[serde(default)]
    pub settings: Value,
    /// Custom field label definitions.
    #[serde(default)]
    pub custom_fields: Value,
    /// Bitmask of enabled modules.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub enabled_modules: i64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub enabled_tax_rates: i64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub enabled_item_tax_rates: i64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub enabled_expense_tax_rates: i64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub first_day_of_week: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub first_month_of_year: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub slack_webhook_url: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub google_analytics_key: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub session_timeout: i64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub markdown_enabled: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub client_can_register: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub is_large: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub is_disabled: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub calculate_taxes: bool,
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
