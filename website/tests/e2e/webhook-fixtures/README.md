# Stripe Webhook Test Fixtures

This directory contains reference webhook payloads for testing.

## Purpose

Maintain real Stripe webhook examples to:
1. Ensure simulated webhooks match production format
2. Catch breaking changes in Stripe's webhook structure
3. Provide reference for test updates

## Usage

When updating webhook simulation in `payment-flows.spec.ts`, compare against these fixtures to ensure compatibility.

## Obtaining Real Webhook Payloads

### Method 1: Stripe CLI
```bash
stripe listen --print-json > webhook-events.json
stripe trigger payment_intent.succeeded
```

### Method 2: Stripe Dashboard
1. Go to **Developers → Webhooks → [Your endpoint]**
2. Click on any event in the event log
3. Copy the raw JSON payload

### Method 3: Production Logs
Check your API server logs for actual webhook payloads received in production.

## Verification Process

When Stripe updates their API:

1. Capture new webhook payload using one of the methods above
2. Update fixture files
3. Compare with simulation in `payment-flows.spec.ts` line 52-78
4. Update simulation if structure changed
5. Run tests to verify: `npx playwright test tests/e2e/payment-flows.spec.ts`

## Current Webhook Handler Dependencies

The webhook handler (`api/src/openapi/webhooks.rs`) currently only reads:
- `type` → Event type string
- `data.object.id` → Payment intent ID

If this changes, update both:
1. The webhook simulation in tests
2. These fixture files
