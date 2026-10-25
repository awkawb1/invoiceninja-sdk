use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub struct ListResponse<T> {
    /// Defaulted because not every list-shaped response carries a `data` key:
    /// a bulk action whose `ids` match nothing returns HTTP 200 with just
    /// `{"message": "No Invoices Found"}`, which should read as "no results"
    /// rather than a decode failure.
    #[serde(default = "Vec::new")]
    pub data: Vec<T>,
    #[serde(default)]
    pub meta: Option<Meta>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemResponse<T> {
    pub data: T,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Meta {
    #[serde(default)]
    pub pagination: Option<Pagination>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pagination {
    pub total: u64,
    pub count: u64,
    pub per_page: u64,
    pub current_page: u64,
    pub total_pages: u64,
}

/// Query parameters accepted by Invoice Ninja's list endpoints.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ListParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    /// Comma-separated list of relationships to eager-load, e.g. "client,payments".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<String>,
    /// Set to 1 to include archived/deleted records.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Extra query parameters appended verbatim.
    ///
    /// The typed fields above are only the parameters common to every list
    /// endpoint; individual resources accept many more (`client_id`,
    /// `vendor_id`, `number`, `created_at`, `is_deleted`, per-resource
    /// filters). Use [`param`](Self::param) to set those.
    ///
    /// Skipped by serde and applied as a separate query pass, because
    /// `serde_urlencoded` — which backs `reqwest`'s query serialization —
    /// does not handle `#[serde(flatten)]`ed maps.
    #[serde(skip)]
    pub custom: Vec<(String, String)>,
}

impl ListParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn page(mut self, page: u64) -> Self {
        self.page = Some(page);
        self
    }

    pub fn per_page(mut self, per_page: u64) -> Self {
        self.per_page = Some(per_page);
        self
    }

    pub fn filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = Some(filter.into());
        self
    }

    pub fn sort(mut self, sort: impl Into<String>) -> Self {
        self.sort = Some(sort.into());
        self
    }

    pub fn include(mut self, include: impl Into<String>) -> Self {
        self.include = Some(include.into());
        self
    }

    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Adds an arbitrary query parameter, for filters the typed setters do
    /// not cover.
    ///
    /// ```
    /// use invoiceninjars::models::ListParams;
    ///
    /// let params = ListParams::new()
    ///     .per_page(100)
    ///     .param("client_id", "abc123")
    ///     .param("is_deleted", "false");
    /// ```
    ///
    /// Can be called more than once with the same key; both pairs are sent.
    pub fn param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom.push((key.into(), value.into()));
        self
    }
}

/// Deserializes `null` as `T::default()` instead of failing.
///
/// `#[serde(default)]` alone only covers a *missing* key — a key that is
/// present with an explicit `null` still errors for non-`Option` types.
/// Invoice Ninja returns `null` for plenty of nominally-string fields, so the
/// models pair `default` with this to tolerate both shapes.
pub(crate) fn null_to_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

/// Deserializes an id that may arrive as either a JSON string or a number.
///
/// The `/statics` tables are inconsistent about this: `payment_types` casts
/// its id to a string server-side while the other lookup tables leave theirs
/// as integers. Rather than guess per table — where a wrong choice is a hard
/// decode error that breaks the whole response — both forms are accepted and
/// normalised to `String`.
pub(crate) fn id_as_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::<Value>::deserialize(deserializer)? {
        None | Some(Value::Null) => Ok(String::new()),
        Some(Value::String(s)) => Ok(s),
        Some(Value::Number(n)) => Ok(n.to_string()),
        Some(other) => Err(serde::de::Error::custom(format!(
            "expected a string or number id, got {other}"
        ))),
    }
}

/// Deserializes an integer that may arrive as a JSON number or as a numeric
/// string.
///
/// The OpenAPI spec calls `frequency_id` a number on recurring invoices and
/// expenses, but a real 5.13.50 server returns the string `"5"`. A strict
/// `i64` made that a hard decode error *after* the record had already been
/// created. A string that is not a whole number is still an error.
pub(crate) fn int_or_numeric_string<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::<Value>::deserialize(deserializer)? {
        None | Some(Value::Null) => Ok(0),
        Some(Value::Number(n)) => n
            .as_i64()
            .ok_or_else(|| serde::de::Error::custom(format!("expected a whole number, got {n}"))),
        Some(Value::String(s)) if s.is_empty() => Ok(0),
        Some(Value::String(s)) => s.trim().parse().map_err(|_| {
            serde::de::Error::custom(format!("expected a whole number, got string `{s}`"))
        }),
        Some(other) => Err(serde::de::Error::custom(format!(
            "expected a whole number or numeric string, got {other}"
        ))),
    }
}

/// Body for a bulk action (`POST /{resource}/bulk`).
///
/// The action set varies by resource; `archive`, `restore`, and `delete` work
/// everywhere, and are what the generated `bulk_*` methods are typed for.
/// See [`crate::InvoiceNinjaClient::bulk_invoices`] for the actions whose
/// responses are *not* a list of entities.
#[derive(Debug, Clone, Serialize)]
pub struct BulkRequest {
    pub action: String,
    pub ids: Vec<String>,
}

impl BulkRequest {
    pub fn new(action: impl Into<String>, ids: &[impl AsRef<str>]) -> Self {
        Self {
            action: action.into(),
            ids: ids.iter().map(|id| id.as_ref().to_string()).collect(),
        }
    }
}

/// The `{"message": "..."}` body that action endpoints answer with.
///
/// `message` is defaulted so a reply that carries none still decodes.
/// Anything else the server adds is kept in `extra`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MessageResponse {
    #[serde(default, deserialize_with = "null_to_default")]
    pub message: String,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}
