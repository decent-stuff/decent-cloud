# OAuth2 Authentication Guide

**Last Updated:** 2025-11-26

## Overview

Decent Cloud supports Google OAuth2 authentication alongside traditional seed phrase authentication. This allows users to sign in with their Google account instead of managing seed phrases.

### Key Features

- **Automatic Account Linking**: If you already have an account with a specific email (created via seed phrase), signing in with Google using the same email will automatically link to your existing account
- **Server-Side Key Management**: OAuth users get Ed25519 keypairs generated and stored server-side in secure httpOnly cookies
- **Unified API**: Both authentication methods use the same signing-based API, keeping the architecture simple
- **7-Day Sessions**: OAuth sessions persist for 7 days without requiring re-authentication
- **No Seed Phrases**: OAuth users don't need to manage seed phrases - authentication is handled by Google

## How It Works

### User Perspective

**New Users:**
1. Click "Sign in with Google" on the login page
2. Authorize Decent Cloud on Google's consent screen
3. Choose a username for your Decent Cloud account
4. Start using Decent Cloud

**Returning Users:**
1. Click "Sign in with Google"
2. Automatically signed in (session persists for 7 days)

**Users with Existing Accounts:**
1. If you created an account with seed phrase using `alice@example.com`
2. Later sign in with Google using the same email
3. OAuth automatically links to your existing account
4. You can now use either authentication method

### Technical Flow

```
User clicks "Sign in with Google"
         ↓
Backend generates PKCE challenge & CSRF token
         ↓
Redirect to Google OAuth consent screen
         ↓
User authorizes application
         ↓
Google redirects back with authorization code
         ↓
Backend exchanges code for access token (PKCE verification)
         ↓
Backend fetches user info (email, Google ID)
         ↓
Check if OAuth account exists
   ├─ YES → Load account, generate keypair
   └─ NO → Check if email matches existing account
       ├─ YES → Link OAuth to account, generate keypair
       └─ NO → Redirect to username selection
                  ↓
            User creates account with username
                  ↓
       Generate Ed25519 keypair, store in cookie
                  ↓
              Redirect to dashboard
```

## Configuration

### Development Setup

#### 1. Get Google OAuth Credentials

1. Visit [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project or select an existing one
3. Enable "Google+ API" or "People API"
4. Navigate to **Credentials** → **Create Credentials** → **OAuth 2.0 Client ID**
5. Application type: **Web application**
6. Add authorized redirect URI: `http://localhost:59001/api/v1/oauth/google/callback`
7. Copy your **Client ID** and **Client Secret**

#### 2. Configure Environment Variables

Edit `cf/.env.dev`:

```bash
GOOGLE_OAUTH_CLIENT_ID=your_dev_google_client_id
GOOGLE_OAUTH_CLIENT_SECRET=your_dev_google_client_secret
GOOGLE_OAUTH_REDIRECT_URL=http://localhost:59001/api/v1/oauth/google/callback
FRONTEND_URL=http://localhost:59000
```

#### 3. Start the Application

```bash
# Terminal 1: Start API
cd api
cargo run

# Terminal 2: Start frontend
cd website
npm run dev
```

#### 4. Test OAuth Flow

1. Visit http://localhost:59000/login
2. Click "Sign in with Google"
3. Authorize with your Google account
4. For new users: Enter a username
5. Redirected to dashboard ✅

### Production Setup

#### 1. Create Production OAuth Credentials

**IMPORTANT**: Use separate OAuth applications for development and production.

1. In Google Cloud Console, create a **new** OAuth 2.0 Client ID for production
2. Add production redirect URI: `https://api.decent-cloud.org/api/v1/oauth/google/callback`
3. Copy the production Client ID and Client Secret

#### 2. Configure Production Environment

Edit `cf/.env.prod`:

```bash
GOOGLE_OAUTH_CLIENT_ID=your_prod_google_client_id
GOOGLE_OAUTH_CLIENT_SECRET=your_prod_google_client_secret
GOOGLE_OAUTH_REDIRECT_URL=https://api.decent-cloud.org/api/v1/oauth/google/callback
FRONTEND_URL=https://decent-cloud.org
TUNNEL_TOKEN=your_production_tunnel_token
```

**Security Notes:**
- Secure cookies are **automatically enabled** when `FRONTEND_URL` starts with `https://`
- Never commit `.env.prod` to version control
- Use separate credentials for dev and prod environments

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/oauth/google/authorize` | Initiates OAuth flow, redirects to Google |
| `GET` | `/api/v1/oauth/google/callback` | OAuth callback handler (automatic) |
| `GET` | `/api/v1/oauth/session/keypair` | Returns current session keypair for signing |
| `GET` | `/api/v1/oauth/info` | Returns OAuth info (email, provider) for username prefill |
| `POST` | `/api/v1/oauth/register` | Completes registration for new OAuth users |
| `POST` | `/api/v1/oauth/logout` | Clears OAuth session cookies |

## Database Schema

### Tables Modified/Created

**`accounts` table** (modified):
- `auth_provider` (TEXT): `'seed_phrase'` or `'google_oauth'`
- `email` (TEXT): User's email address (unique, used for account linking)

**`oauth_accounts` table** (new):
- `id` (BLOB): Primary key
- `account_id` (BLOB): References `accounts(id)`
- `provider` (TEXT): OAuth provider (`'google_oauth'`)
- `external_id` (TEXT): Google user ID
- `email` (TEXT): Email from OAuth provider
- `created_at` (INTEGER): Timestamp in nanoseconds

**Constraints:**
- `UNIQUE(provider, external_id)`: One OAuth account per Google user
- `CHECK (provider IN ('google_oauth'))`: Only Google OAuth supported currently

See: `api/migrations/005_oauth_support.sql`

## Security

### Current Implementation

✅ **PKCE Flow**: Prevents authorization code interception attacks
✅ **HttpOnly Cookies**: Prevents XSS attacks on keypairs
✅ **Secure Cookies**: Automatically enabled for HTTPS (production)
✅ **CSRF Protection**: In-memory state tokens with 10-minute TTL
✅ **Automatic Cleanup**: Expired OAuth states cleaned up every 10 minutes
✅ **7-Day Expiry**: Limits session exposure window

### Account Linking Security

Email-based account linking is secure because:
- Google verifies email ownership during OAuth
- Attackers cannot use a victim's email without access to their Google account
- Only verified Google accounts can link to existing Decent Cloud accounts

**Edge Case**: If a user's email changes on Google after linking, the OAuth connection remains active. The system does not currently sync email changes.

### Production Recommendations

For multi-server production deployments:

1. **Session Store**: Replace in-memory `OAUTH_STATES` with Redis for horizontal scaling
2. **Rate Limiting**: Add rate limits to OAuth endpoints to prevent abuse
3. **Email Sync**: Consider syncing email changes on each OAuth callback
4. **Monitoring**: Log OAuth events for security auditing

## Code References

### Backend

- **Main OAuth logic**: `api/src/oauth_simple.rs`
- **Database methods**: `api/src/database/accounts.rs`
  - `create_oauth_account()` - Link OAuth provider to account
  - `get_oauth_account_by_provider_and_external_id()` - Lookup OAuth account
  - `get_account_by_email()` - Find account by email (for auto-linking)
  - `create_oauth_linked_account()` - Create new account with OAuth
- **Migration**: `api/migrations/005_oauth_support.sql`
- **Router**: `api/src/main.rs`

### Frontend

- **Auth store**: `website/src/lib/stores/auth.ts`
  - `loadOAuthSession()` - Loads keypair from httpOnly cookie
  - `logout()` - Clears OAuth session
- **Google sign-in button**: `website/src/lib/components/GoogleSignInButton.svelte`
- **Auth flow**: `website/src/lib/components/AuthFlow.svelte`
  - OAuth callback detection
  - Username selection for new users
  - OAuth registration
- **Identity utils**: `website/src/lib/utils/identity.ts`

## Troubleshooting

### "GOOGLE_OAUTH_CLIENT_ID environment variable not set"

**Solution:**
- Ensure all 4 environment variables are set in `cf/.env.dev` (or `cf/.env.prod`)
- Restart the API server after adding variables

### OAuth callback redirects to error page

**Possible causes:**
- Redirect URI mismatch in Google Console
- Check that it exactly matches: `http://localhost:59001/api/v1/oauth/google/callback`
- Verify `FRONTEND_URL` is set to `http://localhost:59000` (no trailing slash)

### Cookie not being set / Session not persisting

**Check:**
- Browser allows cookies (check browser settings)
- `credentials: 'include'` is set in fetch requests (already done in code)
- For production: Ensure `FRONTEND_URL` starts with `https://` to enable secure cookies
- Check browser console for cookie errors

### "Invalid or expired OAuth state"

**Cause:** OAuth state tokens expire after 10 minutes

**Solution:**
- Complete the OAuth flow within 10 minutes
- Don't refresh the page during the OAuth flow
- For debugging: Check `OAUTH_STATES` cleanup logic in `api/src/oauth_simple.rs:154`

### Session expires after 7 days

**This is expected behavior.** OAuth sessions expire after 7 days for security.

**Solution:**
- Users need to sign in with Google again after 7 days
- To extend session duration, modify `max_age` in `api/src/oauth_simple.rs:307`

## FAQ

**Q: Can users have both seed phrase and OAuth on the same account?**
A: Yes! If emails match, OAuth automatically links to the existing account. Users can then sign in using either method.

**Q: What happens if a user's email changes on Google?**
A: The OAuth link remains active with the old email. Email changes are not currently synced.

**Q: Why generate Ed25519 keypairs instead of using JWT tokens?**
A: This keeps the API unified - all requests use Ed25519 signatures regardless of authentication method. Simpler architecture than dual auth paths.

**Q: Is this production-ready?**
A: Yes for single-server deployments! For multi-server production:
- Use Redis for OAuth state storage (instead of in-memory)
- Add rate limiting to OAuth endpoints
- Consider email sync on callback

**Q: What providers are supported?**
A: Currently only Google OAuth2. The architecture supports adding more providers (GitHub, Microsoft, etc.) by extending the `oauth_accounts.provider` enum.

**Q: Can I disable seed phrase authentication and use only OAuth?**
A: The system supports both methods simultaneously. To disable seed phrases, you would need to remove the seed phrase UI components from the frontend.

## Testing

### Manual Test Flows

**Flow 1: New User (First Time with Google)**
1. Visit login page
2. Click "Sign in with Google"
3. Authorize on Google consent screen
4. Enter desired username
5. ✅ Redirected to dashboard

**Flow 2: Existing Account (Email Match)**
1. Create account via seed phrase (e.g., alice@example.com)
2. Log out
3. Click "Sign in with Google" using same email
4. ✅ Automatically links and redirects to dashboard

**Flow 3: Returning OAuth User**
1. Sign in with Google (already has account)
2. ✅ Immediately redirected to dashboard
3. Session persists across page refreshes

### Unit Tests

The backend includes comprehensive unit tests:
- `test_generate_ed25519_keypair()` - Keypair generation
- `test_generate_ed25519_keypair_uniqueness()` - Keypair uniqueness
- `test_create_google_oauth_client_*()` - OAuth client setup
- `test_should_use_secure_cookies_*()` - Cookie security flags

Run tests: `cargo test`

---

**Questions or issues?** Check the troubleshooting section above or open an issue on GitHub.
