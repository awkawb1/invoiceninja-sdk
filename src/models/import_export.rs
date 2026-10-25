//! The company import and export jobs.
//!
//! Both endpoints queue work rather than doing it, so neither reply carries
//! the result — an export answers with the URL to fetch later, and an import
//! answers with nothing but an acknowledgement (see `MessageResponse`).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The reply to `POST /export`.
///
/// The export itself is queued, so `url` is a promise rather than a ready
/// file: fetching it before the job finishes gets nothing. The link points at
/// the `protected_download` route and expires — in one hour, or five if the
/// company has more than 10,000 activities — so it is worth following
/// promptly rather than storing.
///
/// `url` is the only reason to call this endpoint, which is why it is typed
/// rather than left in `extra`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ExportJob {
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub message: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub url: String,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// The reply to `POST /preimport` — the first half of a CSV import.
///
/// `hash` is the handle to the uploaded files, which the server caches for 20
/// minutes. Pass it back in the `import` body to start the ingest; without it
/// the files would have to be uploaded again.
///
/// `mappings` is keyed by entity type (`client`, `invoice`, …), one entry per
/// file uploaded. Each value describes what that file's columns could map to:
/// `available` lists the importable fields, `headers` holds the first two
/// rows, and `hints` is the server's guess at a column mapping. Those three
/// stay `Value` because the upstream shapes come from each entity's own
/// `importable()` map and are not documented; a wrong concrete type here
/// would be a hard decode error, where `Value` is lossless.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ImportPreview {
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub hash: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub mappings: Map<String, Value>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
