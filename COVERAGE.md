# API coverage

What this crate wraps of the Invoice Ninja v5 API, and what it doesn't.

Full coverage is a stated goal, so the gaps below are a to-do list rather
than a permanent boundary — [`GOALS.md`](GOALS.md) puts them in rough
priority order.

Checked against `routes/api.php` on the `v5-stable` branch of the
[invoiceninja repo](https://github.com/invoiceninja/invoiceninja), which
declares 34 `Route::resource` groups and 274 explicit route declarations
(272 single-verb, plus two `Route::match` pairs registering two verbs each).

> Note on sources: Invoice Ninja's published `openapi/api-docs.yaml` is
> reliable for **schemas** but incomplete for **routes** — it documents
> `/{id}` and action paths only for clients, locations, and tags. Use
> `routes/api.php` and the controllers when checking whether an endpoint
> exists or what it returns.

**All 34 `Route::resource` groups are now wrapped**, plus `users` (which is
not a resource group). 284 methods in total.

## Resources with full CRUD + bulk

30 resources, each with `list_*`, `list_all_*`, `get_*`, `create_*`,
`update_*`, `delete_*`, and `bulk_*`:

| Resource | Path | Model |
| --- | --- | --- |
| Clients | `/clients` | `Client` (+ `ClientContact`) |
| Invoices | `/invoices` | `Invoice` (+ `InvoiceItem`) |
| Quotes | `/quotes` | `Quote` (alias of `Invoice`) |
| Credits | `/credits` | `Credit` (alias of `Invoice`) |
| Payments | `/payments` | `Payment` (+ `PaymentInvoice`) |
| Products | `/products` | `Product` |
| Purchase Orders | `/purchase_orders` | `PurchaseOrder` |
| Recurring Invoices | `/recurring_invoices` | `RecurringInvoice` |
| Recurring Quotes | `/recurring_quotes` | `RecurringQuote` (alias of `RecurringInvoice`) |
| Recurring Expenses | `/recurring_expenses` | `RecurringExpense` |
| Subscriptions | `/subscriptions` | `Subscription` |
| Projects | `/projects` | `Project` |
| Tasks | `/tasks` | `Task` |
| Task Statuses | `/task_statuses` | `TaskStatus` |
| Expenses | `/expenses` | `Expense` |
| Expense Categories | `/expense_categories` | `ExpenseCategory` |
| Vendors | `/vendors` | `Vendor` (+ `VendorContact`) |
| Bank Transactions | `/bank_transactions` | `BankTransaction` |
| Tax Rates | `/tax_rates` | `TaxRate` |
| Payment Terms | `/payment_terms` | `PaymentTerm` |
| Webhooks | `/webhooks` | `Webhook` |
| Bank Integrations | `/bank_integrations` | `BankIntegration` |
| Bank Transaction Rules | `/bank_transaction_rules` | `BankTransactionRule` |
| Company Gateways | `/company_gateways` | `CompanyGateway` |
| Designs | `/designs` | `Design` |
| Group Settings | `/group_settings` | `GroupSetting` |
| Locations | `/locations` | `Location` |
| Tags | `/tags` | `Tag` |
| Task Schedulers | `/task_schedulers` | `TaskScheduler` |
| Tokens | `/tokens` | `CompanyToken` |

`bulk_*` accepts any action the endpoint supports; `archive`, `restore`, and
`delete` work on all of them. An `ids` list matching nothing returns an empty
`Vec`, not an error.

## Resources without a bulk route

Three `Route::resource` groups have no `/bulk` route, so they get the six
CRUD methods but no `bulk_*` (which would just 405). These use
`crud_methods!` rather than `resource_methods!`:

| Resource | Path | Model |
| --- | --- | --- |
| Client Gateway Tokens | `/client_gateway_tokens` | `ClientGatewayToken` |
| Companies | `/companies` | `Company` |
| System Logs | `/system_logs` | `SystemLog` |

`Company` and `GroupSetting` keep their `settings` blob as
`serde_json::Value`: `CompanySettings` is the largest object in the API, and
`Company` itself returns over a hundred fields, so only the structural ones
are modeled and `extra` carries the feature toggles.

## Users

`/users` is not a `Route::resource` and has no `DELETE /users/{id}`, so it is
hand-written:

| Endpoint | Method | Needs password? |
| --- | --- | --- |
| `GET /users` | `list_users`, `list_all_users` | no |
| `GET /users/{id}` | `get_user` | **yes** |
| `POST /users` | `create_user` | **yes** |
| `PUT /users/{id}` | `update_user` | **yes** |
| `DELETE /users/{id}/detach_from_company` | `detach_user_from_company` | **yes** |
| `POST /users/bulk` | `bulk_users` | **yes** |

"Needs password" means the route sits behind the API's `password_protected`
middleware and requires an `X-API-PASSWORD` header on top of a valid token.
Supply it with `InvoiceNinjaClient::with_api_password`, which also unlocks
client and vendor merge, and client/company/user purge. Note that even
*reading* a single user is gated this way.

## Entity actions

`GET /{resource}/{id}/{action}` exists for **only four** resources — invoices,
quotes, credits, and purchase orders. Each gets `{singular}_action(id, action)`
and `download_{singular}(id)`.

Actions returning the entity: `auto_bill`, `clone_to_invoice`,
`clone_to_quote`, `mark_paid`, `mark_sent`, `archive`, `restore`, `delete`.

Actions that do **not** return the entity, and so are not usable through
`*_action`:

| Action | Returns | Use instead |
| --- | --- | --- |
| `download` | `application/pdf` stream | `download_invoice(id)` etc. |
| `convert_to_purchase_order` | a purchase order | `convert_invoice_to_purchase_order(id)` |
| `cancel`, `history`, `delivery_note` | empty body (upstream quirk) | — not wrapped |

Each of the four also gets `download_{plural}(&ids)` — the bulk `download`
action, which streams one merged PDF. It can't go through `bulk_*` for the
same reason: the response isn't JSON.

Convenience wrappers: `mark_invoice_sent` and `mark_invoice_paid`.

## Document uploads

`PUT /{resource}/{id}/upload` exists for 16 resources, each getting
`upload_{singular}(id, files, is_public)` that returns the refreshed record:

| Resource | Method |
| --- | --- |
| Clients | `upload_client` |
| Companies | `upload_company` |
| Credits | `upload_credit` |
| Expenses | `upload_expense` |
| Group Settings | `upload_group_setting` |
| Invoices | `upload_invoice` |
| Payments | `upload_payment` |
| Products | `upload_product` |
| Projects | `upload_project` |
| Purchase Orders | `upload_purchase_order` |
| Quotes | `upload_quote` |
| Recurring Expenses | `upload_recurring_expense` |
| Recurring Invoices | `upload_recurring_invoice` |
| Recurring Quotes | `upload_recurring_quote` |
| Tasks | `upload_task` |
| Vendors | `upload_vendor` |

This is the *only* way the API creates a document:
`DocumentController::store` is an empty stub upstream, so `Document` has
`list`, `get`, `update`, `delete`, `download`, and `bulk`, but no
`create_document`, and there will not be one.

Three details that are easy to get wrong, all verified against the upstream
source rather than the published spec:

- **The request is a `POST` carrying `_method=PUT`.** The route is
  registered as `PUT`, but PHP does not parse a multipart body on a `PUT`, so
  the files would never arrive. Laravel's kernel turns the spoofed method back
  into a `PUT` before routing. The OpenAPI spec documents `POST` with
  `_method=POST`, which would 405 against these routes.
- **`is_public` is sent as `1`/`0`.** Laravel's `boolean` rule accepts only
  `true`, `false`, `0`, `1`, `'0'`, and `'1'` under strict comparison, so the
  string `"true"` fails validation.
- **Uploads are never retried**, even on a 429. An upload adds a document, so
  it is not idempotent whatever its verb, and a replay after a lost response
  would attach the file twice.

The server enforces file type and size limits; a refusal comes back as a 422
`Error::Api` with the per-field `errors` map. Files are held in memory while
uploading.

## Other wrapped endpoints

| Endpoint | Method |
| --- | --- |
| `GET /documents`, `/documents/{id}` | `list_documents`, `list_all_documents`, `get_document` |
| `PUT/DELETE /documents/{id}` | `update_document`, `delete_document` |
| `GET /documents/{id}/download` | `download_document` → bytes |
| `POST /documents/bulk` | `bulk_documents` |
| `POST /reports/{name}` | `report(name, body)` → raw bytes (CSV for most) |
| `POST /charts/{name}` | `chart(name, body)` → caller-chosen `T` |
| `POST /search` | `search(body)` |
| `GET /statics` | `statics()` |
| `POST /emails` | `send_email(body)` |
| `POST /payments/refund` | `refund_payment(body)` |
| `GET /ping` | `ping()` |
| `GET /health_check` | `health_check()` |

`report` and `chart` take the name as a string, so all 32 report and 11 chart
endpoints are reachable without a per-endpoint method:

- **Reports** — `activities`, `ar_detail_report`, `ar_summary_report`,
  `client_balance_report`, `client_contacts`, `client_locations`, `clients`,
  `client_sales_report`, `contacts`, `credits`, `documents`, `expenses`,
  `invoice_items`, `invoices`, `locations`, `payments`, `preview`, `products`,
  `product_sales`, `profitloss`, `projects`, `purchase_order_items`,
  `purchase_orders`, `quote_items`, `quotes`, `recurring_invoice_items`,
  `recurring_invoices`, `tasks`, `tax_period_report`, `tax_summary_report`,
  `user_sales_report`, `vendors`
- **Charts** — `totals`, `totals_v2`, `chart_summary`, `chart_summary_v2`,
  `calculated_fields`, `analytics_summary`, `analytics_totals`,
  `cashflow_forecast`, `client_payment_analytics`, `project_analytics`,
  `project_burnup`

Three of these take a further path segment rather than carrying the id in the
body: `charts/project_analytics/{project}`, `charts/project_burnup/{project}`,
and `reports/preview/{hash}`. The name argument is interpolated as-is, so pass
the whole tail — `chart("project_burnup/<id>", &body)`.

Large report exports are queued and emailed rather than returned inline; in
that case the body is a short JSON message instead of CSV.

## Not covered

### Nothing is blocked on a dependency

Multipart uploads used to be, and are now wrapped — see [Document
uploads](#document-uploads). One upload route is deliberately left out:
`PUT /edocument/upload` is not a document attachment. It imports an expense
from an e-document (`ExpenseController::edocument`), and is better treated as
an import endpoint alongside the others under *Import/export* below.

### Resource groups not wrapped

None — all 34 `Route::resource` groups are covered.

Adding a new one is mechanical: add a model under `src/models/`, re-export it
from `src/models/mod.rs`, and add a `resource_methods!` invocation in
`src/client.rs` (or `crud_methods!` if the resource has no `/bulk` route —
check `routes/api.php`).

### Password-protected endpoints still unwrapped

`with_api_password` makes these reachable, but the methods themselves aren't
written yet:

- `POST /clients/{id}/purge`
- `POST /clients/{id}/{other}/merge`
- `POST /vendors/{id}/{other}/merge`
- `POST /companies/purge/{company}`,
  `POST /companies/purge_save_settings/{company}`
- `POST /users/{id}/invite`, `POST /users/{id}/purge`, `GET /users/create`

### Individual endpoints not wrapped

- **Client extras** — `updateTaxData`, `show_settings`, `peppol_discovery`,
  `check_vat`, `vat_status`, `documents`
- **Bank integration extras** — `refresh_accounts`, `remove_account/{id}`,
  `get_transactions/{id}`, and `POST /bank_transactions/match`
- **Company gateway extras** — `test`, `clone`, `import_customers`
- **Company extras** — `GET /companies/{id}/logo`,
  `POST /companies/{id}/default`, `updateOriginTaxData`,
  `POST /companies/current`
- **Design extras** — `POST /designs/set/default`
- **Client gateway token extras** — `setAsDefault`
- **Task extras** — `POST /tasks/sort`
- **Webhook extras** — `POST /webhooks/{id}/retry`
- **Subscription extras** — `GET /subscriptions/steps`,
  `POST /subscriptions/steps/check`
- **User extras** (not password-protected, unlike the rest of `/users`) —
  `POST /users/{id}/disconnect_mailer`, `POST /users/{id}/disconnect_oauth`,
  `POST /user/{id}/reconfirm`
- **Preview** — `POST /preview`, `POST /preview/purchase_order`, live preview
- **Activities** — `GET /activities`, `POST /activities/entity`,
  `POST /activities/notes`, `download_entity/{activity}`
- **Email history** — `POST /emails/clientHistory/{client}`,
  `POST /emails/entityHistory`
- **Auth** — `/login`, `/logout`, `/refresh`, `/connected_account`,
  passkey routes
- **Import/export** — `/import`, `/preimport`, `/import_json`, `/export`,
  `/migration/start`
- **Invoice extras** — `delivery_note`, `payment_schedule`,
  `update_reminders`
- **Public invitation-key downloads** — `GET /{entity}/{invitation_key}/download`
  for `invoice`, `quote`, `credit`, and `purchase_order` (note the singular
  path segment), plus the matching e-invoice variants `download_e_invoice`,
  `download_e_quote`, `download_e_credit`,
  `download_e_purchase_order`. These are invitation-key based and take no
  auth token, so they don't fit the client's authenticated request path.
- **Integrations** — Stripe, QuickBooks, Apple in-app purchase, PostMark,
  einvoice (17 routes), calendar_connection, self-update, templates, hooks,
  Yodlee (`/yodlee/*`), Nordigen (`/nordigen/institutions`)
- **Misc** — `/company_users`, `/company_ledger`, `/client_statement`,
  `/claim_license`, `/check_license`, `/check_subdomain`, `/one_time_token`,
  `/last_error`, `/support/messages/send`, `/verify/*`, `/accounts/{id}`,
  `/feedback`, `/filters/{entity}`, `/scheduler`, `/mailer/check`,
  `/smtp/check`, `/password_timeout`, `/refresh_react`, `/webcron`,
  `/token_hash_router`, `/reactivate_email/{bounce_id}`,
  `/protected_download/{hash}`, `/confirm_purchase`,
  `/exports/preview/{hash}`, `/templates/preview/{hash}`

### Typed response gaps

`statics`, `search`, and `health_check` return concrete types (`Statics`,
`SearchResults`, `HealthCheck`). Each keeps the crate's `extra` convention,
so fields they don't model stay reachable rather than being dropped, and
each has a generic counterpart — `statics_as`, `search_as`,
`health_check_as` — for pinning a shape they leave untyped.

Two remain generic on purpose:

- **`chart`** — the eleven chart endpoints share no common response shape
  (`totals_v2` returns per-currency maps, `cashflow_forecast` a date series,
  `calculated_fields` scalars), so there is nothing to narrow to.
- **`send_email`** — the endpoint serves invoices, quotes, credits, and
  purchase orders, and answers with whichever entity was emailed. The `data`
  envelope *is* unwrapped, so `T` is the entity itself.

Within the typed models, a few fields are deliberately left as
`serde_json::Value`: `Statics::gateways`, `Statics::templates`,
`Statics::bulk_updates`, `HealthCheck::extensions`, and
`HealthCheck::queue_data`. These are nested configuration blobs rather than
uniform lookup rows, and the same reasoning applies as for
`Company::settings` — a wrong concrete type is a hard decode error, whereas
`Value` is lossless.

## Compared with the official PHP SDK

[invoiceninja/sdk-php](https://github.com/invoiceninja/sdk-php) is the
first-party wrapper. It's worth checking against, but it is deliberately
thinner than this crate: every one of its endpoint classes is just a URI on
top of a shared `BaseEntity` offering `all`, `get`, `create`, `update`,
`bulk`, `archive`, `delete`, `restore`, and `download`. Only `Companies` and
`Statics` add anything of their own.

**It covers nothing we don't.** The five resources it used to have on us —
`users`, `companies`, `group_settings`, `bank_integrations`,
`client_gateway_tokens` — are all wrapped now.

**We cover a lot it doesn't:** `payment_terms`, `task_statuses`,
`recurring_quotes`, `recurring_expenses`, `bank_transaction_rules`,
`company_gateways`, `designs`, `locations`, `tags`, `task_schedulers`,
`tokens`, `system_logs`, documents, per-entity actions (`mark_sent`,
`mark_paid`, `clone_to_*`, …), per-entity PDF download, reports, charts,
search, `send_email`, `refund_payment`, `ping`, `health_check`, automatic
pagination (`list_all_*`), and `X-API-PASSWORD` support for the
password-protected routes.

Deliberate differences in behaviour:

| | PHP SDK | This crate |
| --- | --- | --- |
| Request bodies | `form_params` (urlencoded) | JSON |
| Base URL | host only; appends `/api/v1` itself | full API root, or `hosted()` |
| Single-entity delete | none — `delete()` is bulk | `delete_*(id)` via HTTP `DELETE`, plus `bulk_*` |
| Validation errors | surfaces only the first message | whole per-field `errors` map on `Error::Api` |
| List filters | arbitrary array | typed setters plus `ListParams::param` |

The urlencoded-vs-JSON difference is the one that matters: form encoding
bracket-encodes nested structures, so a payload like an invoice's
`line_items` array of objects is far cleaner as JSON. Both are accepted by
the API.

Two things in the PHP SDK are *not* worth copying: it sends
`stream: false` in download payloads, but no controller reads a `stream`
input, and its `BaseEntity::download` is available on every resource even
though only PDF-rendering entities support the action.
