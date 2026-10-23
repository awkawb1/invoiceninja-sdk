use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The static lookup tables returned by `GET /statics`.
///
/// This is a large, near-constant payload — cache it rather than fetching it
/// per request.
///
/// Only the tables with a stable, uniform row shape are typed. `gateways`,
/// `templates`, and `bulk_updates` are left as `Value`: they are nested
/// configuration blobs rather than lookup lists, and the same reasoning
/// applies as for [`Company::settings`](super::Company). Anything not named
/// here — including `einvoice_schema` and the self-host-only `license_key` —
/// is preserved in `extra`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Statics {
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub currencies: Vec<Currency>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub countries: Vec<StaticItem>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub industries: Vec<StaticItem>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub languages: Vec<StaticItem>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub payment_types: Vec<StaticItem>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub sizes: Vec<StaticItem>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub timezones: Vec<StaticItem>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub date_formats: Vec<DateFormat>,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub datetime_formats: Vec<DateFormat>,

    /// Payment gateway definitions. A nested configuration blob, left
    /// untyped.
    #[serde(default)]
    pub gateways: Value,
    /// Email/PDF template defaults. Left untyped for the same reason.
    #[serde(default)]
    pub templates: Value,
    /// Columns that each entity accepts in a bulk update, keyed by entity.
    #[serde(default)]
    pub bulk_updates: Value,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A row in one of the simple lookup tables.
///
/// `name` arrives already translated into the requesting user's locale.
/// Per-table extras — a country's ISO codes, a language's locale, a
/// timezone's UTC offset — stay in `extra`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StaticItem {
    /// Normalised to a string: most tables return a number here, but
    /// `payment_types` returns a string.
    #[serde(default, deserialize_with = "super::common::id_as_string")]
    pub id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub name: String,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A currency, which carries formatting detail the other lookup tables do
/// not.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Currency {
    #[serde(default, deserialize_with = "super::common::id_as_string")]
    pub id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub name: String,
    /// ISO 4217 code, e.g. `USD`.
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub code: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub symbol: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub thousand_separator: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub decimal_separator: String,
    /// Number of decimal places. `Option` rather than defaulted, since 0 is a
    /// meaningful precision (JPY) and must not be confused with "absent".
    #[serde(default)]
    pub precision: Option<i64>,
    #[serde(default)]
    pub swap_currency_symbol: Option<bool>,
    #[serde(default)]
    pub exchange_rate: Option<f64>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A date or datetime format option.
///
/// `format` is the PHP format string; the `format_moment` and `format_dart`
/// variants for other client libraries stay in `extra`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DateFormat {
    #[serde(default, deserialize_with = "super::common::id_as_string")]
    pub id: String,
    #[serde(default, deserialize_with = "super::common::null_to_default")]
    pub format: String,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
