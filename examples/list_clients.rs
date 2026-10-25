use invoiceninja_sdk::models::ListParams;
use invoiceninja_sdk::InvoiceNinjaClient;

#[tokio::main]
async fn main() -> invoiceninja_sdk::Result<()> {
    let base_url =
        std::env::var("IN_BASE_URL").unwrap_or_else(|_| "https://invoicing.co/api/v1".into());
    let token = std::env::var("IN_API_TOKEN").expect("set IN_API_TOKEN");

    let client = InvoiceNinjaClient::new(base_url, token)?;

    let clients = client
        .list_clients(&ListParams::new().per_page(10).sort("created_at|desc"))
        .await?;

    for c in clients {
        println!("{:?}\t{}", c.id, c.name);
    }

    Ok(())
}
