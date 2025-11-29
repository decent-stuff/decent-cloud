# Stripe E2E Testing Setup

> **Prerequisites**: Complete [Stripe Configuration](../../../docs/development.md#stripe-configuration) first.

## Quick Start

### 1. Start Both Servers

```bash
# Terminal 1: API Server
cd api && cargo run --bin api-server  # http://localhost:59001

# Terminal 2: Website
cd website && npm run dev  # http://localhost:59000
```

### 2. Verify Test Data

```bash
# Ensure offerings exist
curl http://localhost:59001/api/v1/offerings | jq '.data | length'
```

## Running the E2E Tests

### Run All Payment Flow Tests
```bash
cd website
npx playwright test tests/e2e/payment-flows.spec.ts
```

### Run Specific Test
```bash
# DCT payment only
npx playwright test tests/e2e/payment-flows.spec.ts -g "DCT Payment"

# Stripe success flow only
npx playwright test tests/e2e/payment-flows.spec.ts -g "Stripe Payment Success"

# Stripe failure flow only
npx playwright test tests/e2e/payment-flows.spec.ts -g "Stripe Payment Failure"
```

### Run with UI (Headed Mode)
```bash
npx playwright test tests/e2e/payment-flows.spec.ts --headed
```

### Debug Mode
```bash
npx playwright test tests/e2e/payment-flows.spec.ts --debug
```

## How the Tests Work

### 1. DCT Payment Test
- Creates contract with DCT payment method
- Verifies `payment_status="succeeded"` immediately
- Contract stays in `status="requested"` (requires manual provider acceptance)

### 2. Stripe Success Test
1. User fills card details in Stripe Elements iframe
2. Submits rental request
3. Backend creates Stripe PaymentIntent
4. Frontend calls `stripe.confirmCardPayment()` with test card
5. **Stripe actually processes the payment** (using real test card)
6. Contract created with `payment_status="pending"`
7. **Test SIMULATES webhook** (see note below) - sends signed POST to `/api/v1/webhooks/stripe`
8. Webhook handler updates `payment_status="succeeded"` and auto-accepts contract
9. Verifies `status="accepted"`

**Note**: Tests simulate webhooks (signed POST to local endpoint) since Stripe can't reach localhost.

### 3. Stripe Failure Test
1. User fills declined test card (`4000 0000 0000 0002`)
2. Stripe declines payment (real API call)
3. User sees error: "Your card was declined"
4. Dialog stays open for retry

## Troubleshooting

### Test fails to find offering
```bash
curl http://localhost:59001/api/v1/offerings | jq
# Should return offerings from migration 008
```

### Webhook simulation fails
Ensure `STRIPE_WEBHOOK_SECRET=whsec_test_secret` in `api/.env`

### Card element doesn't appear
- Verify `VITE_STRIPE_PUBLISHABLE_KEY` is set in `website/.env`
- Restart dev server after changing env vars
- Check browser console for errors
