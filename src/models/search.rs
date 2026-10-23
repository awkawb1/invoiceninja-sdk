use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Results of `POST /search`.
///
/// Which buckets are populated depends on how the server is configured: with
/// Elasticsearch enabled the controller fills every entity bucket, and
/// without it only `clients`, `client_contacts`, `invoices`, `projects`, and
/// `settings` are returned. Every field therefore defaults to empty rather
/// than being required, so both shapes decode.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchResults {
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub clients: Vec<SearchHit>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub client_contacts: Vec<SearchHit>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub invoices: Vec<SearchHit>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub quotes: Vec<SearchHit>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub credits: Vec<SearchHit>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub recurring_invoices: Vec<SearchHit>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub purchase_orders: Vec<SearchHit>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub expenses: Vec<SearchHit>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub vendors: Vec<SearchHit>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub vendor_contacts: Vec<SearchHit>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub projects: Vec<SearchHit>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub tasks: Vec<SearchHit>,
    /// Matching settings pages rather than records.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub settings: Vec<SearchHit>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// One search result.
///
/// Every bucket uses the same row shape, so this is not entity-specific —
/// use `id` to fetch the full record through the matching `get_*` method.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchHit {
    /// Hashed id of the matched record.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub name: String,
    /// Entity kind, as a leading-slash tag like `/client` or `/invoice`.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub r#type: String,
    /// Front-end route for the record, e.g. `/clients/Wpmbk5ezJn`.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub path: String,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
