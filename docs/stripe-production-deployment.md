# Stripe Production Deployment Guide

This guide explains how to deploy Stripe payment integration to production.

## Prerequisites

1. **Stripe Account** (if you don't have one):
   - Sign up at https://stripe.com
   - Complete business verification
   - Activate live mode

2. **Get Production Keys**:
   - Go to https://dashboard.stripe.com/apikeys (ensure you're in LIVE mode, not test)
   - Copy your **Publishable key** (`pk_live_...`)
   - Reveal and copy your **Secret key** (`sk_live_...`)
   - **⚠️ NEVER commit these to git!**

3. **Configure Production Webhook**:
   - Go to https://dashboard.stripe.com/webhooks
   - Click **Add endpoint**
   - Endpoint URL: `https://your-domain.com/api/v1/webhooks/stripe`
   - Select events to listen to:
     - `payment_intent.succeeded`
     - `payment_intent.payment_failed`
   - Click **Add endpoint**
   - Reveal and copy the **Signing secret** (`whsec_...`)

## Deployment Steps

### 1. Configure Environment Variables

Create or update the environment config file for your target environment:

**For production (`cf/.env.prod`)**:
```bash
# Copy example and edit
cd cf
cp .env.example .env.prod

# Edit .env.prod and add your LIVE Stripe keys:
export STRIPE_SECRET_KEY=sk_live_YOUR_SECRET_KEY
export STRIPE_PUBLISHABLE_KEY=pk_live_YOUR_PUBLISHABLE_KEY
export STRIPE_WEBHOOK_SECRET=whsec_YOUR_WEBHOOK_SECRET
```

**For development (`cf/.env.dev`)**:
```bash
# Edit .env.dev and add your TEST Stripe keys:
export STRIPE_SECRET_KEY=sk_test_YOUR_SECRET_KEY
export STRIPE_PUBLISHABLE_KEY=pk_test_YOUR_PUBLISHABLE_KEY
export STRIPE_WEBHOOK_SECRET=whsec_test_secret
```

### 2. Deploy with deploy.py

The deploy script **automatically**:
- Reads Stripe keys from `.env.{environment}` file
- Embeds `VITE_STRIPE_PUBLISHABLE_KEY` into website build
- Validates Stripe key format before building
- Passes API keys to docker-compose

```bash
cd cf

# Deploy to production
python3 deploy.py deploy prod

# Or deploy to dev/staging
python3 deploy.py deploy dev
```

**What happens**:
1. Script reads `STRIPE_PUBLISHABLE_KEY` from `.env.prod` (or `.env.dev`)
2. Creates `website/.env.local` with `VITE_STRIPE_PUBLISHABLE_KEY=$STRIPE_PUBLISHABLE_KEY`
3. Runs `npm run build` which:
   - Validates Stripe key format (warns if missing, fails if invalid)
   - Embeds key into JavaScript bundle
4. Passes `STRIPE_SECRET_KEY` and `STRIPE_WEBHOOK_SECRET` to docker-compose for API services

**Important**: The `VITE_STRIPE_PUBLISHABLE_KEY` is embedded in the JavaScript bundle at build time. It's not a secret (it's visible in browser), but you should use your **live** publishable key for production.

### 3. Verify Webhook Configuration

Test that Stripe webhooks reach your server:

```bash
# Install Stripe CLI (one-time)
brew install stripe/stripe-cli/stripe

# Login to your Stripe account
stripe login

# Test webhook delivery to production
stripe trigger payment_intent.succeeded --forward-to https://your-domain.com/api/v1/webhooks/stripe
```

Check your API logs for:
```
[INFO] Received Stripe webhook: payment_intent.succeeded
[INFO] Payment succeeded: pi_xxx
```

### 4. Test Payment Flow

1. Navigate to your production marketplace
2. Click **Rent Resource** on any offering
3. Select **Credit Card** payment method
4. Use a Stripe test card (even in live mode, you can test):
   - Test card: `4242 4242 4242 4242`
   - Any future expiry (e.g., `12/34`)
   - Any 3-digit CVC (e.g., `123`)
5. Verify:
   - Payment processes successfully
   - Contract auto-accepts
   - Webhook is received

**⚠️ Important**: Test cards work in live mode for testing, but real customers will use real cards.

## Environment Variable Summary

### Build-time (website)
Set BEFORE running `npm run build`:

| Variable | Value | Used For |
|----------|-------|----------|
| `VITE_STRIPE_PUBLISHABLE_KEY` | `pk_live_...` | Stripe.js initialization in browser |

### Runtime (docker-compose)
Set BEFORE running `docker-compose up`:

| Variable | Value | Used For |
|----------|-------|----------|
| `STRIPE_SECRET_KEY` | `sk_live_...` | API server - creating PaymentIntents |
| `STRIPE_PUBLISHABLE_KEY` | `pk_live_...` | (Same as VITE version, used by API) |
| `STRIPE_WEBHOOK_SECRET` | `whsec_...` | API server - verifying webhooks |

## Security Checklist

- [ ] **Never commit** Stripe secret keys to git
- [ ] Use **live keys** (`pk_live_...`, `sk_live_...`) in production
- [ ] Use **test keys** (`pk_test_...`, `sk_test_...`) in dev/staging
- [ ] Configure **webhook endpoint** in Stripe Dashboard
- [ ] Verify **webhook signature** is checked (already implemented)
- [ ] Test **payment flow** end-to-end before going live
- [ ] Monitor **Stripe Dashboard** for payment errors

## Troubleshooting

### "Stripe is not defined" in browser
**Problem**: Website built without `VITE_STRIPE_PUBLISHABLE_KEY`

**Solution**: Rebuild website with env var set:
```bash
export VITE_STRIPE_PUBLISHABLE_KEY=pk_live_...
npm run build
```

### Webhooks not received
**Problem**: Stripe can't reach your webhook endpoint

**Check**:
1. Webhook URL is correct in Stripe Dashboard
2. HTTPS is properly configured (Stripe requires HTTPS)
3. Firewall allows Stripe's IP ranges
4. Check Stripe Dashboard → Webhooks → Endpoint → Event Log

**Test**: Use Stripe CLI to trigger test webhook:
```bash
stripe trigger payment_intent.succeeded --forward-to https://your-domain.com/api/v1/webhooks/stripe
```

### Payment succeeds but contract doesn't auto-accept
**Problem**: Webhook signature verification failing or webhook not configured

**Check API logs for**:
```
[ERROR] Webhook signature verification failed
[WARN] Contract not found for payment_intent_id: pi_xxx
```

**Solution**: Verify `STRIPE_WEBHOOK_SECRET` matches the signing secret in Stripe Dashboard

### Using wrong keys (test vs live)
**Problem**: Test keys in production or live keys in development

**How to identify**:
- Test keys start with: `pk_test_...`, `sk_test_...`
- Live keys start with: `pk_live_...`, `sk_live_...`

**Fix**: Update environment variables with correct keys for the environment

## Monitoring

### Stripe Dashboard
Monitor payments in real-time:
- https://dashboard.stripe.com/payments
- https://dashboard.stripe.com/webhooks

### API Logs
Watch for payment-related log messages:
```bash
docker logs -f decent-cloud-api-serve-prod | grep -i "stripe\|payment"
```

## Rollback Plan

If payments fail in production:

1. **Immediate**: Disable Stripe by unsetting env vars and restarting API:
   ```bash
   unset STRIPE_SECRET_KEY
   unset STRIPE_WEBHOOK_SECRET
   docker-compose restart api-serve
   ```
   This disables credit card payments; DCT payments still work.

2. **Investigate**: Check Stripe Dashboard and API logs

3. **Fix and Redeploy**: Correct the issue and redeploy

## Support

- **Stripe Documentation**: https://docs.stripe.com
- **Stripe Support**: https://support.stripe.com
- **Stripe Status**: https://status.stripe.com

## Next Steps

Once Stripe is working in production:
- Monitor payment success rate
- Set up Stripe email notifications for failed payments
- Consider adding Stripe Radar for fraud detection
- Review Stripe fees and optimize currency handling
