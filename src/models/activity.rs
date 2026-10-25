use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// One row of the activity log, as `GET /activities` returns it.
///
/// Every reference is a hashed id, or an empty string when the activity does
/// not concern that kind of record. `activity_type_id` arrives as a string
/// here but as a number in the [feed](ActivityFeedEntry).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Activity {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub activity_type_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub user_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub company_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub client_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub contact_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub vendor_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub vendor_contact_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub invoice_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub quote_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub credit_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub payment_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub recurring_invoice_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub recurring_expense_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub purchase_order_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub expense_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub task_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub token_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub is_system: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub notes: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub ip: String,
    #[serde(default)]
    pub created_at: Option<i64>,
    #[serde(default)]
    pub updated_at: Option<i64>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A named record an activity refers to: the display label and its hashed id.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActivityRef {
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub label: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub hashed_id: String,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// One entry of the activity feed (`activity_feed`, `entity_activity`).
///
/// A different shape from [`Activity`]: the server resolves the records an
/// activity mentions into labelled references. Which references are present
/// depends on the activity type, so each is optional, and any the server adds
/// that this does not name lands in `extra`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActivityFeedEntry {
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub hashed_id: String,
    #[serde(default, deserialize_with = "super::common::int_or_numeric_string")]
    pub activity_type_id: i64,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub notes: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub ip: String,
    #[serde(default, deserialize_with = "super::common::int_or_numeric_string")]
    pub created_at: i64,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<ActivityRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client: Option<ActivityRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vendor: Option<ActivityRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invoice: Option<ActivityRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quote: Option<ActivityRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credit: Option<ActivityRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment: Option<ActivityRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<ActivityRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expense: Option<ActivityRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purchase_order: Option<ActivityRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recurring_invoice: Option<ActivityRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recurring_expense: Option<ActivityRef>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
