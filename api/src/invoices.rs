use crate::database::Database;
use anyhow::{Context, Result};
use chrono::Datelike;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

/// Invoice metadata stored in database
#[derive(Debug, Serialize, Deserialize, Object, sqlx::FromRow)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct Invoice {
    pub id: i64,
    #[serde(skip)]
    pub contract_id: Vec<u8>,
    pub invoice_number: String,
    pub invoice_date_ns: i64,
    pub seller_name: String,
    pub seller_address: String,
    pub seller_vat_id: Option<String>,
    pub buyer_name: Option<String>,
    pub buyer_address: Option<String>,
    pub buyer_vat_id: Option<String>,
    pub subtotal_e9s: i64,
    pub vat_rate_percent: i64,
    pub vat_amount_e9s: i64,
    pub total_e9s: i64,
    pub currency: String,
    #[serde(skip)]
    pub pdf_blob: Option<Vec<u8>>,
    pub pdf_generated_at_ns: Option<i64>,
    pub created_at_ns: i64,
}

/// JSON format for invoice-maker Typst template
#[derive(Debug, Serialize)]
struct InvoiceData {
    language: String,
    #[serde(rename = "invoice-id")]
    invoice_id: String,
    #[serde(rename = "issuing-date")]
    issuing_date: String,
    #[serde(rename = "delivery-date")]
    delivery_date: String,
    #[serde(rename = "due-date")]
    due_date: String,
    biller: InvoiceParty,
    recipient: InvoiceParty,
    items: Vec<InvoiceItem>,
    vat: i64,
    currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

#[derive(Debug, Serialize)]
struct InvoiceParty {
    name: String,
    address: InvoiceAddress,
    #[serde(rename = "vat-id", skip_serializing_if = "Option::is_none")]
    vat_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct InvoiceAddress {
    street: String,
    city: String,
    #[serde(rename = "postal-code")]
    postal_code: String,
    country: String,
}

#[derive(Debug, Serialize)]
struct InvoiceItem {
    number: i64,
    description: String,
    quantity: i64,
    price: f64,
}

/// Get next invoice number atomically, format: INV-YYYY-NNNNNN
async fn get_next_invoice_number(db: &Database) -> Result<String> {
    let current_year = chrono::Utc::now().year();

    // Try to get and increment for current year
    #[derive(sqlx::FromRow)]
    struct InvoiceNumberRow {
        number: i64,
    }

    let result: Option<InvoiceNumberRow> = sqlx::query_as(
        "UPDATE invoice_sequence SET next_number = next_number + 1 WHERE id = 1 AND year = ? RETURNING next_number - 1 as number",
    )
    .bind(current_year)
    .fetch_optional(&db.pool)
    .await?;

    let number = match result {
        Some(row) => row.number,
        None => {
            // Year changed, reset sequence
            sqlx::query("UPDATE invoice_sequence SET year = ?, next_number = 2 WHERE id = 1")
                .bind(current_year)
                .execute(&db.pool)
                .await?;
            1
        }
    };

    Ok(format!("INV-{}-{:06}", current_year, number))
}

/// Create invoice record in database and generate PDF
pub async fn create_invoice(db: &Database, contract_id: &[u8]) -> Result<Invoice> {
    // Check if invoice already exists
    if let Some(invoice) = get_invoice_by_contract(db, contract_id).await? {
        return Ok(invoice);
    }

    // Get contract details
    let contract = db
        .get_contract(contract_id)
        .await?
        .context("Contract not found")?;

    // Get next invoice number
    let invoice_number = get_next_invoice_number(db).await?;
    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    // Seller details from environment (required for compliant invoices)
    let seller_name =
        std::env::var("INVOICE_SELLER_NAME").unwrap_or_else(|_| "Decent Cloud Ltd".to_string());
    let seller_address =
        std::env::var("INVOICE_SELLER_ADDRESS").unwrap_or_else(|_| "Address TBD".to_string());
    let seller_vat_id = std::env::var("INVOICE_SELLER_VAT_ID").ok();

    // Warn if seller details not configured (invoices won't be EU-compliant)
    if std::env::var("INVOICE_SELLER_ADDRESS").is_err() {
        tracing::warn!(
            "INVOICE_SELLER_ADDRESS not set - invoices will NOT be EU VAT compliant! \
             Set INVOICE_SELLER_NAME, INVOICE_SELLER_ADDRESS, and INVOICE_SELLER_VAT_ID for compliance."
        );
    }

    // Buyer details (from contract or placeholder)
    let buyer_name = Some(contract.requester_contact.clone());
    let buyer_address: Option<String> = None;
    let buyer_vat_id = contract.customer_tax_id.clone();

    // Amounts - use tax from contract if available (from Stripe Tax or manual entry)
    let subtotal_e9s = contract.payment_amount_e9s;
    let vat_rate_percent = contract.tax_rate_percent.unwrap_or(0.0) as i64;
    let vat_amount_e9s = contract.tax_amount_e9s.unwrap_or(0);
    let total_e9s = subtotal_e9s + vat_amount_e9s;

    // Insert invoice record
    let result = sqlx::query(
        r#"INSERT INTO invoices
           (contract_id, invoice_number, invoice_date_ns, seller_name, seller_address, seller_vat_id,
            buyer_name, buyer_address, buyer_vat_id, subtotal_e9s, vat_rate_percent, vat_amount_e9s,
            total_e9s, currency, created_at_ns)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(contract_id)
    .bind(&invoice_number)
    .bind(now_ns)
    .bind(&seller_name)
    .bind(&seller_address)
    .bind(&seller_vat_id)
    .bind(&buyer_name)
    .bind(&buyer_address)
    .bind(&buyer_vat_id)
    .bind(subtotal_e9s)
    .bind(vat_rate_percent)
    .bind(vat_amount_e9s)
    .bind(total_e9s)
    .bind(&contract.currency)
    .bind(now_ns)
    .execute(&db.pool)
    .await?;

    let invoice_id = result.last_insert_rowid();

    // Fetch the created invoice
    let invoice: Invoice = sqlx::query_as("SELECT * FROM invoices WHERE id = ?")
        .bind(invoice_id)
        .fetch_one(&db.pool)
        .await?;

    tracing::info!(
        "Created invoice {} for contract {}",
        invoice_number,
        hex::encode(contract_id)
    );

    Ok(invoice)
}

/// Get invoice by contract ID
async fn get_invoice_by_contract(db: &Database, contract_id: &[u8]) -> Result<Option<Invoice>> {
    let invoice: Option<Invoice> = sqlx::query_as("SELECT * FROM invoices WHERE contract_id = ?")
        .bind(contract_id)
        .fetch_optional(&db.pool)
        .await?;

    Ok(invoice)
}

/// Generate PDF invoice using Typst CLI
async fn generate_invoice_pdf(db: &Database, invoice: &Invoice) -> Result<Vec<u8>> {
    // Get contract details for invoice content
    let contract = db
        .get_contract(&invoice.contract_id)
        .await?
        .context("Contract not found")?;

    // Format dates
    let invoice_date = chrono::DateTime::from_timestamp(invoice.invoice_date_ns / 1_000_000_000, 0)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "N/A".to_string());

    let delivery_date = invoice_date.clone();
    let due_date = "Paid".to_string(); // Invoice is only generated after payment

    // Build invoice description
    let offering_name = format!("Cloud VPS Rental - Offering {}", contract.offering_id);
    let duration = contract.duration_hours.unwrap_or(0);
    let start_date = contract
        .start_timestamp_ns
        .and_then(|ts| chrono::DateTime::from_timestamp(ts / 1_000_000_000, 0))
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "N/A".to_string());
    let end_date = contract
        .end_timestamp_ns
        .and_then(|ts| chrono::DateTime::from_timestamp(ts / 1_000_000_000, 0))
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "N/A".to_string());

    let description = format!(
        "{}\nDuration: {} hours\nPeriod: {} - {}",
        offering_name, duration, start_date, end_date
    );

    // Convert e9s to decimal
    let price = invoice.subtotal_e9s as f64 / 1_000_000_000.0;

    // Determine invoice note based on VAT status and payment method
    let note = if invoice.seller_vat_id.is_none() && invoice.vat_rate_percent == 0 {
        // Seller not VAT registered
        Some("VAT not applicable - seller not registered for VAT.".to_string())
    } else if contract.payment_method == "icpay" {
        // Crypto payment - tax handling differs
        Some("Paid via cryptocurrency. Buyer responsible for any applicable tax obligations.".to_string())
    } else {
        None
    };

    // Build invoice data
    let invoice_data = InvoiceData {
        language: "en".to_string(),
        invoice_id: invoice.invoice_number.clone(),
        issuing_date: invoice_date,
        delivery_date,
        due_date,
        biller: InvoiceParty {
            name: invoice.seller_name.clone(),
            address: InvoiceAddress {
                street: "".to_string(),
                city: "".to_string(),
                postal_code: "".to_string(),
                country: invoice.seller_address.clone(),
            },
            vat_id: invoice.seller_vat_id.clone(),
        },
        recipient: InvoiceParty {
            name: invoice
                .buyer_name
                .clone()
                .unwrap_or_else(|| "Customer".to_string()),
            address: InvoiceAddress {
                street: "".to_string(),
                city: "".to_string(),
                postal_code: "".to_string(),
                country: invoice.buyer_address.clone().unwrap_or_default(),
            },
            vat_id: invoice.buyer_vat_id.clone(),
        },
        items: vec![InvoiceItem {
            number: 1,
            description,
            quantity: 1,
            price,
        }],
        vat: invoice.vat_rate_percent,
        currency: invoice.currency.clone(),
        note,
    };

    // Serialize to JSON
    let json_data = serde_json::to_string(&invoice_data)?;

    // Create temp directory for Typst output
    let temp_dir = tempfile::tempdir()?;
    let output_path = temp_dir.path().join("invoice.pdf");

    // Get template path (relative to workspace root)
    let template_path = std::env::current_dir()?.join("api/templates/invoice.typ");

    if !template_path.exists() {
        anyhow::bail!("Invoice template not found at {:?}", template_path);
    }

    // Run Typst CLI
    let output = tokio::process::Command::new("typst")
        .arg("compile")
        .arg("--input")
        .arg(format!("data={}", json_data))
        .arg(&template_path)
        .arg(&output_path)
        .output()
        .await
        .context("Failed to execute typst command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Typst compilation failed: {}", stderr);
    }

    // Read generated PDF
    let pdf_bytes = tokio::fs::read(&output_path)
        .await
        .context("Failed to read generated PDF")?;

    tracing::info!(
        "Generated PDF invoice {} ({} bytes)",
        invoice.invoice_number,
        pdf_bytes.len()
    );

    Ok(pdf_bytes)
}

/// Get invoice PDF, generate if not cached
pub async fn get_invoice_pdf(db: &Database, contract_id: &[u8]) -> Result<Vec<u8>> {
    // Get or create invoice
    let mut invoice = match get_invoice_by_contract(db, contract_id).await? {
        Some(inv) => inv,
        None => create_invoice(db, contract_id).await?,
    };

    // Return cached PDF if exists
    if let Some(pdf_blob) = &invoice.pdf_blob {
        return Ok(pdf_blob.clone());
    }

    // Generate PDF
    let pdf_bytes = generate_invoice_pdf(db, &invoice).await?;
    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    // Cache PDF in database
    sqlx::query("UPDATE invoices SET pdf_blob = ?, pdf_generated_at_ns = ? WHERE id = ?")
        .bind(&pdf_bytes)
        .bind(now_ns)
        .bind(invoice.id)
        .execute(&db.pool)
        .await?;

    invoice.pdf_blob = Some(pdf_bytes.clone());
    invoice.pdf_generated_at_ns = Some(now_ns);

    Ok(pdf_bytes)
}

/// Get invoice metadata
pub async fn get_invoice_metadata(db: &Database, contract_id: &[u8]) -> Result<Invoice> {
    match get_invoice_by_contract(db, contract_id).await? {
        Some(invoice) => Ok(invoice),
        None => create_invoice(db, contract_id).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_helpers::setup_test_db;

    #[tokio::test]
    async fn test_get_next_invoice_number_sequential() {
        let db = setup_test_db().await;

        let num1 = get_next_invoice_number(&db).await.unwrap();
        let num2 = get_next_invoice_number(&db).await.unwrap();
        let num3 = get_next_invoice_number(&db).await.unwrap();

        let year = chrono::Utc::now().year();
        assert_eq!(num1, format!("INV-{}-000001", year));
        assert_eq!(num2, format!("INV-{}-000002", year));
        assert_eq!(num3, format!("INV-{}-000003", year));
    }

    #[tokio::test]
    async fn test_get_next_invoice_number_year_rollover() {
        let db = setup_test_db().await;

        // Set sequence to year 2024
        sqlx::query("UPDATE invoice_sequence SET year = 2024, next_number = 100 WHERE id = 1")
            .execute(&db.pool)
            .await
            .unwrap();

        // Get invoice number (should reset for current year)
        let num = get_next_invoice_number(&db).await.unwrap();

        let current_year = chrono::Utc::now().year();
        assert_eq!(num, format!("INV-{}-000001", current_year));

        // Next number should increment normally
        let num2 = get_next_invoice_number(&db).await.unwrap();
        assert_eq!(num2, format!("INV-{}-000002", current_year));
    }

    #[tokio::test]
    async fn test_create_invoice() {
        let db = setup_test_db().await;

        // Create test contract
        let contract_id = vec![1u8; 32];
        let requester_pk = vec![2u8; 32];
        let provider_pk = vec![3u8; 32];

        sqlx::query(
            r#"INSERT INTO contract_sign_requests
               (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact,
                provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns,
                payment_method, payment_status, currency)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&contract_id)
        .bind(&requester_pk)
        .bind("ssh-key")
        .bind("customer@example.com")
        .bind(&provider_pk)
        .bind("off-1")
        .bind(100_000_000_000i64) // 100 USD
        .bind("test")
        .bind(0i64)
        .bind("stripe")
        .bind("succeeded")
        .bind("USD")
        .execute(&db.pool)
        .await
        .unwrap();

        // Create invoice
        let invoice = create_invoice(&db, &contract_id).await.unwrap();

        assert!(invoice.invoice_number.starts_with("INV-"));
        assert_eq!(invoice.subtotal_e9s, 100_000_000_000);
        assert_eq!(invoice.currency, "USD");
        assert_eq!(invoice.seller_name, "Decent Cloud Ltd");
        assert_eq!(invoice.buyer_name, Some("customer@example.com".to_string()));

        // Creating again should return existing invoice
        let invoice2 = create_invoice(&db, &contract_id).await.unwrap();
        assert_eq!(invoice.id, invoice2.id);
    }

    #[tokio::test]
    async fn test_get_invoice_metadata() {
        let db = setup_test_db().await;

        // Create test contract
        let contract_id = vec![4u8; 32];
        let requester_pk = vec![5u8; 32];
        let provider_pk = vec![6u8; 32];

        sqlx::query(
            r#"INSERT INTO contract_sign_requests
               (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact,
                provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns,
                payment_method, payment_status, currency)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&contract_id)
        .bind(&requester_pk)
        .bind("ssh-key")
        .bind("test@example.com")
        .bind(&provider_pk)
        .bind("off-2")
        .bind(50_000_000_000i64)
        .bind("test")
        .bind(0i64)
        .bind("stripe")
        .bind("succeeded")
        .bind("EUR")
        .execute(&db.pool)
        .await
        .unwrap();

        // Get metadata (should create invoice if doesn't exist)
        let metadata = get_invoice_metadata(&db, &contract_id).await.unwrap();

        assert!(metadata.invoice_number.starts_with("INV-"));
        assert_eq!(metadata.currency, "EUR");
        assert_eq!(metadata.total_e9s, 50_000_000_000);
    }
}
