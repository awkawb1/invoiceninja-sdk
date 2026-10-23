use reqwest::{header, StatusCode};
use serde::{de::DeserializeOwned, Serialize};

use crate::error::{ApiErrorBody, Error, Result};
use crate::models::{
    Client as ClientModel, Credit, Expense, Invoice, ItemResponse, ListParams, ListResponse,
    Payment, Product, Project, Quote, Task, Vendor,
};

/// An async client for the Invoice Ninja v5 API.
///
/// `base_url` should point at the API root, e.g. `https://invoicing.co/api/v1`
/// for the hosted app, or `https://your-domain.example/api/v1` for a
/// self-hosted install.
#[derive(Debug, Clone)]
pub struct InvoiceNinjaClient {
    http: reqwest::Client,
    base_url: String,
}

impl InvoiceNinjaClient {
    pub fn new(base_url: impl Into<String>, api_token: impl Into<String>) -> Result<Self> {
        let base_url = base_url.into();
        if base_url.trim().is_empty() {
            return Err(Error::InvalidBaseUrl(base_url));
        }
        let base_url = base_url.trim_end_matches('/').to_string();

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
            .build()?;

        Ok(Self { http, base_url })
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
        let response = self.http.get(self.url(path)).query(params).send().await?;
        let body: ListResponse<T> = Self::decode(response).await?;
        Ok(body.data)
    }

    /// Pages through a list endpoint, following `page`/`per_page` until a
    /// page comes back shorter than requested (or empty).
    async fn get_all_pages<T: DeserializeOwned>(
        &self,
        path: &str,
        params: &ListParams,
    ) -> Result<Vec<T>> {
        const DEFAULT_PAGE_SIZE: u64 = 50;

        let per_page = params.per_page.unwrap_or(DEFAULT_PAGE_SIZE);
        let mut page = params.page.unwrap_or(1);
        let mut params = params.clone();
        params.per_page = Some(per_page);

        let mut all = Vec::new();
        loop {
            params.page = Some(page);
            let response = self.http.get(self.url(path)).query(&params).send().await?;
            let body: ListResponse<T> = Self::decode(response).await?;
            let got = body.data.len() as u64;
            all.extend(body.data);

            if got < per_page {
                break;
            }
            page += 1;
        }
        Ok(all)
    }

    async fn get_one<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let response = self.http.get(self.url(path)).send().await?;
        let body: ItemResponse<T> = Self::decode(response).await?;
        Ok(body.data)
    }

    async fn post_one<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        payload: &B,
    ) -> Result<T> {
        let response = self.http.post(self.url(path)).json(payload).send().await?;
        let body: ItemResponse<T> = Self::decode(response).await?;
        Ok(body.data)
    }

    async fn put_one<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        payload: &B,
    ) -> Result<T> {
        let response = self.http.put(self.url(path)).json(payload).send().await?;
        let body: ItemResponse<T> = Self::decode(response).await?;
        Ok(body.data)
    }

    async fn delete_one(&self, path: &str) -> Result<()> {
        let response = self.http.delete(self.url(path)).send().await?;
        let status = response.status();
        let bytes = response.bytes().await?;
        if status.is_success() {
            Ok(())
        } else {
            Err(Self::api_error(status, &bytes))
        }
    }
}

/// Generates typed list/list_all/get/create/update/delete methods for a
/// resource backed by a single Invoice Ninja REST endpoint (e.g. `/clients`).
macro_rules! resource_methods {
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
            self.get_one(&format!("{}/{}", $path, id)).await
        }

        pub async fn $create(&self, body: &$model) -> Result<$model> {
            self.post_one($path, body).await
        }

        pub async fn $update(&self, id: &str, body: &$model) -> Result<$model> {
            self.put_one(&format!("{}/{}", $path, id), body).await
        }

        pub async fn $delete(&self, id: &str) -> Result<()> {
            self.delete_one(&format!("{}/{}", $path, id)).await
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
        "vendors",
        Vendor
    );
}
