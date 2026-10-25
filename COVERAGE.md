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
not a resource group). 325 methods in total.

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

`create_recurring_quote` currently 500s against Invoice Ninja 5.13.50: the
server's own insert names a `design_id` column its `recurring_quotes` table
does not have. The request is well-formed; nothing here can work around it.

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

## Activities, email history and previews

Each has a response shape of its own. Marked *observed* where confirmed
against a 5.13.50 server.

| Endpoint | Method | Returns |
| --- | --- | --- |
| `GET /activities` | `list_activities`, `list_all_activities` | `Vec<Activity>` (*observed*) |
| `GET /activities?reactv2=1` | `activity_feed(rows)` | `Vec<ActivityFeedEntry>` (*observed*) |
| `POST /activities/entity` | `entity_activity(entity, id, rows)` | `Vec<ActivityFeedEntry>` (*observed*) |
| `POST /activities/notes` | `add_activity_note(entity, id, notes)` | `Activity` (*observed*) |
| `GET /activities/download_entity/{id}` | `download_activity_entity` | PDF bytes |
| `POST /emails/clientHistory/{client}` | `client_email_history` | `Vec<serde_json::Value>` |
| `POST /emails/entityHistory` | `entity_email_history` | `Vec<serde_json::Value>` |
| `POST /preview` | `preview`, `preview_html` | PDF bytes / HTML |
| `POST /live_preview` | `live_preview` | PDF bytes (*observed*) |
| `POST /live_design` | `live_design` | PDF bytes |
| `POST /preview/purchase_order` | `preview_purchase_order` | PDF bytes |
| `POST /live_preview/purchase_order` | `live_preview_purchase_order` | PDF bytes |

Things that are easy to get wrong:

- **The entity name is singular in one route and plural in the next.**
  `entity_activity` takes `invoice`; `add_activity_note` takes `invoices`.
- **`rows`, not `per_page`, caps the activity log** (default 75). Pass it
  through `ListParams::param`. `entity_activity` leaves `rows` out when not
  given rather than sending `null`, because the server tests for the key and
  a null would count as given and remove the cap.
- **The two activity shapes differ.** `Activity` carries hashed ids and a
  string `activity_type_id`; the feed's `ActivityFeedEntry` carries a numeric
  one and labelled references (`invoice`, `client`, … each `{label,
  hashed_id}`).
- **Email history is raw JSON.** The objects mirror what the mail provider
  reported, and the instance available had never sent mail, so there was no
  shape to take.
- **Not yet run against a real server:** `preview`, `preview_html`, and both
  purchase-order previews (they need a design object or a purchase order and a
  PDF backend), and `live_design`, which answered a bare minimal request with
  a 500 from the server on 5.13.50.

## Password-protected actions

Behind the `password_protected` middleware, so each needs
`with_api_password` (the server also lets a recent password-authenticated
session through without the header). Most are **irreversible**, and all are
`POST`s except one, so none is retried on a 5xx.

| Endpoint | Method | Returns |
| --- | --- | --- |
| `POST /clients/{id}/purge` | `purge_client` | `MessageResponse` |
| `POST /clients/{id}/{other}/merge` | `merge_clients(id, other)` | the surviving `Client` |
| `POST /vendors/{id}/{other}/merge` | `merge_vendors(id, other)` | the surviving `Vendor` |
| `POST /companies/purge/{id}` | `purge_company` | `MessageResponse` |
| `POST /companies/purge_save_settings/{id}` | `purge_company_save_settings` | `MessageResponse` |
| `POST /users/{id}/invite` | `invite_user` | `MessageResponse` |
| `POST /users/{id}/purge` | `purge_user` | `()` (the server answers 204) |
| `GET /users/create` | `new_user_template` | a blank `User`, nothing stored |

A merge keeps the first id and folds the second into it. The smoke test's
optional password tier (merge two throwaway clients, purge the survivor) was
run against a disposable 5.13.50 instance on 2026-10-24 and passes; it also
established that a purged record reads back as a **400**, not a 404. A
password containing a control character cannot be a header value and goes out
as `X-API-PASSWORD-BASE64` instead.

## Per-resource extras

Routes that sit beside a resource's CRUD set, each with its own response
shape (read from the upstream controllers; the ones marked *observed* were
also confirmed against a 5.13.50 server). All are `POST` unless noted, so none
is retried on a 5xx or a timeout.

| Endpoint | Method | Returns |
| --- | --- | --- |
| `POST /webhooks/{id}/retry` | `retry_webhook` | `Webhook` |
| `POST /tasks/sort` | `sort_tasks` | `MessageResponse` |
| `POST /designs/set/default` | `set_default_design` | `MessageResponse` |
| `POST /client_gateway_tokens/{id}/setAsDefault` | `set_default_client_gateway_token` | `ClientGatewayToken` |
| `GET /subscriptions/steps` | `subscription_steps` | `Vec<SubscriptionStep>` (*observed*: an object keyed by class name, not the array the source suggests) |
| `POST /subscriptions/steps/check` | `check_subscription_steps` | `()` |
| `POST /company_gateways/{id}/test` | `test_company_gateway` | `MessageResponse` |
| `POST /company_gateways/{id}/clone` | `clone_company_gateway` | `CompanyGateway` |
| `POST /company_gateways/{id}/import_customers` | `import_company_gateway_customers` | `MessageResponse` |
| `POST /bank_integrations/refresh_accounts` | `refresh_bank_accounts` | `()` (the body is a serialised query builder) |
| `POST /bank_integrations/remove_account/{id}` | `remove_bank_account` | `BankIntegration` |
| `POST /bank_integrations/get_transactions/{id}` | `fetch_bank_transactions` | `MessageResponse` (the server ignores the id) |
| `POST /bank_transactions/match` | `match_bank_transactions` | `Vec<BankTransaction>` |
| `POST /clients/{id}/updateTaxData` | `update_client_tax_data` | `Client` |
| `POST /clients/{id}/show_settings` | `client_show_settings` | `serde_json::Value` (*observed*) |
| `POST /clients/{id}/peppol_discovery` | `client_peppol_discovery` | `serde_json::Value` |
| `POST /clients/{id}/check_vat` | `check_client_vat` | `MessageResponse` |
| `GET /clients/{id}/vat_status` | `client_vat_status` | `MessageResponse` (*observed*) |
| `POST /clients/{id}/documents` | `client_documents` | `Vec<Document>` (*observed*; one page) |

`updateTaxData` and `check_vat` call out to external tax services, so the
smoke test does not exercise them.

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

### Individual endpoints not wrapped

- **Company extras** — `GET /companies/{id}/logo`,
  `POST /companies/{id}/default`, `updateOriginTaxData`,
  `POST /companies/current`
- **User extras** (not password-protected, unlike the rest of `/users`) —
  `POST /users/{id}/disconnect_mailer`, `POST /users/{id}/disconnect_oauth`,
  `POST /user/{id}/reconfirm`
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
