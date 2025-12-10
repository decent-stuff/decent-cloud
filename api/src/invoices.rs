use crate::database::Database;
use crate::invoice_storage::{self, InvoiceType};
use anyhow::{Context, Result};
use chrono::Datelike;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

/// Invoice metadata stored in database (PDF stored on disk)
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
    pub pdf_generated_at_ns: Option<i64>,
    pub created_at_ns: i64,
}

/// JSON format for Typst invoice template
#[derive(Debug, Serialize)]
struct InvoiceData {
    /// Language code for invoice (en, de, fr, es). Defaults to "en" if not specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<String>,
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
    #[serde(skip_serializing_if = "String::is_empty")]
    iban: String,
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
    description: String,
    quantity: i64,
    price: f64,
}

/// Parse freeform buyer address into structured address fields.
/// Expected format (from UI placeholder):
///   Company Name (already in buyer_name)
///   Street Address
///   City, Postal Code
///   Country
/// Falls back gracefully if format doesn't match.
fn parse_buyer_address(address: Option<&str>) -> InvoiceAddress {
    let Some(addr) = address.filter(|s| !s.trim().is_empty()) else {
        return InvoiceAddress {
            street: String::new(),
            city: String::new(),
            postal_code: String::new(),
            country: String::new(),
        };
    };

    let lines: Vec<&str> = addr
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    match lines.len() {
        0 => InvoiceAddress {
            street: String::new(),
            city: String::new(),
            postal_code: String::new(),
            country: String::new(),
        },
        1 => InvoiceAddress {
            street: lines[0].to_string(),
            city: String::new(),
            postal_code: String::new(),
            country: String::new(),
        },
        2 => InvoiceAddress {
            street: lines[0].to_string(),
            city: String::new(),
            postal_code: String::new(),
            country: lines[1].to_string(),
        },
        3 => {
            // street, city+postal, country
            let (city, postal) = parse_city_postal(lines[1]);
            InvoiceAddress {
                street: lines[0].to_string(),
                city,
                postal_code: postal,
                country: lines[2].to_string(),
            }
        }
        _ => {
            // 4+ lines: join first lines as street, then city+postal, country
            let street = lines[..lines.len() - 2].join(", ");
            let (city, postal) = parse_city_postal(lines[lines.len() - 2]);
            InvoiceAddress {
                street,
                city,
                postal_code: postal,
                country: lines[lines.len() - 1].to_string(),
            }
        }
    }
}

/// Parse "City, Postal Code" or just "City" into (city, postal_code)
fn parse_city_postal(s: &str) -> (String, String) {
    if let Some((city, postal)) = s.split_once(',') {
        (city.trim().to_string(), postal.trim().to_string())
    } else {
        (s.to_string(), String::new())
    }
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

/// Create invoice record in database
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

    // Buyer details (from contract)
    let buyer_name = Some(contract.requester_contact.clone());
    let buyer_address = contract.buyer_address.clone();
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

    // Determine invoice note based on VAT status, reverse charge, and payment method
    let note = if contract.reverse_charge.unwrap_or(0) == 1 {
        // Reverse charge applies - B2B cross-border EU transaction
        Some("Reverse charge - VAT to be accounted for by the recipient as per Article 196 of Council Directive 2006/112/EC.".to_string())
    } else if invoice.seller_vat_id.is_none() && invoice.vat_rate_percent == 0 {
        // Seller not VAT registered
        Some("VAT not applicable - seller not registered for VAT.".to_string())
    } else if contract.payment_method == "icpay" {
        // Crypto payment - tax handling differs
        Some(
            "Paid via cryptocurrency. Buyer responsible for any applicable tax obligations."
                .to_string(),
        )
    } else {
        None
    };

    // Build invoice data
    // Get seller IBAN from environment (optional - bank details section will be hidden if not set)
    let seller_iban = std::env::var("INVOICE_SELLER_IBAN").unwrap_or_default();

    let invoice_data = InvoiceData {
        language: None, // Use default (English) for now
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
            iban: seller_iban,
        },
        recipient: InvoiceParty {
            name: invoice
                .buyer_name
                .clone()
                .unwrap_or_else(|| "Customer".to_string()),
            address: parse_buyer_address(invoice.buyer_address.as_deref()),
            vat_id: invoice.buyer_vat_id.clone(),
            iban: String::new(),
        },
        items: vec![InvoiceItem {
            description,
            quantity: 1,
            price,
        }],
        vat: invoice.vat_rate_percent,
        currency: invoice.currency.clone(),
        note,
    };

    // Create temp directory for Typst files
    let temp_dir = tempfile::tempdir()?;
    let output_path = temp_dir.path().join("invoice.pdf");

    // Write JSON data to file (avoids command-line length limits)
    let json_path = temp_dir.path().join("data.json");
    let json_data = serde_json::to_string(&invoice_data)?;
    tokio::fs::write(&json_path, &json_data)
        .await
        .context(format!(
            "Failed to write invoice data to {}",
            json_path.display()
        ))?;

    // Write embedded template to temp file (template is compiled into binary)
    const INVOICE_TEMPLATE: &str = include_str!("../templates/invoice.typ");
    let template_path = temp_dir.path().join("invoice.typ");
    tokio::fs::write(&template_path, INVOICE_TEMPLATE)
        .await
        .context(format!(
            "Failed to write invoice template to {}",
            template_path.display()
        ))?;

    // Run Typst CLI - set cache dir if not already set (for package downloads)
    let mut cmd = tokio::process::Command::new("typst");
    if std::env::var("XDG_CACHE_HOME").is_err() {
        cmd.env("XDG_CACHE_HOME", temp_dir.path());
    }
    let output = cmd
        .arg("compile")
        .arg("--input")
        .arg("data_file=data.json") // Relative to template in same temp dir
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
    let pdf_bytes = tokio::fs::read(&output_path).await.context(format!(
        "Failed to read generated PDF from {}",
        output_path.display()
    ))?;

    tracing::info!(
        "Generated PDF invoice {} ({} bytes)",
        invoice.invoice_number,
        pdf_bytes.len()
    );

    Ok(pdf_bytes)
}

/// Get invoice PDF - prefers Stripe invoice, falls back to Typst
///
/// Strategy:
/// 1. Always ensure a Typst invoice exists on disk (generated on first call)
/// 2. For Stripe payments with invoice ID, fetch and persist Stripe PDF to disk
/// 3. Return Stripe PDF if available, otherwise return Typst PDF
///
/// Both invoice types are stored on disk in {LEDGER_DIR}/invoices/{contract_id}/
/// This ensures both invoices are available, and if a Stripe invoice
/// arrives later (e.g., after user already clicked UI to get Typst invoice),
/// it will be persisted and used for subsequent requests.
pub async fn get_invoice_pdf(db: &Database, contract_id: &[u8]) -> Result<Vec<u8>> {
    // Get contract to check payment method and Stripe invoice ID
    let contract = db
        .get_contract(contract_id)
        .await?
        .context("Contract not found")?;

    // Step 1: Ensure Typst invoice exists on disk (creates record + generates PDF if needed)
    // This is our fallback, so we always want it available
    let typst_pdf = get_typst_invoice_pdf(db, contract_id).await?;

    // Step 2: For Stripe payments, try to get/fetch Stripe PDF
    if contract.payment_method == "stripe" {
        if let Some(stripe_invoice_id) = &contract.stripe_invoice_id {
            // Check if we already have cached Stripe PDF on disk
            if let Some(cached_pdf) =
                invoice_storage::load_invoice_pdf(contract_id, InvoiceType::Stripe).await?
            {
                return Ok(cached_pdf);
            }

            // Fetch from Stripe and persist to disk
            match fetch_and_persist_stripe_pdf(contract_id, stripe_invoice_id).await {
                Ok(pdf) => return Ok(pdf),
                Err(e) => {
                    tracing::warn!(
                        "Failed to fetch Stripe invoice PDF for contract {}: {}. Using Typst fallback.",
                        hex::encode(contract_id),
                        e
                    );
                    // Fall through to Typst
                }
            }
        } else {
            tracing::debug!(
                "Stripe payment without invoice ID for contract {}, using Typst invoice",
                hex::encode(contract_id)
            );
        }
    }

    // Return Typst PDF as fallback
    Ok(typst_pdf)
}

/// Fetch Stripe invoice PDF and persist it to disk
async fn fetch_and_persist_stripe_pdf(contract_id: &[u8], invoice_id: &str) -> Result<Vec<u8>> {
    let stripe_client = crate::stripe_client::StripeClient::new()?;
    let pdf_url = stripe_client
        .get_invoice_pdf_url(invoice_id)
        .await?
        .context("Stripe invoice PDF not yet available")?;

    // Download PDF from Stripe URL
    let response = reqwest::get(&pdf_url)
        .await
        .context("Failed to download invoice PDF from Stripe")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to download invoice PDF from Stripe: {}",
            response.status()
        );
    }

    let pdf_bytes = response
        .bytes()
        .await
        .context("Failed to read invoice PDF bytes")?
        .to_vec();

    // Persist to disk for future use
    invoice_storage::save_invoice_pdf(contract_id, InvoiceType::Stripe, &pdf_bytes).await?;

    tracing::info!(
        "Fetched and persisted Stripe invoice PDF for contract {} ({} bytes)",
        hex::encode(contract_id),
        pdf_bytes.len()
    );

    Ok(pdf_bytes)
}

/// Get or generate Typst invoice PDF (stored on disk)
async fn get_typst_invoice_pdf(db: &Database, contract_id: &[u8]) -> Result<Vec<u8>> {
    // Check if PDF already exists on disk
    if let Some(pdf) = invoice_storage::load_invoice_pdf(contract_id, InvoiceType::Typst).await? {
        return Ok(pdf);
    }

    // Get or create invoice metadata record
    let invoice = match get_invoice_by_contract(db, contract_id).await? {
        Some(inv) => inv,
        None => create_invoice(db, contract_id).await?,
    };

    // Generate PDF using Typst
    let pdf_bytes = generate_invoice_pdf(db, &invoice).await?;

    // Save to disk
    invoice_storage::save_invoice_pdf(contract_id, InvoiceType::Typst, &pdf_bytes).await?;

    // Update metadata in DB (just the timestamp, PDF is on disk)
    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    sqlx::query("UPDATE invoices SET pdf_generated_at_ns = ? WHERE id = ?")
        .bind(now_ns)
        .bind(invoice.id)
        .execute(&db.pool)
        .await?;

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

    #[test]
    fn test_parse_buyer_address_none() {
        let addr = parse_buyer_address(None);
        assert_eq!(addr.street, "");
        assert_eq!(addr.city, "");
        assert_eq!(addr.postal_code, "");
        assert_eq!(addr.country, "");
    }

    #[test]
    fn test_parse_buyer_address_empty() {
        let addr = parse_buyer_address(Some("   "));
        assert_eq!(addr.street, "");
        assert_eq!(addr.country, "");
    }

    #[test]
    fn test_parse_buyer_address_single_line() {
        let addr = parse_buyer_address(Some("123 Main St"));
        assert_eq!(addr.street, "123 Main St");
        assert_eq!(addr.city, "");
        assert_eq!(addr.country, "");
    }

    #[test]
    fn test_parse_buyer_address_two_lines() {
        let addr = parse_buyer_address(Some("123 Main St\nGermany"));
        assert_eq!(addr.street, "123 Main St");
        assert_eq!(addr.country, "Germany");
    }

    #[test]
    fn test_parse_buyer_address_three_lines() {
        let addr = parse_buyer_address(Some("123 Main St\nBerlin, 10115\nGermany"));
        assert_eq!(addr.street, "123 Main St");
        assert_eq!(addr.city, "Berlin");
        assert_eq!(addr.postal_code, "10115");
        assert_eq!(addr.country, "Germany");
    }

    #[test]
    fn test_parse_buyer_address_four_lines() {
        let addr = parse_buyer_address(Some("Acme Corp\n123 Main St\nBerlin, 10115\nGermany"));
        assert_eq!(addr.street, "Acme Corp, 123 Main St");
        assert_eq!(addr.city, "Berlin");
        assert_eq!(addr.postal_code, "10115");
        assert_eq!(addr.country, "Germany");
    }

    #[test]
    fn test_parse_buyer_address_trims_whitespace() {
        let addr = parse_buyer_address(Some("  123 Main St  \n  Berlin, 10115  \n  Germany  "));
        assert_eq!(addr.street, "123 Main St");
        assert_eq!(addr.city, "Berlin");
        assert_eq!(addr.postal_code, "10115");
        assert_eq!(addr.country, "Germany");
    }
}
