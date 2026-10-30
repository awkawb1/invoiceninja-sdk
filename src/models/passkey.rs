use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A WebAuthn passkey credential registered against the current user.
/// Served from `GET /settings/passkeys`.
///
/// Registering a new passkey is not wrapped: the server's `/settings/
/// passkeys/options` and `/settings/passkeys` routes exist to drive a
/// WebAuthn ceremony that only a browser, talking to its platform
/// authenticator, can complete — there is no attestation for this crate to
/// produce on its own. Listing and deleting an existing credential need no
/// such ceremony, so those are the two passkey operations this crate
/// offers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Passkey {
    /// Hashed id — pass to [`crate::InvoiceNinjaClient::delete_passkey`] to
    /// remove this credential.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub name: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub created_at: String,
    /// Unix timestamp of the credential's last successful authentication,
    /// or `None` if it has never been used to log in.
    #[serde(default)]
    pub last_used_at: Option<i64>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
