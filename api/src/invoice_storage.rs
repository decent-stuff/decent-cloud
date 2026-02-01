//! File-based invoice PDF storage
//!
//! Stores invoice PDFs on disk in the data directory (alongside ledger data).
//! Structure: {LEDGER_DIR}/invoices/{contract_id_hex}/{type}.pdf
//!
//! Types:
//! - stripe.pdf: Invoice PDF from Stripe
//! - typst.pdf: Locally generated invoice using Typst

use crate::ledger_path::ledger_dir_path;
use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::fs;

const INVOICES_DIR: &str = "invoices";

/// Invoice PDF type
#[derive(Debug, Clone, Copy)]
pub enum InvoiceType {
    Stripe,
    Typst,
}

impl InvoiceType {
    fn filename(&self) -> &'static str {
        match self {
            Self::Stripe => "stripe.pdf",
            Self::Typst => "typst.pdf",
        }
    }
}

/// Get the invoices directory path
fn invoices_dir() -> Result<PathBuf> {
    let ledger_dir = ledger_dir_path().context("Failed to get ledger directory")?;
    Ok(ledger_dir.join(INVOICES_DIR))
}

/// Get the directory for a specific contract's invoices
fn contract_invoice_dir(contract_id: &[u8]) -> Result<PathBuf> {
    let invoices = invoices_dir()?;
    Ok(invoices.join(hex::encode(contract_id)))
}

/// Get the full path for an invoice PDF
fn invoice_path(contract_id: &[u8], invoice_type: InvoiceType) -> Result<PathBuf> {
    let dir = contract_invoice_dir(contract_id)?;
    Ok(dir.join(invoice_type.filename()))
}

/// Save an invoice PDF to disk
pub async fn save_invoice_pdf(
    contract_id: &[u8],
    invoice_type: InvoiceType,
    pdf_bytes: &[u8],
) -> Result<()> {
    let dir = contract_invoice_dir(contract_id)?;
    fs::create_dir_all(&dir).await.context(format!(
        "Failed to create invoice directory {}",
        dir.display()
    ))?;

    let path = dir.join(invoice_type.filename());
    fs::write(&path, pdf_bytes)
        .await
        .context(format!("Failed to write invoice PDF to {}", path.display()))?;

    tracing::debug!(
        "Saved {:?} invoice for contract {} ({} bytes)",
        invoice_type,
        hex::encode(contract_id),
        pdf_bytes.len()
    );

    Ok(())
}

/// Load an invoice PDF from disk
pub async fn load_invoice_pdf(
    contract_id: &[u8],
    invoice_type: InvoiceType,
) -> Result<Option<Vec<u8>>> {
    let path = invoice_path(contract_id, invoice_type)?;

    if !path.exists() {
        return Ok(None);
    }

    let bytes = fs::read(&path).await.context(format!(
        "Failed to read invoice PDF from {}",
        path.display()
    ))?;

    Ok(Some(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    fn setup_test_env() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("LEDGER_DIR", temp_dir.path());
        temp_dir
    }

    #[tokio::test]
    #[serial]
    async fn test_save_and_load_typst_invoice() {
        let _temp = setup_test_env();
        let contract_id = vec![1u8; 32];
        let pdf_content = b"fake pdf content for typst";

        save_invoice_pdf(&contract_id, InvoiceType::Typst, pdf_content)
            .await
            .unwrap();

        let loaded = load_invoice_pdf(&contract_id, InvoiceType::Typst)
            .await
            .unwrap();
        assert_eq!(loaded, Some(pdf_content.to_vec()));
    }

    #[tokio::test]
    #[serial]
    async fn test_save_and_load_stripe_invoice() {
        let _temp = setup_test_env();
        let contract_id = vec![2u8; 32];
        let pdf_content = b"fake stripe invoice pdf";

        save_invoice_pdf(&contract_id, InvoiceType::Stripe, pdf_content)
            .await
            .unwrap();

        let loaded = load_invoice_pdf(&contract_id, InvoiceType::Stripe)
            .await
            .unwrap();
        assert_eq!(loaded, Some(pdf_content.to_vec()));
    }

    #[tokio::test]
    #[serial]
    async fn test_load_nonexistent_invoice() {
        let _temp = setup_test_env();
        let contract_id = vec![3u8; 32];

        let loaded = load_invoice_pdf(&contract_id, InvoiceType::Stripe)
            .await
            .unwrap();
        assert_eq!(loaded, None);
    }

    #[tokio::test]
    #[serial]
    async fn test_both_invoice_types_coexist() {
        let _temp = setup_test_env();
        let contract_id = vec![15u8; 32];
        let stripe_pdf = b"stripe invoice";
        let typst_pdf = b"typst invoice";

        save_invoice_pdf(&contract_id, InvoiceType::Stripe, stripe_pdf)
            .await
            .unwrap();
        save_invoice_pdf(&contract_id, InvoiceType::Typst, typst_pdf)
            .await
            .unwrap();

        let loaded_stripe = load_invoice_pdf(&contract_id, InvoiceType::Stripe)
            .await
            .unwrap();
        let loaded_typst = load_invoice_pdf(&contract_id, InvoiceType::Typst)
            .await
            .unwrap();

        assert_eq!(loaded_stripe, Some(stripe_pdf.to_vec()));
        assert_eq!(loaded_typst, Some(typst_pdf.to_vec()));
    }
}
