# Email Testing Guide

## Pre-Production Testing Checklist

This guide walks you through testing all email functionality before production deployment.

## Prerequisites

1. MailChannels API key (from https://app.mailchannels.com/)
2. DKIM private key (base64 encoded)
3. DNS access to add DKIM TXT records

## Setup

### 1. Configure Environment Variables

Create `api/.env` from `api/.env.example`:

```bash
cd api
cp .env.example .env
```

Edit `.env` and set:

```bash
# Required
MAILCHANNELS_API_KEY=your_actual_api_key_here
FRONTEND_URL=http://localhost:59000  # or your staging URL

# Optional but recommended for production
DKIM_DOMAIN=decentcloud.org
DKIM_SELECTOR=mailchannels
DKIM_PRIVATE_KEY=your_base64_encoded_private_key_here
```

### 2. Add DKIM DNS Records

If using DKIM, add these TXT records to your domain DNS:

```
mailchannels._domainkey.decentcloud.org TXT "v=DKIM1; k=rsa; p=<your_public_key>"
```

*Note: The public key format depends on your key generation method.*

## Test 1: Basic Email Sending

Test that MailChannels API key is working:

```bash
cd /home/sat/projects/decent-cloud
cargo run --bin api-server -- test-email --to your@email.com
```

**Expected output:**
```
========================================
  Email Configuration Test
========================================

✓ MailChannels API key found
✓ DKIM signing: disabled (use --with-dkim to enable)

Sending test email...
  From: noreply@decentcloud.org
  To: your@email.com
  Subject: Decent Cloud Email Test

✅ SUCCESS! Test email sent successfully.

Please check your inbox at: your@email.com
```

**Verify:**
- [ ] Email arrives in inbox (check spam folder)
- [ ] From address shows correctly
- [ ] Email content is readable

## Test 2: DKIM Email Signing

Test that DKIM signing works:

```bash
cargo run --bin api-server -- test-email --to your@email.com --with-dkim
```

**Expected output:**
```
✓ DKIM configuration found:
  - Domain: decentcloud.org
  - Selector: mailchannels
  - Private key: MIIEvgIBA...BAQEF (XXX bytes)

...

✅ SUCCESS! Test email sent successfully.

🔒 DKIM Configuration Test:
  - Check email headers for 'DKIM-Signature' field
  - Verify signature shows as valid in your email client
  - Run online DKIM checker tools to validate signature
```

**Verify:**
- [ ] Email arrives with DKIM signature in headers
- [ ] DKIM signature validates successfully
- [ ] Use https://www.mail-tester.com/ to verify DKIM

## Test 3: Welcome Email Flow

Test that welcome emails are sent when creating accounts:

### OAuth Registration
1. Start the API server:
   ```bash
   cargo run --bin api-server -- serve
   ```

2. Complete OAuth registration via the frontend
3. Check database for queued email:
   ```bash
   sqlite3 api/test.db "SELECT subject, to_addr, status FROM email_queue ORDER BY created_at DESC LIMIT 1"
   ```

**Verify:**
- [ ] Welcome email is queued with `email_type='welcome'`
- [ ] Email status is 'pending'
- [ ] Recipient address matches the registered user's email

### Check Email Processor
The email processor runs every 30 seconds by default. Watch logs:

```bash
# In the server logs, you should see:
"Starting email processor (interval: 30s, batch: 10)"
"Sent email to user@example.com (subject: Welcome to Decent Cloud, type: welcome)"
```

**Verify:**
- [ ] Email processor starts successfully
- [ ] Welcome email is sent within 30 seconds
- [ ] User receives welcome email

## Test 4: Account Recovery Flow

### Option A: Automated Test Script

Run the recovery flow test script:

```bash
./scripts/test-account-recovery.sh test@example.com
```

This will:
1. Request a recovery token
2. Extract the token from the database
3. Complete recovery with a new key

### Option B: Manual Testing

1. **Request Recovery:**
   ```bash
   curl -X POST http://localhost:59001/api/v1/accounts/recovery/request \
     -H "Content-Type: application/json" \
     -d '{"email": "user@example.com"}'
   ```

2. **Check Email Queue:**
   ```bash
   sqlite3 api/test.db "SELECT subject, email_type, status FROM email_queue WHERE email_type='recovery' ORDER BY created_at DESC LIMIT 1"
   ```

3. **Get Token from Database** (simulating email click):
   ```bash
   sqlite3 api/test.db "SELECT hex(token) FROM recovery_tokens ORDER BY created_at DESC LIMIT 1"
   ```

4. **Complete Recovery:**
   ```bash
   curl -X POST http://localhost:59001/api/v1/accounts/recovery/complete \
     -H "Content-Type: application/json" \
     -d '{
       "token": "<token_from_step_3>",
       "public_key": "<32_byte_hex_public_key>"
     }'
   ```

**Verify:**
- [ ] Recovery email is queued with `email_type='recovery'`
- [ ] Recovery email is sent (check logs or email inbox)
- [ ] Token expires after 24 hours
- [ ] Token can only be used once
- [ ] New public key is added to account

## Test 5: Email Retry Logic

Test that failed emails are retried with exponential backoff:

1. **Simulate Failure:** Use an invalid API key temporarily
   ```bash
   # In .env, set a bad API key
   MAILCHANNELS_API_KEY=invalid_key_for_testing
   ```

2. **Queue an Email:**
   ```bash
   # Request recovery for testing
   curl -X POST http://localhost:59001/api/v1/accounts/recovery/request \
     -H "Content-Type: application/json" \
     -d '{"email": "test@example.com"}'
   ```

3. **Watch the Logs:**
   Server logs should show retry attempts with backoff:
   ```
   "Email <id> failed (attempt 1/24, type: recovery): ..."
   "Skipping email <id> (backoff: 60s remaining)"
   "Email <id> failed (attempt 2/24, type: recovery): ..."
   ```

4. **Restore Valid API Key** and watch email eventually send

**Verify:**
- [ ] Failed emails are retried
- [ ] Backoff increases exponentially (2^n minutes)
- [ ] After max attempts, email is marked as 'failed'
- [ ] Email type determines retry count (Recovery=24, Welcome=12, General=6)

## Test 6: Production Smoke Test

Before going to production:

```bash
# Set production env vars
export ENVIRONMENT=prod
export FRONTEND_URL=https://decent-cloud.org
export MAILCHANNELS_API_KEY=<prod_key>
export DKIM_DOMAIN=decentcloud.org
export DKIM_SELECTOR=mailchannels
export DKIM_PRIVATE_KEY=<prod_key>

# Test email send
cargo run --bin api-server -- test-email --to admin@decentcloud.org --with-dkim
```

**Verify:**
- [ ] Email arrives from correct domain
- [ ] DKIM signature validates
- [ ] Email doesn't go to spam
- [ ] Recovery links point to production URL

## Troubleshooting

### Email Not Sending

1. Check API key is valid:
   ```bash
   echo $MAILCHANNELS_API_KEY
   ```

2. Check email processor is running:
   ```bash
   # Look for this in server logs:
   "Starting email processor (interval: 30s, batch: 10)"
   ```

3. Check email queue for errors:
   ```bash
   sqlite3 api/test.db "SELECT subject, status, attempts, last_error FROM email_queue WHERE status='pending' OR status='failed'"
   ```

### DKIM Not Validating

1. Verify DNS records are published:
   ```bash
   dig TXT mailchannels._domainkey.decentcloud.org
   ```

2. Check private key format (must be base64 encoded)

3. Use online DKIM validator: https://dkimcore.org/tools/

### Recovery Emails Not Arriving

1. Check `FRONTEND_URL` is set correctly
2. Verify recovery token was created in database
3. Check email processor logs for send errors
4. Verify email isn't in spam folder

## Production Readiness Checklist

Before deploying to production:

- [ ] MailChannels API key tested and working
- [ ] DKIM keys configured and DNS records published
- [ ] DKIM signatures validate correctly
- [ ] Welcome emails send successfully for new accounts
- [ ] Recovery emails send and links work correctly
- [ ] Email retry logic tested
- [ ] `FRONTEND_URL` set to production URL
- [ ] Emails arrive in inbox (not spam)
- [ ] Test email sent to admin address successfully

## Monitoring in Production

After deployment, monitor:

1. **Email Queue Status:**
   ```sql
   SELECT status, COUNT(*) FROM email_queue GROUP BY status;
   ```

2. **Failed Emails:**
   ```sql
   SELECT subject, to_addr, last_error FROM email_queue WHERE status='failed';
   ```

3. **Email Send Rate:**
   ```sql
   SELECT COUNT(*) FROM email_queue WHERE sent_at > datetime('now', '-1 hour');
   ```

Set up alerts for:
- High number of failed emails
- Emails stuck in pending state for > 1 hour
- Recovery tokens not being cleaned up
