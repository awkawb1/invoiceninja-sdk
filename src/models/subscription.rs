use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A subscription (billing plan) definition.
///
/// Note that unlike [`RecurringInvoice::frequency_id`], the API returns this
/// resource's `frequency_id` as a string.
///
/// [`RecurringInvoice::frequency_id`]: super::RecurringInvoice::frequency_id
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Subscription {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub product_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub recurring_invoice_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub frequency_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub auto_bill: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub promo_code: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub promo_discount: f64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub is_recurring: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub is_amount_discount: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub allow_cancellation: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub per_seat_enabled: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub max_seats_limit: i64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub trial_enabled: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub trial_duration: i64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub allow_query_overrides: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub allow_plan_changes: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub refund_period: i64,
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

/// One step of the subscription purchase flow and the steps it depends on,
/// as returned by `GET /subscriptions/steps`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubscriptionStep {
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub dependencies: Vec<String>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
