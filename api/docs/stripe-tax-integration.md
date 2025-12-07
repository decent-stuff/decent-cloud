# Stripe Tax Integration Notes

## Current Status

**Database:** Ready - Migration 040 adds tax tracking columns to `contract_sign_requests`
**Invoice Generation:** Ready - Tax is displayed on invoices when present
**Stripe Integration:** NOT IMPLEMENTED - See implementation requirements below

## Why Automatic Tax Is Not Yet Implemented

The current payment flow uses **Stripe Payment Intents** with Stripe Elements for card input. Stripe's `automatic_tax` feature is only available for:

1. **Checkout Sessions** - Hosted payment page
2. **Invoices** - Stripe Billing invoices
3. **Tax Calculation API** - Manual tax calculation (complex, requires separate API calls)

### Current Implementation

```rust
// api/src/stripe_client.rs - Payment Intent creation
pub async fn create_payment_intent(&self, amount: i64, currency: &str) -> Result<(String, String)> {
    let mut params = CreatePaymentIntent::new(amount, currency);
    params.automatic_payment_methods = Some(...);
    // NO automatic_tax field available for Payment Intents
    PaymentIntent::create(&self.client, params).await
}
```

## Implementation Options

### Option A: Migrate to Checkout Sessions (Recommended)

**Pros:**
- Automatic tax calculation with single parameter: `automatic_tax: { enabled: true }`
- Billing address collection built-in
- VAT ID collection built-in
- Tax details automatically available in webhook

**Cons:**
- Requires frontend refactor (remove Stripe Elements, redirect to Checkout)
- Different user experience (leaves site, then returns)

**Implementation:**
```rust
// Create Checkout Session instead of Payment Intent
let session = Session::create(&self.client, CreateSession {
    line_items: vec![...],
    automatic_tax: Some(CreateSessionAutomaticTax {
        enabled: true,
        liability: None,
    }),
    mode: CheckoutSessionMode::Payment,
    success_url: "...",
    cancel_url: "...",
    ..Default::default()
}).await?;

// In webhook: checkout.session.completed
let session = event.data.object; // CheckoutSession
let tax_amount = session.total_details.amount_tax;
let tax_rate = session.total_details.tax_breakdown[0].tax_rate_details.percentage_decimal;
```

### Option B: Tax Calculation API (Complex)

**Pros:**
- Keep current Payment Intent flow
- Full control over tax calculation

**Cons:**
- Requires multiple API calls per payment
- Must collect billing address manually
- Must manage tax calculation lifecycle
- More complex error handling

**Implementation:**
```rust
// 1. Calculate tax (separate API call)
let tax_calc = TaxCalculation::create(&self.client, CreateTaxCalculation {
    currency: currency.clone(),
    line_items: vec![...],
    customer_details: CreateTaxCalculationCustomerDetails {
        address: customer_address,
        address_source: Some(AddressSource::Billing),
        ..Default::default()
    },
    ..Default::default()
}).await?;

// 2. Create Payment Intent with calculated amount
let total_amount = base_amount + tax_calc.tax_amount_exclusive;
let payment_intent = PaymentIntent::create(&self.client, CreatePaymentIntent {
    amount: total_amount,
    currency: currency.clone(),
    metadata: [
        ("tax_calculation_id", tax_calc.id.to_string())
    ].into(),
    ..Default::default()
}).await?;

// 3. Link tax calculation to payment intent (after success)
// This happens automatically if metadata.tax_calculation is set
```

### Option C: Manual Tax Entry (Current Fallback)

**Status:** IMPLEMENTED
**Approach:** Tax columns exist in database, can be populated manually or by future integration

## Recommended Next Steps

1. **For MVP:** Use Option C (manual tax entry), add note on invoices for ICPay
2. **For Production:** Implement Option A (Checkout Sessions) for full Stripe Tax automation

## Stripe Dashboard Configuration Required

Regardless of implementation option, you MUST:

1. Enable Stripe Tax in Dashboard: https://dashboard.stripe.com/tax
2. Register for tax in applicable jurisdictions
3. Configure tax behavior (inclusive vs exclusive pricing)
4. Set up tax reporting preferences

Without these steps, automatic tax will fail even with correct API implementation.

## Current Behavior

- **Stripe Payments:** Tax columns are NULL, invoice shows 0% VAT
- **ICPay Payments:** Tax columns are NULL, invoice note: "Tax not included"
- **Manual Entry:** Admin can set tax fields directly in database

## References

- Stripe Tax Documentation: https://docs.stripe.com/tax
- Checkout Sessions with Tax: https://docs.stripe.com/tax/checkout
- Tax Calculation API: https://docs.stripe.com/tax/payment-intent
