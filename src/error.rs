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
}

pub type Result<T> = std::result::Result<T, Error>;
