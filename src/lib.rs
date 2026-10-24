//! An async Rust client for the [Invoice Ninja](https://www.invoiceninja.com/) v5 API.
//!
//! ```no_run
//! use invoiceninjars::{InvoiceNinjaClient, models::ListParams};
//!
//! # async fn run() -> invoiceninjars::Result<()> {
//! let client = InvoiceNinjaClient::new(
//!     "https://invoicing.co/api/v1",
//!     std::env::var("IN_API_TOKEN").unwrap(),
//! )?;
//!
//! let clients = client.list_clients(&ListParams::new().per_page(50)).await?;
//! # Ok(())
//! # }
//! ```

mod client;
mod error;
pub mod models;
mod retry;
mod upload;

pub use client::{InvoiceNinjaClient, DEFAULT_TIMEOUT, HOSTED_BASE_URL};
pub use error::{ApiErrorBody, Error, Result};
pub use retry::RetryPolicy;
pub use upload::FileUpload;
