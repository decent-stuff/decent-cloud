use crate::database::email::EmailType;
use crate::database::Database;
use crate::invoices;
use anyhow::{Context, Result};
use email_utils::{EmailAttachment, EmailService};
use std::sync::Arc;

/// Send payment receipt email after successful payment
/// If email_service is provided, sends directly with invoice PDF attached.
/// Otherwise, queues plain text receipt for reliable delivery.
/// Returns the receipt number assigned.
pub async fn send_payment_receipt(
    db: &Database,
    contract_id: &[u8],
    email_service: Option<&Arc<EmailService>>,
) -> Result<i64> {
    // Get contract details
    let contract_hex = hex::encode(contract_id);
    let contract = db
        .get_contract(contract_id)
        .await?
        .context("Contract not found")?;

    // Get next receipt number atomically
    let receipt_number = get_next_receipt_number(db).await?;

    // Update contract with receipt number and sent timestamp
    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    update_contract_receipt_info(db, contract_id, receipt_number, now_ns).await?;

    // Format receipt email
    let subject = format!("Receipt for your Decent Cloud rental - #{}", receipt_number);

    // Format amount (e9s to decimal)
    let amount = format!(
        "{:.2}",
        contract.payment_amount_e9s as f64 / 1_000_000_000.0
    );

    // Format payment method for display
    let payment_method_display = match contract.payment_method.as_str() {
        "stripe" => "Credit Card (Stripe)",
        "icpay" => "Cryptocurrency (ICPay)",
        "dct" => "DCT Token",
        other => other,
    };

    // Transaction ID
    let transaction_id = contract
        .stripe_payment_intent_id
        .or(contract.icpay_payment_id)
        .unwrap_or_else(|| "N/A".to_string());

    // Format duration
    let duration = contract.duration_hours.unwrap_or(0);

    // Format dates
    let start_date = contract
        .start_timestamp_ns
        .map(|ts| {
            chrono::DateTime::from_timestamp(ts / 1_000_000_000, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
                .unwrap_or_else(|| "N/A".to_string())
        })
        .unwrap_or_else(|| "N/A".to_string());

    let end_date = contract
        .end_timestamp_ns
        .map(|ts| {
            chrono::DateTime::from_timestamp(ts / 1_000_000_000, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
                .unwrap_or_else(|| "N/A".to_string())
        })
        .unwrap_or_else(|| "N/A".to_string());

    let receipt_date = chrono::Utc::now().format("%Y-%m-%d").to_string();

    // TODO: Get offering name and provider name from database
    // For now use placeholders - will be implemented when needed
    let offering_name = format!("Offering {}", contract.offering_id);
    let provider_name = "Provider"; // Will fetch from provider_profiles table

    // Try to generate invoice PDF if email service is available
    let invoice_pdf = if email_service.is_some() {
        match invoices::get_invoice_pdf(db, contract_id).await {
            Ok(pdf) => Some(pdf),
            Err(e) => {
                tracing::warn!(
                    "Failed to generate invoice PDF for contract {}, will send receipt without attachment: {}",
                    contract_hex,
                    e
                );
                None
            }
        }
    } else {
        None
    };

    // Get invoice number for the email body
    let invoice_number = match invoices::get_invoice_metadata(db, contract_id).await {
        Ok(inv) => Some(inv.invoice_number),
        Err(_) => None,
    };

    // Adjust footer based on whether invoice is attached
    let footer = if invoice_pdf.is_some() {
        format!(
            "Your tax invoice ({}) is attached to this email.",
            invoice_number.as_deref().unwrap_or("N/A")
        )
    } else {
        "For a tax invoice, visit your dashboard or contact support.".to_string()
    };

    let body = format!(
        r#"Receipt #{receipt_number}
Date: {receipt_date}

Thank you for your payment!

PAYMENT DETAILS
───────────────────────────────────
Amount Paid:     {amount} {currency}
Payment Method:  {payment_method}
Transaction ID:  {transaction_id}

CONTRACT DETAILS
───────────────────────────────────
Offering:        {offering_name}
Provider:        {provider_name}
Duration:        {duration} hours
Start Date:      {start_date}
End Date:        {end_date}
Contract ID:     {contract_id}

View your rentals: https://decent-cloud.org/dashboard/rentals

───────────────────────────────────
{footer}

Decent Cloud
"#,
        receipt_number = receipt_number,
        receipt_date = receipt_date,
        amount = amount,
        currency = contract.currency,
        payment_method = payment_method_display,
        transaction_id = transaction_id,
        offering_name = offering_name,
        provider_name = provider_name,
        duration = duration,
        start_date = start_date,
        end_date = end_date,
        contract_id = contract_hex,
        footer = footer,
    );

    let from_addr = "noreply@decent-cloud.org";

    // Send with attachment if we have both email service and PDF
    if let (Some(service), Some(pdf)) = (email_service, invoice_pdf) {
        let invoice_num = invoice_number.as_deref().unwrap_or("invoice");
        let attachment = EmailAttachment {
            content_type: "application/pdf".to_string(),
            filename: format!("{}.pdf", invoice_num),
            content: pdf,
        };

        service
            .send_email_with_attachments(
                from_addr,
                &contract.requester_contact,
                &subject,
                &body,
                false,
                Some(&[attachment]),
            )
            .await
            .context("Failed to send receipt email with invoice")?;
    } else {
        // Fall back to queuing plain text receipt
        db.queue_email(
            &contract.requester_contact,
            from_addr,
            &subject,
            &body,
            false,
            EmailType::General,
        )
        .await
        .context("Failed to queue receipt email")?;
    }

    tracing::info!(
        "Receipt #{} sent to {} for contract {}",
        receipt_number,
        contract.requester_contact,
        contract_hex
    );

    Ok(receipt_number)
}

/// Get next receipt number atomically
async fn get_next_receipt_number(db: &Database) -> Result<i64> {
    // Use SQLite's UPDATE RETURNING to atomically get and increment
    let row = sqlx::query!(
        "UPDATE receipt_sequence SET next_number = next_number + 1 WHERE id = 1 RETURNING next_number - 1 as receipt_number"
    )
    .fetch_one(&db.pool)
    .await?;

    Ok(row.receipt_number)
}

/// Update contract with receipt number and timestamp
async fn update_contract_receipt_info(
    db: &Database,
    contract_id: &[u8],
    receipt_number: i64,
    sent_at_ns: i64,
) -> Result<()> {
    sqlx::query!(
        "UPDATE contract_sign_requests SET receipt_number = ?, receipt_sent_at_ns = ? WHERE contract_id = ?",
        receipt_number,
        sent_at_ns,
        contract_id
    )
    .execute(&db.pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_helpers::setup_test_db;

    #[tokio::test]
    async fn test_get_next_receipt_number_sequential() {
        let db = setup_test_db().await;

        let num1 = get_next_receipt_number(&db).await.unwrap();
        let num2 = get_next_receipt_number(&db).await.unwrap();
        let num3 = get_next_receipt_number(&db).await.unwrap();

        assert_eq!(num1, 1);
        assert_eq!(num2, 2);
        assert_eq!(num3, 3);
    }

    #[tokio::test]
    async fn test_get_next_receipt_number_atomic() {
        use std::sync::Arc;
        let db = Arc::new(setup_test_db().await);

        // Simulate concurrent requests
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let db_clone = Arc::clone(&db);
                tokio::spawn(async move { get_next_receipt_number(&db_clone).await.unwrap() })
            })
            .collect();

        let mut numbers: Vec<i64> = Vec::new();
        for handle in handles {
            numbers.push(handle.await.unwrap());
        }

        // All numbers should be unique (no duplicates)
        numbers.sort();
        let mut unique_numbers = numbers.clone();
        unique_numbers.dedup();
        assert_eq!(
            numbers.len(),
            unique_numbers.len(),
            "Receipt numbers must be unique"
        );

        // Should be continuous sequence (no gaps)
        for i in 0..numbers.len() {
            assert_eq!(numbers[i], (i + 1) as i64);
        }
    }

    #[tokio::test]
    async fn test_update_contract_receipt_info() {
        let db = setup_test_db().await;

        // Create test contract
        let contract_id = vec![1u8; 32];
        let requester_pk = vec![2u8; 32];
        let provider_pk = vec![3u8; 32];

        sqlx::query!(
            r#"INSERT INTO contract_sign_requests
               (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact,
                provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns,
                payment_method, payment_status, currency)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            contract_id,
            requester_pk,
            "ssh-key",
            "test@example.com",
            provider_pk,
            "off-1",
            1000000000i64,
            "test",
            0i64,
            "stripe",
            "succeeded",
            "USD"
        )
        .execute(&db.pool)
        .await
        .unwrap();

        // Update receipt info
        let receipt_number = 42;
        let sent_at_ns = 123456789;
        update_contract_receipt_info(&db, &contract_id, receipt_number, sent_at_ns)
            .await
            .unwrap();

        // Verify update
        #[derive(sqlx::FromRow)]
        struct ReceiptInfo {
            receipt_number: Option<i64>,
            receipt_sent_at_ns: Option<i64>,
        }

        let row: ReceiptInfo = sqlx::query_as(
            "SELECT receipt_number, receipt_sent_at_ns FROM contract_sign_requests WHERE contract_id = ?"
        )
        .bind(&contract_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();

        assert_eq!(row.receipt_number, Some(receipt_number));
        assert_eq!(row.receipt_sent_at_ns, Some(sent_at_ns));
    }
}
