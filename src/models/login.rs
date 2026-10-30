use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::CompanyToken;

/// Response of `POST /login` (via [`crate::InvoiceNinjaClient::login`]).
///
/// One entry per company the account can access. The full upstream payload
/// also nests the whole company (clients, invoices, designs, ...), the
/// user, and the account under each entry — this crate does not model that;
/// it stays reachable, losslessly, through [`LoginCompanyUser::extra`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoginResponse {
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub data: Vec<LoginCompanyUser>,
}

/// One company membership returned by a login, carrying that company's API
/// token.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoginCompanyUser {
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub is_owner: bool,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub is_admin: bool,
    /// The token to pass to [`InvoiceNinjaClient::new`](crate::InvoiceNinjaClient::new)
    /// for this company. Only absent if the server ever omits the `token`
    /// include; every server-side `first_load` request requests it.
    #[serde(default)]
    pub token: Option<CompanyToken>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
