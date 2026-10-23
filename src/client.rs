use std::time::Duration;

use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use reqwest::{header, Method, StatusCode};
use serde::{de::DeserializeOwned, Serialize};

use crate::error::{ApiErrorBody, Error, Result};
use crate::models::{
    BankIntegration, BankTransaction, BankTransactionRule, BulkRequest, Client as ClientModel,
    ClientGatewayToken, Company, CompanyGateway, CompanyToken, Credit, Design, Document, Expense,
    ExpenseCategory, GroupSetting, HealthCheck, Invoice, ItemResponse, ListParams, ListResponse,
    Location, Payment, PaymentTerm, Product, Project, PurchaseOrder, Quote, RecurringExpense,
    RecurringInvoice, RecurringQuote, SearchResults, Statics, Subscription, SystemLog, Tag, Task,
    TaskScheduler, TaskStatus, TaxRate, User, Vendor, Webhook,
};
use crate::retry::{self, RetryPolicy};

/// API root of the hosted Invoice Ninja app.
pub const HOSTED_BASE_URL: &str = "https://invoicing.co/api/v1";

/// Deadline applied to every request unless overridden with
/// [`InvoiceNinjaClient::with_timeout`].
///
/// `reqwest` applies no timeout of its own, so without this a stalled
/// connection hangs the caller indefinitely. Generous enough for PDF renders
/// and report exports, which are the slowest responses the API produces.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

/// Separate, shorter deadline for establishing the connection, so an
/// unreachable host fails fast rather than consuming the full request budget.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Characters escaped when interpolating a value into a request path: every
/// non-alphanumeric except the RFC 3986 unreserved punctuation, none of which
/// can change a URL's structure.
const PATH_SEGMENT: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'_').remove(b'~');

/// Escapes one path segment, rejecting values that cannot safely be one.
///
/// Every id and action reaching a `format!` that builds a request path goes
/// through this. Ids are caller-supplied and often come from search results
/// or user input, so an unescaped `/`, `?`, or `#` would change which
/// endpoint is called rather than failing cleanly.
///
/// Escaping alone is not sufficient, which is why this returns a `Result`.
/// The `url` crate implements the WHATWG rules, and those decode `%2E` back
/// to `.` *before* removing dot segments — so even a correctly escaped `..`
/// collapses the path (`clients/%2E%2E` resolves to `/`). Values that are
/// empty, or made only of dots, are therefore refused rather than encoded:
/// an empty id would address the collection route instead of a record, and a
/// dot-only id would walk up the path. Invoice Ninja ids are hashids, so
/// nothing legitimate is rejected.
///
/// This is for single segments only — `report` and `chart` take a
/// multi-segment tail by design and are left unescaped.
fn seg(value: &str) -> Result<impl std::fmt::Display + '_> {
    if value.is_empty() || value.chars().all(|c| c == '.') {
        return Err(Error::InvalidPathSegment(value.to_string()));
    }
    Ok(utf8_percent_encode(value, PATH_SEGMENT))
}

/// An async client for the Invoice Ninja v5 API.
///
/// `base_url` should point at the API root — note the `/api/v1` suffix, e.g.
/// `https://invoicing.co/api/v1` for the hosted app (or just use
/// [`hosted`](Self::hosted)), or `https://your-domain.example/api/v1` for a
/// self-hosted install.
#[derive(Debug, Clone)]
pub struct InvoiceNinjaClient {
    http: reqwest::Client,
    base_url: String,
    /// Marked sensitive, so it renders as `Sensitive` rather than the value
    /// in the derived `Debug`.
    api_password: Option<header::HeaderValue>,
    timeout: Duration,
    retry: RetryPolicy,
}

impl InvoiceNinjaClient {
    pub fn new(base_url: impl Into<String>, api_token: impl Into<String>) -> Result<Self> {
        let base_url = base_url.into();
        let trimmed = base_url.trim();
        if trimmed.is_empty() {
            return Err(Error::InvalidBaseUrl(base_url));
        }

        // Parse up front rather than letting a malformed base URL surface as a
        // confusing per-request failure. A missing scheme is the common typo
        // (`invoicing.co/api/v1`), and it fails here instead of on every call.
        let parsed = reqwest::Url::parse(trimmed)
            .map_err(|e| Error::InvalidBaseUrl(format!("{trimmed}: {e}")))?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(Error::InvalidBaseUrl(format!(
                "{trimmed}: expected an http or https url, got scheme `{}`",
                parsed.scheme()
            )));
        }

        let base_url = trimmed.trim_end_matches('/').to_string();

        let mut headers = header::HeaderMap::new();
        let mut token_value = header::HeaderValue::from_str(&api_token.into())
            .map_err(|e| Error::InvalidToken(e.to_string()))?;
        token_value.set_sensitive(true);
        headers.insert("X-Api-Token", token_value);
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            "X-Requested-With",
            header::HeaderValue::from_static("XMLHttpRequest"),
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .connect_timeout(CONNECT_TIMEOUT)
            .build()?;

        Ok(Self {
            http,
            base_url,
            api_password: None,
            timeout: DEFAULT_TIMEOUT,
            retry: RetryPolicy::default(),
        })
    }

    /// Replaces the retry policy, which defaults to
    /// [`RetryPolicy::default`] (3 retries with full-jitter backoff).
    ///
    /// Pass [`RetryPolicy::none`] to attempt every request exactly once.
    pub fn with_retry_policy(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    /// Overrides the per-request deadline, which defaults to
    /// [`DEFAULT_TIMEOUT`].
    ///
    /// Worth raising when pulling large report exports or paging through big
    /// result sets with `list_all_*`, where a single call can legitimately
    /// run long.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Attaches an `X-API-PASSWORD` header to every subsequent request.
    ///
    /// A number of endpoints sit behind the API's `password_protected`
    /// middleware and reject requests without the account password, even
    /// when the token is valid: reading or writing an individual user,
    /// `/users/bulk`, purging a client or company, and merging clients.
    /// Without this, those calls fail rather than returning data.
    ///
    /// ```no_run
    /// # fn main() -> invoiceninjars::Result<()> {
    /// use invoiceninjars::InvoiceNinjaClient;
    ///
    /// let client = InvoiceNinjaClient::hosted(std::env::var("IN_API_TOKEN").unwrap())?
    ///     .with_api_password(std::env::var("IN_API_PASSWORD").unwrap())?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_api_password(mut self, password: impl Into<String>) -> Result<Self> {
        let mut value = header::HeaderValue::from_str(&password.into())
            .map_err(|e| Error::InvalidToken(e.to_string()))?;
        value.set_sensitive(true);
        self.api_password = Some(value);
        Ok(self)
    }

    /// Single place every request is built, so the optional password header
    /// is attached exactly once.
    fn req(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        let builder = self
            .http
            .request(method, self.url(path))
            .timeout(self.timeout);
        match &self.api_password {
            Some(password) => builder.header("X-API-PASSWORD", password.clone()),
            None => builder,
        }
    }

    /// Dispatches a built request, retrying transient failures per the
    /// configured [`RetryPolicy`].
    ///
    /// Every helper goes through here, so retrying is uniform. The request is
    /// cloned before each attempt; a body that cannot be cloned (none of ours
    /// today, since all payloads are in-memory JSON) simply gets one attempt.
    ///
    /// Note what is *not* retried: a `POST` that fails with a 5xx or times
    /// out is surfaced, because the server may already have applied it and a
    /// replay would duplicate an invoice, payment, or email. See
    /// [`crate::retry`] for the full rules.
    async fn execute(&self, builder: reqwest::RequestBuilder) -> Result<reqwest::Response> {
        let mut request = builder.build()?;
        let method = request.method().clone();
        let mut attempt = 0u32;

        loop {
            let replay = if attempt < self.retry.max_retries {
                request.try_clone()
            } else {
                None
            };

            let delay = match self.http.execute(request).await {
                Ok(response) => {
                    let Some(next) =
                        replay.filter(|_| retry::status_is_retryable(response.status(), &method))
                    else {
                        return Ok(response);
                    };
                    // A server-supplied Retry-After wins over our backoff,
                    // but one longer than we are willing to wait is returned
                    // as the error instead of slept through.
                    let delay = retry::retry_after(response.headers())
                        .unwrap_or_else(|| self.retry.backoff(attempt));
                    if delay > self.retry.max_delay {
                        return Ok(response);
                    }
                    request = next;
                    delay
                }
                Err(e) => {
                    let Some(next) =
                        replay.filter(|_| retry::transport_error_is_retryable(&e, &method))
                    else {
                        return Err(e.into());
                    };
                    request = next;
                    self.retry.backoff(attempt)
                }
            };

            tokio::time::sleep(delay).await;
            attempt += 1;
        }
    }

    /// Builds a client pointed at the hosted app at `https://invoicing.co`.
    ///
    /// Equivalent to [`new`](Self::new) with [`HOSTED_BASE_URL`]. Prefer this
    /// over passing the URL by hand: the base URL must include the `/api/v1`
    /// prefix, and a bare host silently 404s every request.
    pub fn hosted(api_token: impl Into<String>) -> Result<Self> {
        Self::new(HOSTED_BASE_URL, api_token)
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    async fn decode<T: DeserializeOwned>(response: reqwest::Response) -> Result<T> {
        let status = response.status();
        let bytes = response.bytes().await?;

        if status.is_success() {
            Ok(serde_json::from_slice(&bytes)?)
        } else {
            Err(Self::api_error(status, &bytes))
        }
    }

    fn api_error(status: StatusCode, bytes: &[u8]) -> Error {
        match serde_json::from_slice::<ApiErrorBody>(bytes) {
            Ok(body) => Error::Api {
                status,
                message: body.message,
                errors: body.errors,
            },
            Err(_) => Error::Api {
                status,
                message: String::from_utf8_lossy(bytes).into_owned(),
                errors: Default::default(),
            },
        }
    }

    async fn get_list<T: DeserializeOwned>(
        &self,
        path: &str,
        params: &ListParams,
    ) -> Result<Vec<T>> {
        let response = self
            .execute(
                self.req(Method::GET, path)
                    .query(params)
                    .query(&params.custom),
            )
            .await?;
        let body: ListResponse<T> = Self::decode(response).await?;
        Ok(body.data)
    }

    /// Pages through a list endpoint until the paginator reports the last
    /// page, falling back to a short-page check when no metadata is sent.
    ///
    /// Termination prefers `meta.pagination.total_pages`, which is
    /// authoritative. The short-page heuristic alone is not: the server caps
    /// page size (see [`MAX_PAGE_SIZE`]), so a page can be shorter than
    /// requested while more records remain.
    async fn get_all_pages<T: DeserializeOwned>(
        &self,
        path: &str,
        params: &ListParams,
    ) -> Result<Vec<T>> {
        /// Page size used when the caller does not pick one.
        const DEFAULT_PAGE_SIZE: u64 = 50;

        /// Server-side ceiling on `per_page`: Invoice Ninja's `BaseController`
        /// applies `min(abs(per_page), 5000)`.
        ///
        /// Requesting more than this comes back capped, so the request is
        /// clamped to match. Otherwise the short-page fallback below would
        /// read the very first (capped, therefore "short") page as the last
        /// one and silently drop every record after it.
        const MAX_PAGE_SIZE: u64 = 5000;

        /// Backstop for a server that never signals the end. At the default
        /// page size this is well past any real result set, so reaching it
        /// means the pagination contract is broken rather than the data being
        /// large.
        const MAX_PAGES: u64 = 10_000;

        // `per_page` of 0 would make the short-page check `got < 0`, which no
        // page length can satisfy — the loop would never terminate.
        let per_page = match params.per_page {
            Some(0) | None => DEFAULT_PAGE_SIZE,
            Some(n) => n.min(MAX_PAGE_SIZE),
        };
        let mut page = params.page.unwrap_or(1).max(1);
        let mut params = params.clone();
        params.per_page = Some(per_page);

        let mut all = Vec::new();
        let mut fetched = 0u64;
        loop {
            params.page = Some(page);
            let response = self
                .execute(
                    self.req(Method::GET, path)
                        .query(&params)
                        .query(&params.custom),
                )
                .await?;
            let body: ListResponse<T> = Self::decode(response).await?;
            let got = body.data.len() as u64;
            let total_pages = body.meta.and_then(|m| m.pagination).map(|p| p.total_pages);
            all.extend(body.data);
            fetched += 1;

            // An empty page ends the walk whatever the metadata claims, so a
            // bogus `total_pages` cannot spin on a resource that has run out.
            if got == 0 {
                break;
            }

            let more = match total_pages {
                Some(total) if total > 0 => page < total,
                // No usable metadata: fall back to treating a short page as
                // the last. Sound here because `per_page` is clamped to what
                // the server will actually honour.
                _ => got >= per_page,
            };
            if !more {
                break;
            }

            if fetched >= MAX_PAGES {
                return Err(Error::PaginationLimit {
                    path: path.to_string(),
                    pages: fetched,
                });
            }

            // Advance our own counter rather than trusting `current_page`, so
            // a server that echoes a fixed page number cannot loop forever.
            page += 1;
        }
        Ok(all)
    }

    async fn get_one<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let response = self.execute(self.req(Method::GET, path)).await?;
        let body: ItemResponse<T> = Self::decode(response).await?;
        Ok(body.data)
    }

    async fn post_one<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        payload: &B,
    ) -> Result<T> {
        let response = self
            .execute(self.req(Method::POST, path).json(payload))
            .await?;
        let body: ItemResponse<T> = Self::decode(response).await?;
        Ok(body.data)
    }

    async fn put_one<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        payload: &B,
    ) -> Result<T> {
        let response = self
            .execute(self.req(Method::PUT, path).json(payload))
            .await?;
        let body: ItemResponse<T> = Self::decode(response).await?;
        Ok(body.data)
    }

    async fn delete_one(&self, path: &str) -> Result<()> {
        let response = self.execute(self.req(Method::DELETE, path)).await?;
        let status = response.status();
        let bytes = response.bytes().await?;
        if status.is_success() {
            Ok(())
        } else {
            Err(Self::api_error(status, &bytes))
        }
    }

    async fn post_list<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        payload: &B,
    ) -> Result<Vec<T>> {
        let response = self
            .execute(self.req(Method::POST, path).json(payload))
            .await?;
        let body: ListResponse<T> = Self::decode(response).await?;
        Ok(body.data)
    }

    /// Returns the raw response body. Used for endpoints that do not answer
    /// with JSON — PDF streams (`application/pdf`) and CSV report exports.
    async fn get_raw(&self, path: &str) -> Result<Vec<u8>> {
        let response = self.execute(self.req(Method::GET, path)).await?;
        let status = response.status();
        let bytes = response.bytes().await?;
        if status.is_success() {
            Ok(bytes.to_vec())
        } else {
            Err(Self::api_error(status, &bytes))
        }
    }

    async fn post_raw<B: Serialize>(&self, path: &str, payload: &B) -> Result<Vec<u8>> {
        let response = self
            .execute(self.req(Method::POST, path).json(payload))
            .await?;
        let status = response.status();
        let bytes = response.bytes().await?;
        if status.is_success() {
            Ok(bytes.to_vec())
        } else {
            Err(Self::api_error(status, &bytes))
        }
    }

    /// Decodes a response that has no envelope and no stable schema
    /// (`/statics`, `/charts/*`, `/search`).
    async fn get_unwrapped<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let response = self.execute(self.req(Method::GET, path)).await?;
        Self::decode(response).await
    }

    async fn post_unwrapped<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        payload: &B,
    ) -> Result<T> {
        let response = self
            .execute(self.req(Method::POST, path).json(payload))
            .await?;
        Self::decode(response).await
    }

    /// Issues a request whose body is discarded, failing on non-2xx.
    async fn get_discard(&self, path: &str) -> Result<()> {
        self.get_raw(path).await.map(|_| ())
    }
}

/// Generates typed list/list_all/get/create/update/delete methods for a
/// resource backed by a single Invoice Ninja REST endpoint (e.g. `/clients`).
///
/// Most resources should use [`resource_methods!`] instead, which adds
/// `bulk`. This bare form is for the handful of `Route::resource` groups
/// that have no `/bulk` route — `client_gateway_tokens`, `system_logs`, and
/// `companies` — where a `bulk_*` method would just 405.
macro_rules! crud_methods {
    ($list:ident, $list_all:ident, $get:ident, $create:ident, $update:ident, $delete:ident, $path:literal, $model:ty) => {
        pub async fn $list(&self, params: &ListParams) -> Result<Vec<$model>> {
            self.get_list($path, params).await
        }

        /// Like the paginated list method, but automatically follows
        /// pagination and returns every matching record across all pages.
        pub async fn $list_all(&self, params: &ListParams) -> Result<Vec<$model>> {
            self.get_all_pages($path, params).await
        }

        pub async fn $get(&self, id: &str) -> Result<$model> {
            self.get_one(&format!("{}/{}", $path, seg(id)?)).await
        }

        pub async fn $create(&self, body: &$model) -> Result<$model> {
            self.post_one($path, body).await
        }

        pub async fn $update(&self, id: &str, body: &$model) -> Result<$model> {
            self.put_one(&format!("{}/{}", $path, seg(id)?), body).await
        }

        pub async fn $delete(&self, id: &str) -> Result<()> {
            self.delete_one(&format!("{}/{}", $path, seg(id)?)).await
        }
    };
}

/// Generates everything [`crud_methods!`] does, plus `bulk`.
macro_rules! resource_methods {
    ($list:ident, $list_all:ident, $get:ident, $create:ident, $update:ident, $delete:ident, $bulk:ident, $path:literal, $model:ty) => {
        crud_methods!($list, $list_all, $get, $create, $update, $delete, $path, $model);

        /// Applies `action` to many records at once and returns the affected
        /// records. `archive`, `restore`, and `delete` are available on every
        /// resource that has a `/bulk` route.
        ///
        /// A few actions answer with something other than a list of entities
        /// and so cannot be used here — see
        /// [`download_invoices`](Self::download_invoices) for the PDF ones.
        /// An `ids` list that matches nothing is not an error; it comes back
        /// as an empty `Vec`.
        pub async fn $bulk(&self, action: &str, ids: &[impl AsRef<str>]) -> Result<Vec<$model>> {
            self.post_list(&format!("{}/bulk", $path), &BulkRequest::new(action, ids))
                .await
        }
    };
}

/// Generates the `GET /{resource}/{id}/{action}` helpers. Only invoices,
/// quotes, credits, and purchase orders expose this route.
macro_rules! entity_action_methods {
    ($action:ident, $download:ident, $download_many:ident, $path:literal, $model:ty) => {
        /// Performs a single-entity action and returns the updated record.
        ///
        /// Actions that return the entity: `auto_bill`, `clone_to_invoice`,
        /// `clone_to_quote`, `mark_paid`, `mark_sent`, `archive`, `restore`,
        /// `delete`.
        ///
        /// Three actions on this route return something else and will fail to
        /// decode here: `download` (a PDF — use the `download_*` method),
        /// `convert_to_purchase_order` (a purchase order — use
        /// [`convert_invoice_to_purchase_order`](Self::convert_invoice_to_purchase_order)),
        /// and `history` / `delivery_note` / `cancel`, which upstream answers
        /// with an empty body.
        pub async fn $action(&self, id: &str, action: &str) -> Result<$model> {
            self.get_one(&format!("{}/{}/{}", $path, seg(id)?, seg(action)?))
                .await
        }

        /// Downloads the rendered PDF bytes for one record.
        pub async fn $download(&self, id: &str) -> Result<Vec<u8>> {
            self.get_raw(&format!("{}/{}/download", $path, seg(id)?))
                .await
        }

        /// Downloads one PDF covering every listed record.
        ///
        /// This is the bulk `download` action, which streams a merged PDF
        /// rather than returning JSON, so it cannot go through the `bulk_*`
        /// method for this resource.
        pub async fn $download_many(&self, ids: &[impl AsRef<str>]) -> Result<Vec<u8>> {
            self.post_raw(
                &format!("{}/bulk", $path),
                &BulkRequest::new("download", ids),
            )
            .await
        }
    };
}

impl InvoiceNinjaClient {
    resource_methods!(
        list_clients,
        list_all_clients,
        get_client,
        create_client,
        update_client,
        delete_client,
        bulk_clients,
        "clients",
        ClientModel
    );

    resource_methods!(
        list_invoices,
        list_all_invoices,
        get_invoice,
        create_invoice,
        update_invoice,
        delete_invoice,
        bulk_invoices,
        "invoices",
        Invoice
    );

    resource_methods!(
        list_quotes,
        list_all_quotes,
        get_quote,
        create_quote,
        update_quote,
        delete_quote,
        bulk_quotes,
        "quotes",
        Quote
    );

    resource_methods!(
        list_credits,
        list_all_credits,
        get_credit,
        create_credit,
        update_credit,
        delete_credit,
        bulk_credits,
        "credits",
        Credit
    );

    resource_methods!(
        list_payments,
        list_all_payments,
        get_payment,
        create_payment,
        update_payment,
        delete_payment,
        bulk_payments,
        "payments",
        Payment
    );

    resource_methods!(
        list_products,
        list_all_products,
        get_product,
        create_product,
        update_product,
        delete_product,
        bulk_products,
        "products",
        Product
    );

    resource_methods!(
        list_expenses,
        list_all_expenses,
        get_expense,
        create_expense,
        update_expense,
        delete_expense,
        bulk_expenses,
        "expenses",
        Expense
    );

    resource_methods!(
        list_projects,
        list_all_projects,
        get_project,
        create_project,
        update_project,
        delete_project,
        bulk_projects,
        "projects",
        Project
    );

    resource_methods!(
        list_tasks,
        list_all_tasks,
        get_task,
        create_task,
        update_task,
        delete_task,
        bulk_tasks,
        "tasks",
        Task
    );

    resource_methods!(
        list_vendors,
        list_all_vendors,
        get_vendor,
        create_vendor,
        update_vendor,
        delete_vendor,
        bulk_vendors,
        "vendors",
        Vendor
    );

    resource_methods!(
        list_recurring_invoices,
        list_all_recurring_invoices,
        get_recurring_invoice,
        create_recurring_invoice,
        update_recurring_invoice,
        delete_recurring_invoice,
        bulk_recurring_invoices,
        "recurring_invoices",
        RecurringInvoice
    );

    resource_methods!(
        list_recurring_quotes,
        list_all_recurring_quotes,
        get_recurring_quote,
        create_recurring_quote,
        update_recurring_quote,
        delete_recurring_quote,
        bulk_recurring_quotes,
        "recurring_quotes",
        RecurringQuote
    );

    resource_methods!(
        list_recurring_expenses,
        list_all_recurring_expenses,
        get_recurring_expense,
        create_recurring_expense,
        update_recurring_expense,
        delete_recurring_expense,
        bulk_recurring_expenses,
        "recurring_expenses",
        RecurringExpense
    );

    resource_methods!(
        list_purchase_orders,
        list_all_purchase_orders,
        get_purchase_order,
        create_purchase_order,
        update_purchase_order,
        delete_purchase_order,
        bulk_purchase_orders,
        "purchase_orders",
        PurchaseOrder
    );

    resource_methods!(
        list_tax_rates,
        list_all_tax_rates,
        get_tax_rate,
        create_tax_rate,
        update_tax_rate,
        delete_tax_rate,
        bulk_tax_rates,
        "tax_rates",
        TaxRate
    );

    resource_methods!(
        list_expense_categories,
        list_all_expense_categories,
        get_expense_category,
        create_expense_category,
        update_expense_category,
        delete_expense_category,
        bulk_expense_categories,
        "expense_categories",
        ExpenseCategory
    );

    resource_methods!(
        list_task_statuses,
        list_all_task_statuses,
        get_task_status,
        create_task_status,
        update_task_status,
        delete_task_status,
        bulk_task_statuses,
        "task_statuses",
        TaskStatus
    );

    resource_methods!(
        list_payment_terms,
        list_all_payment_terms,
        get_payment_term,
        create_payment_term,
        update_payment_term,
        delete_payment_term,
        bulk_payment_terms,
        "payment_terms",
        PaymentTerm
    );

    resource_methods!(
        list_subscriptions,
        list_all_subscriptions,
        get_subscription,
        create_subscription,
        update_subscription,
        delete_subscription,
        bulk_subscriptions,
        "subscriptions",
        Subscription
    );

    resource_methods!(
        list_bank_transactions,
        list_all_bank_transactions,
        get_bank_transaction,
        create_bank_transaction,
        update_bank_transaction,
        delete_bank_transaction,
        bulk_bank_transactions,
        "bank_transactions",
        BankTransaction
    );

    resource_methods!(
        list_webhooks,
        list_all_webhooks,
        get_webhook,
        create_webhook,
        update_webhook,
        delete_webhook,
        bulk_webhooks,
        "webhooks",
        Webhook
    );

    entity_action_methods!(
        invoice_action,
        download_invoice,
        download_invoices,
        "invoices",
        Invoice
    );

    entity_action_methods!(
        quote_action,
        download_quote,
        download_quotes,
        "quotes",
        Quote
    );

    entity_action_methods!(
        credit_action,
        download_credit,
        download_credits,
        "credits",
        Credit
    );

    entity_action_methods!(
        purchase_order_action,
        download_purchase_order,
        download_purchase_orders,
        "purchase_orders",
        PurchaseOrder
    );
}

/// Documents, reports, and the remaining endpoints that do not follow the
/// per-resource CRUD shape.
impl InvoiceNinjaClient {
    // --- documents ------------------------------------------------------

    pub async fn list_documents(&self, params: &ListParams) -> Result<Vec<Document>> {
        self.get_list("documents", params).await
    }

    pub async fn list_all_documents(&self, params: &ListParams) -> Result<Vec<Document>> {
        self.get_all_pages("documents", params).await
    }

    pub async fn get_document(&self, id: &str) -> Result<Document> {
        self.get_one(&format!("documents/{}", seg(id)?)).await
    }

    pub async fn update_document(&self, id: &str, body: &Document) -> Result<Document> {
        self.put_one(&format!("documents/{}", seg(id)?), body).await
    }

    pub async fn delete_document(&self, id: &str) -> Result<()> {
        self.delete_one(&format!("documents/{}", seg(id)?)).await
    }

    /// Downloads a document's raw bytes.
    pub async fn download_document(&self, id: &str) -> Result<Vec<u8>> {
        self.get_raw(&format!("documents/{}/download", seg(id)?))
            .await
    }

    pub async fn bulk_documents(
        &self,
        action: &str,
        ids: &[impl AsRef<str>],
    ) -> Result<Vec<Document>> {
        self.post_list("documents/bulk", &BulkRequest::new(action, ids))
            .await
    }

    // --- entity actions with non-entity responses -----------------------

    /// Converts an invoice into a purchase order. Separate from
    /// [`invoice_action`](Self::invoice_action) because the response is a
    /// purchase order rather than an invoice.
    pub async fn convert_invoice_to_purchase_order(&self, id: &str) -> Result<PurchaseOrder> {
        self.get_one(&format!("invoices/{}/convert_to_purchase_order", seg(id)?))
            .await
    }

    /// Marks an invoice as sent, returning the updated invoice.
    pub async fn mark_invoice_sent(&self, id: &str) -> Result<Invoice> {
        self.invoice_action(id, "mark_sent").await
    }

    /// Marks an invoice as paid, returning the updated invoice.
    ///
    /// Upstream rejects this with HTTP 400 if the invoice is already paid or
    /// deleted, which surfaces as [`Error::Api`](crate::Error::Api).
    pub async fn mark_invoice_paid(&self, id: &str) -> Result<Invoice> {
        self.invoice_action(id, "mark_paid").await
    }

    // --- reports --------------------------------------------------------

    /// Runs a report and returns the raw export body, which is CSV for most
    /// reports.
    ///
    /// `name` is the report path segment, e.g. `invoices`, `clients`,
    /// `payments`, `profitloss`, `ar_detail_report`, `tax_summary_report`.
    /// `body` carries the report's filters (commonly `start_date`,
    /// `end_date`, `date_range`, `report_keys`).
    ///
    /// Large exports are queued and emailed instead of returned inline; in
    /// that case the body is a short JSON message rather than CSV.
    pub async fn report<B: Serialize>(&self, name: &str, body: &B) -> Result<Vec<u8>> {
        self.post_raw(&format!("reports/{}", name), body).await
    }

    // --- charts, search, and lookups ------------------------------------

    /// Posts to a `/charts/{name}` endpoint, e.g. `totals_v2`,
    /// `chart_summary_v2`, `calculated_fields`, `cashflow_forecast`.
    ///
    /// Deliberately still generic. Unlike the other endpoints here, the
    /// eleven charts share no common response shape — `totals_v2` answers
    /// with per-currency maps, `cashflow_forecast` with a date series,
    /// `calculated_fields` with scalars — so there is nothing to narrow to.
    /// Pass `serde_json::Value`, or your own struct for the chart you are
    /// calling.
    ///
    /// `project_analytics` and `project_burnup` take a project id as a
    /// further path segment: `chart("project_burnup/<id>", &body)`.
    pub async fn chart<B: Serialize, T: DeserializeOwned>(
        &self,
        name: &str,
        body: &B,
    ) -> Result<T> {
        self.post_unwrapped(&format!("charts/{}", name), body).await
    }

    /// Full-text search across entities.
    ///
    /// Which buckets of [`SearchResults`] are filled depends on whether the
    /// server has Elasticsearch enabled; see the type's docs.
    pub async fn search<B: Serialize>(&self, body: &B) -> Result<SearchResults> {
        self.search_as(body).await
    }

    /// [`search`](Self::search) decoded into a caller-chosen type.
    pub async fn search_as<B: Serialize, T: DeserializeOwned>(&self, body: &B) -> Result<T> {
        self.post_unwrapped("search", body).await
    }

    /// Fetches the static lookup tables.
    ///
    /// This is a large and near-constant payload — cache it rather than
    /// fetching per request.
    pub async fn statics(&self) -> Result<Statics> {
        self.statics_as().await
    }

    /// [`statics`](Self::statics) decoded into a caller-chosen type.
    ///
    /// [`Statics`] types the uniform lookup tables and keeps the rest in
    /// `extra`; use this to pin tables it leaves untyped, such as `gateways`
    /// or `templates`.
    pub async fn statics_as<T: DeserializeOwned>(&self) -> Result<T> {
        self.get_unwrapped("statics").await
    }

    /// Queues an email for an entity. The body selects the target, e.g.
    /// `{"entity": "invoice", "entity_id": "...", "template": "email_template_invoice"}`.
    ///
    /// Returns the refreshed entity, so `T` is the type the email was sent
    /// for — [`Invoice`], [`Quote`], [`Credit`], or [`PurchaseOrder`]:
    ///
    /// ```no_run
    /// # async fn run(client: &invoiceninjars::InvoiceNinjaClient) -> invoiceninjars::Result<()> {
    /// use invoiceninjars::models::Invoice;
    /// use serde_json::json;
    ///
    /// let invoice: Invoice = client
    ///     .send_email(&json!({ "entity": "invoice", "entity_id": "Wpmbk5ezJn" }))
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// It stays generic over the entity because the endpoint serves all four,
    /// but the `data` envelope is now unwrapped for you — it previously was
    /// not, so callers had to model the wrapper themselves.
    pub async fn send_email<B: Serialize, T: DeserializeOwned>(&self, body: &B) -> Result<T> {
        self.post_one("emails", body).await
    }

    /// Refunds a payment. The body takes `id`, optionally `amount`,
    /// `invoices`, and `send_email`.
    pub async fn refund_payment<B: Serialize>(&self, body: &B) -> Result<Payment> {
        self.post_one("payments/refund", body).await
    }

    // --- health ---------------------------------------------------------

    /// Cheap authenticated round-trip, useful for validating a token.
    pub async fn ping(&self) -> Result<()> {
        self.get_discard("ping").await
    }

    /// Server-side health check (database, queues, and so on).
    ///
    /// The hosted app does not run the check and answers with a bare
    /// message, so most fields of [`HealthCheck`] are `Option` — see its
    /// docs.
    pub async fn health_check(&self) -> Result<HealthCheck> {
        self.health_check_as().await
    }

    /// [`health_check`](Self::health_check) decoded into a caller-chosen
    /// type.
    pub async fn health_check_as<T: DeserializeOwned>(&self) -> Result<T> {
        self.get_unwrapped("health_check").await
    }
}

/// Resource groups added to match (and go past) the official PHP SDK.
impl InvoiceNinjaClient {
    resource_methods!(
        list_bank_integrations,
        list_all_bank_integrations,
        get_bank_integration,
        create_bank_integration,
        update_bank_integration,
        delete_bank_integration,
        bulk_bank_integrations,
        "bank_integrations",
        BankIntegration
    );

    resource_methods!(
        list_bank_transaction_rules,
        list_all_bank_transaction_rules,
        get_bank_transaction_rule,
        create_bank_transaction_rule,
        update_bank_transaction_rule,
        delete_bank_transaction_rule,
        bulk_bank_transaction_rules,
        "bank_transaction_rules",
        BankTransactionRule
    );

    resource_methods!(
        list_company_gateways,
        list_all_company_gateways,
        get_company_gateway,
        create_company_gateway,
        update_company_gateway,
        delete_company_gateway,
        bulk_company_gateways,
        "company_gateways",
        CompanyGateway
    );

    resource_methods!(
        list_designs,
        list_all_designs,
        get_design,
        create_design,
        update_design,
        delete_design,
        bulk_designs,
        "designs",
        Design
    );

    resource_methods!(
        list_group_settings,
        list_all_group_settings,
        get_group_setting,
        create_group_setting,
        update_group_setting,
        delete_group_setting,
        bulk_group_settings,
        "group_settings",
        GroupSetting
    );

    resource_methods!(
        list_locations,
        list_all_locations,
        get_location,
        create_location,
        update_location,
        delete_location,
        bulk_locations,
        "locations",
        Location
    );

    resource_methods!(
        list_tags,
        list_all_tags,
        get_tag,
        create_tag,
        update_tag,
        delete_tag,
        bulk_tags,
        "tags",
        Tag
    );

    resource_methods!(
        list_task_schedulers,
        list_all_task_schedulers,
        get_task_scheduler,
        create_task_scheduler,
        update_task_scheduler,
        delete_task_scheduler,
        bulk_task_schedulers,
        "task_schedulers",
        TaskScheduler
    );

    resource_methods!(
        list_tokens,
        list_all_tokens,
        get_token,
        create_token,
        update_token,
        delete_token,
        bulk_tokens,
        "tokens",
        CompanyToken
    );

    // --- no /bulk route ---------------------------------------------------

    crud_methods!(
        list_client_gateway_tokens,
        list_all_client_gateway_tokens,
        get_client_gateway_token,
        create_client_gateway_token,
        update_client_gateway_token,
        delete_client_gateway_token,
        "client_gateway_tokens",
        ClientGatewayToken
    );

    crud_methods!(
        list_companies,
        list_all_companies,
        get_company,
        create_company,
        update_company,
        delete_company,
        "companies",
        Company
    );

    crud_methods!(
        list_system_logs,
        list_all_system_logs,
        get_system_log,
        create_system_log,
        update_system_log,
        delete_system_log,
        "system_logs",
        SystemLog
    );
}

/// Users.
///
/// `/users` is not a `Route::resource`, and all of it except the index
/// listing is behind the `password_protected` middleware, so every method
/// here other than [`list_users`](Self::list_users) and
/// [`list_all_users`](Self::list_all_users) needs
/// [`with_api_password`](Self::with_api_password). There is also no plain
/// `DELETE /users/{id}` route — use
/// [`detach_user_from_company`](Self::detach_user_from_company) or the bulk
/// `delete` action.
impl InvoiceNinjaClient {
    pub async fn list_users(&self, params: &ListParams) -> Result<Vec<User>> {
        self.get_list("users", params).await
    }

    pub async fn list_all_users(&self, params: &ListParams) -> Result<Vec<User>> {
        self.get_all_pages("users", params).await
    }

    /// Requires an API password.
    pub async fn get_user(&self, id: &str) -> Result<User> {
        self.get_one(&format!("users/{}", seg(id)?)).await
    }

    /// Requires an API password.
    pub async fn create_user(&self, body: &User) -> Result<User> {
        self.post_one("users", body).await
    }

    /// Requires an API password.
    pub async fn update_user(&self, id: &str, body: &User) -> Result<User> {
        self.put_one(&format!("users/{}", seg(id)?), body).await
    }

    /// Removes a user from the current company. Requires an API password.
    ///
    /// This is the closest thing to deleting a user; the API exposes no
    /// `DELETE /users/{id}`.
    pub async fn detach_user_from_company(&self, id: &str) -> Result<()> {
        self.delete_one(&format!("users/{}/detach_from_company", seg(id)?))
            .await
    }

    /// Requires an API password.
    pub async fn bulk_users(&self, action: &str, ids: &[impl AsRef<str>]) -> Result<Vec<User>> {
        self.post_list("users/bulk", &BulkRequest::new(action, ids))
            .await
    }
}
