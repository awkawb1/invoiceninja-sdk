use serde::Deserialize;
use std::collections::HashMap;

/// Error shape returned by the Invoice Ninja API for failed requests.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiErrorBody {
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub errors: HashMap<String, Vec<String>>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("http transport error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("failed to decode JSON response: {0}")]
    Json(#[from] serde_json::Error),

    #[error("api returned {status}: {message}")]
    Api {
        status: reqwest::StatusCode,
        message: String,
        errors: HashMap<String, Vec<String>>,
    },

    #[error("invalid base url: {0}")]
    InvalidBaseUrl(String),

    #[error("invalid api token: {0}")]
    InvalidToken(String),

    /// An id or action could not be used as a single URL path segment.
    ///
    /// Raised for values that would change which endpoint is addressed rather
    /// than identifying a record — an empty id (which would hit the
    /// collection route) or one made only of dots (which the URL parser
    /// resolves away). Invoice Ninja ids are hashids, so this only rejects
    /// input that was never a valid id.
    #[error("`{0}` cannot be used as a url path segment")]
    InvalidPathSegment(String),

    /// A `list_all_*` call kept receiving full pages past the point where any
    /// plausible result set would have ended.
    ///
    /// This means the server is not paginating the way the client expects —
    /// typically because it ignores the `page` parameter and keeps replaying
    /// the same page. Raised instead of returning the partial result, so the
    /// truncation cannot be mistaken for a complete answer.
    #[error("pagination did not terminate for {path} after {pages} pages")]
    PaginationLimit { path: String, pages: u64 },
}

pub type Result<T> = std::result::Result<T, Error>;
