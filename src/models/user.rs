use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A user account.
///
/// Every `/users` endpoint except the index listing sits behind the API's
/// `password_protected` middleware, so reads of a single user and all writes
/// need an `X-API-PASSWORD` header — see
/// [`InvoiceNinjaClient::with_api_password`](crate::InvoiceNinjaClient::with_api_password).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct User {
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
    pub signature: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub avatar: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub language_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub accepted_terms_version: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub oauth_user_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub oauth_provider_id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub verified_phone_number: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub has_password: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub last_confirmed_email_address: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub passkey_enabled: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub passkey_count: i64,
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
