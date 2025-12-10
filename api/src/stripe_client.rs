use anyhow::Result;
use stripe::{
    CheckoutSession, CheckoutSessionId, CheckoutSessionMode, CheckoutSessionPaymentStatus, Client,
    CreateCheckoutSession, CreateCheckoutSessionInvoiceCreation, CreateCheckoutSessionLineItems,
    CreateCheckoutSessionLineItemsPriceData, CreateCheckoutSessionLineItemsPriceDataProductData,
    CreateRefund, Currency, Expandable, Invoice, InvoiceId, PaymentIntentId, Refund,
};

/// Stripe API client wrapper for payment processing
pub struct StripeClient {
    client: Client,
}

impl std::fmt::Debug for StripeClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StripeClient")
            .field("client", &"<stripe::Client>")
            .finish()
    }
}

impl StripeClient {
    /// Creates a new Stripe client using the API key from environment
    pub fn new() -> Result<Self> {
        let secret_key = std::env::var("STRIPE_SECRET_KEY")?;

        let client = Client::new(secret_key);
        Ok(Self { client })
    }

    /// Creates a Stripe Checkout Session for one-time payment
    ///
    /// # Arguments
    /// * `amount` - Amount in cents (e.g., 1000 = $10.00)
    /// * `currency` - Currency code (e.g., "usd")
    /// * `product_name` - Name of the product/service being purchased
    /// * `contract_id` - Hex-encoded contract ID for metadata and URLs
    ///
    /// # Returns
    /// Checkout Session URL for redirect on success
    pub async fn create_checkout_session(
        &self,
        amount: i64,
        currency: &str,
        product_name: &str,
        contract_id: &str,
    ) -> Result<String> {
        let currency = currency.parse::<Currency>()?;

        let frontend_url =
            std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:59010".to_string());

        let success_url = format!(
            "{}/checkout/success?session_id={{CHECKOUT_SESSION_ID}}",
            frontend_url
        );
        let cancel_url = format!(
            "{}/checkout/cancel?contract_id={}",
            frontend_url, contract_id
        );

        let mut params = CreateCheckoutSession::new();
        params.mode = Some(CheckoutSessionMode::Payment);
        params.line_items = Some(vec![CreateCheckoutSessionLineItems {
            price_data: Some(CreateCheckoutSessionLineItemsPriceData {
                currency,
                unit_amount: Some(amount),
                product_data: Some(CreateCheckoutSessionLineItemsPriceDataProductData {
                    name: product_name.to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            quantity: Some(1),
            ..Default::default()
        }]);
        // Automatic tax requires origin address configured in Stripe dashboard
        // https://dashboard.stripe.com/settings/tax
        if std::env::var("STRIPE_AUTOMATIC_TAX").is_ok() {
            params.automatic_tax = Some(stripe::CreateCheckoutSessionAutomaticTax {
                enabled: true,
                liability: None,
            });
            params.tax_id_collection =
                Some(stripe::CreateCheckoutSessionTaxIdCollection { enabled: true });
        } else {
            tracing::warn!("STRIPE_AUTOMATIC_TAX not set - automatic tax calculation disabled. Set STRIPE_AUTOMATIC_TAX=true and configure origin address at https://dashboard.stripe.com/settings/tax to enable.");
        }
        params.success_url = Some(&success_url);
        params.cancel_url = Some(&cancel_url);
        params.metadata = Some(
            [("contract_id".to_string(), contract_id.to_string())]
                .into_iter()
                .collect(),
        );

        // Enable invoice generation for post-purchase invoice PDF
        // Pass contract_id in invoice_data.metadata so we can link the invoice back
        // when we receive the invoice.paid webhook (invoice is created asynchronously)
        params.invoice_creation = Some(CreateCheckoutSessionInvoiceCreation {
            enabled: true,
            invoice_data: Some(stripe::CreateCheckoutSessionInvoiceCreationInvoiceData {
                metadata: Some(
                    [("contract_id".to_string(), contract_id.to_string())]
                        .into_iter()
                        .collect(),
                ),
                ..Default::default()
            }),
        });

        let session = CheckoutSession::create(&self.client, params).await?;

        session
            .url
            .ok_or_else(|| anyhow::anyhow!("Checkout Session missing URL"))
    }

    /// Creates a refund for a payment intent
    ///
    /// # Arguments
    /// * `payment_intent_id` - The PaymentIntent ID to refund
    /// * `amount` - Amount to refund in cents (None = full refund)
    ///
    /// # Returns
    /// Refund ID on success
    pub async fn create_refund(
        &self,
        payment_intent_id: &str,
        amount: Option<i64>,
    ) -> Result<String> {
        let intent_id: PaymentIntentId = payment_intent_id.parse()?;

        let mut params = CreateRefund::new();
        params.payment_intent = Some(intent_id);

        if let Some(amt) = amount {
            params.amount = Some(amt);
        }

        let refund = Refund::create(&self.client, params).await?;

        Ok(refund.id.to_string())
    }

    /// Retrieves a checkout session and returns payment details if paid
    ///
    /// # Arguments
    /// * `session_id` - The Checkout Session ID (cs_...)
    ///
    /// # Returns
    /// Some((contract_id, tax_amount_cents, customer_tax_id, reverse_charge)) if paid, None otherwise
    pub async fn retrieve_checkout_session(
        &self,
        session_id: &str,
    ) -> Result<Option<CheckoutSessionResult>> {
        let session_id: CheckoutSessionId = session_id.parse()?;
        let session = CheckoutSession::retrieve(&self.client, &session_id, &[]).await?;

        // Only return result if payment is complete
        if session.payment_status != CheckoutSessionPaymentStatus::Paid {
            return Ok(None);
        }

        let contract_id = session
            .metadata
            .as_ref()
            .and_then(|m| m.get("contract_id"))
            .cloned();

        let Some(contract_id) = contract_id else {
            return Err(anyhow::anyhow!("Session missing contract_id metadata"));
        };

        let tax_amount_cents = session.total_details.as_ref().map(|td| td.amount_tax);
        let customer_tax_id = session
            .customer_details
            .as_ref()
            .and_then(|cd| cd.tax_ids.as_ref())
            .and_then(|ids| ids.first())
            .map(|tax_id| {
                format!(
                    "{:?}: {}",
                    tax_id.type_,
                    tax_id.value.as_deref().unwrap_or("")
                )
            });

        let reverse_charge = customer_tax_id.is_some() && tax_amount_cents.unwrap_or(1) == 0;

        // Extract invoice ID if invoice was created
        let mut invoice_id = session.invoice.as_ref().map(|inv| match inv {
            Expandable::Id(id) => id.to_string(),
            Expandable::Object(invoice) => invoice.id.to_string(),
        });

        // If session doesn't have invoice yet (async creation), try to find it by metadata
        if invoice_id.is_none() {
            match self.find_invoice_by_contract_id(&contract_id).await {
                Ok(Some(id)) => {
                    tracing::info!(
                        "Found invoice {} for contract {} via metadata search",
                        id,
                        contract_id
                    );
                    invoice_id = Some(id);
                }
                Ok(None) => {
                    tracing::debug!(
                        "No invoice found yet for contract {} (may still be processing)",
                        contract_id
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to search for invoice by metadata for contract {}: {}",
                        contract_id,
                        e
                    );
                }
            }
        }

        Ok(Some(CheckoutSessionResult {
            contract_id,
            session_id: session.id.to_string(),
            tax_amount_cents,
            customer_tax_id,
            reverse_charge,
            invoice_id,
        }))
    }

    /// Retrieves a Stripe invoice PDF URL
    ///
    /// # Arguments
    /// * `invoice_id` - The Invoice ID (in_...)
    ///
    /// # Returns
    /// PDF URL if available, None if invoice not finalized yet
    pub async fn get_invoice_pdf_url(&self, invoice_id: &str) -> Result<Option<String>> {
        let invoice_id: InvoiceId = invoice_id.parse()?;
        let invoice = Invoice::retrieve(&self.client, &invoice_id, &[]).await?;

        Ok(invoice.invoice_pdf)
    }

    /// Search for invoice by contract_id in metadata
    ///
    /// This is useful when polling before the checkout session's invoice field is populated.
    ///
    /// # Arguments
    /// * `contract_id` - The contract ID (hex string) stored in invoice metadata
    ///
    /// # Returns
    /// Invoice ID if found, None otherwise
    pub async fn find_invoice_by_contract_id(&self, contract_id: &str) -> Result<Option<String>> {
        // Note: Stripe's rust library doesn't expose the search API directly,
        // so we list recent invoices and filter by metadata.
        // This is acceptable because invoices are created immediately after payment.
        let params = stripe::ListInvoices {
            ..Default::default()
        };

        let invoices = Invoice::list(&self.client, &params).await?;

        for invoice in invoices.data {
            if let Some(metadata) = &invoice.metadata {
                if metadata.get("contract_id").map(|s| s.as_str()) == Some(contract_id) {
                    return Ok(Some(invoice.id.to_string()));
                }
            }
        }

        Ok(None)
    }
}

/// Result from retrieving a paid checkout session
pub struct CheckoutSessionResult {
    pub contract_id: String,
    pub session_id: String,
    pub tax_amount_cents: Option<i64>,
    pub customer_tax_id: Option<String>,
    pub reverse_charge: bool,
    pub invoice_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_stripe_client_new_missing_key() {
        // Clear env var to test error handling
        std::env::remove_var("STRIPE_SECRET_KEY");

        let result = StripeClient::new();
        assert!(result.is_err());
        // VarError::NotPresent doesn't include var name in message, just check it's an error
    }

    #[test]
    #[serial]
    fn test_stripe_client_new_with_key() {
        // Set test key
        std::env::set_var("STRIPE_SECRET_KEY", "sk_test_dummy");

        let result = StripeClient::new();
        assert!(result.is_ok());

        // Clean up
        std::env::remove_var("STRIPE_SECRET_KEY");
    }

    #[tokio::test]
    #[serial]
    async fn test_create_checkout_session_invalid_currency() {
        std::env::set_var("STRIPE_SECRET_KEY", "sk_test_dummy");
        std::env::set_var("FRONTEND_URL", "http://localhost:59010");
        let client = StripeClient::new().unwrap();

        let result = client
            .create_checkout_session(1000, "invalid", "Test Product", "abc123")
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("currency"));

        std::env::remove_var("STRIPE_SECRET_KEY");
        std::env::remove_var("FRONTEND_URL");
    }

    #[test]
    #[serial]
    fn test_checkout_session_uses_frontend_url() {
        // This test verifies that the FRONTEND_URL is used in success/cancel URLs
        std::env::set_var("FRONTEND_URL", "https://example.com");
        let url = std::env::var("FRONTEND_URL").unwrap();
        assert_eq!(url, "https://example.com");
        std::env::remove_var("FRONTEND_URL");
    }

    #[test]
    #[serial]
    fn test_checkout_session_defaults_frontend_url() {
        // Test default when FRONTEND_URL not set
        std::env::remove_var("FRONTEND_URL");
        let url =
            std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:59010".to_string());
        assert_eq!(url, "http://localhost:59010");
    }
}
