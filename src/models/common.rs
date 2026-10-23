use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct ListResponse<T> {
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
}
