# Deployment Configuration Guide

This guide explains how to configure environment-specific settings for dev and production deployments.

## Overview

The deployment script (`cf/deploy.py`) uses environment-specific configuration files:
- **Development:** `cf/.env.dev`
- **Production:** `cf/.env.prod`

This allows you to maintain separate credentials and URLs for dev and production environments in the same repository.

---

## Setup Instructions

### 1. Configure Development Environment

Edit `cf/.env.dev`:

```bash
# Development Environment Configuration

# Google OAuth Configuration (Development)
GOOGLE_OAUTH_CLIENT_ID=your_dev_google_client_id
GOOGLE_OAUTH_CLIENT_SECRET=your_dev_google_client_secret
GOOGLE_OAUTH_REDIRECT_URL=http://localhost:59001/api/v1/oauth/google/callback
FRONTEND_URL=http://localhost:59000

# Cloudflare Tunnel (optional for dev)
# TUNNEL_TOKEN=your_dev_tunnel_token
```

**Getting Dev OAuth Credentials:**
1. Go to [Google Cloud Console](https://console.cloud.google.com/apis/credentials)
2. Create a new OAuth 2.0 Client ID (or use existing dev credentials)
3. Add authorized redirect URI: `http://localhost:59001/api/v1/oauth/google/callback`
4. Copy Client ID and Secret to `.env.dev`

### 2. Configure Production Environment

Edit `cf/.env.prod`:

```bash
# Production Environment Configuration

# Google OAuth Configuration (Production)
GOOGLE_OAUTH_CLIENT_ID=your_prod_google_client_id
GOOGLE_OAUTH_CLIENT_SECRET=your_prod_google_client_secret
GOOGLE_OAUTH_REDIRECT_URL=https://api.decent-cloud.org/api/v1/oauth/google/callback
FRONTEND_URL=https://decent-cloud.org

# Cloudflare Tunnel (REQUIRED for production)
TUNNEL_TOKEN=your_production_tunnel_token
```

**Getting Prod OAuth Credentials:**
1. Create a **separate** OAuth app for production in Google Cloud Console
2. Add authorized redirect URI: `https://api.decent-cloud.org/api/v1/oauth/google/callback`
3. Copy Client ID and Secret to `.env.prod`

**Getting Tunnel Token:**
1. Go to [Cloudflare Dashboard](https://one.dash.cloudflare.com/)
2. Navigate to Networks > Connectors > Cloudflare Tunnels
3. Create a tunnel and copy the token from the Docker installation command

---

## Deployment

### Deploy to Development

```bash
python3 cf/deploy.py deploy dev
```

This command:
- Loads configuration from `cf/.env.dev`
- Builds website with dev API URL (`http://localhost:59001`)
- Starts services with development settings
- OAuth uses HTTP cookies (not secure)

### Deploy to Production

```bash
python3 cf/deploy.py deploy prod
```

This command:
- Loads configuration from `cf/.env.prod`
- Verifies TUNNEL_TOKEN is present
- Builds website with production API URL (`https://api.decent-cloud.org`)
- Starts services with production settings
- OAuth uses HTTPS cookies (secure flag enabled)

---

## Security Notes

### Separate OAuth Apps

**IMPORTANT:** Always use separate OAuth applications for development and production:

- **Development:** Redirect URI uses `http://localhost:59001`
- **Production:** Redirect URI uses `https://api.decent-cloud.org`

Google OAuth will reject tokens if the redirect URI doesn't match exactly.

### Cookie Security

The API automatically enables secure cookies based on `FRONTEND_URL`:
- `http://` → Secure cookies **disabled** (works over HTTP)
- `https://` → Secure cookies **enabled** (requires HTTPS)

### .gitignore Protection

Both `.env.dev` and `.env.prod` are automatically ignored by git (see `.gitignore`):

```gitignore
.env
.env.*
!.env.example
```

**Never commit these files to version control!**

---

## Environment Variables Reference

### Required for Development

| Variable | Example | Description |
|----------|---------|-------------|
| `GOOGLE_OAUTH_CLIENT_ID` | `123456.apps.googleusercontent.com` | Dev OAuth Client ID |
| `GOOGLE_OAUTH_CLIENT_SECRET` | `GOCSPX-abc123...` | Dev OAuth Secret |
| `GOOGLE_OAUTH_REDIRECT_URL` | `http://localhost:59001/api/v1/oauth/google/callback` | OAuth callback URL |
| `FRONTEND_URL` | `http://localhost:59000` | Frontend base URL |

### Required for Production

| Variable | Example | Description |
|----------|---------|-------------|
| `GOOGLE_OAUTH_CLIENT_ID` | `789012.apps.googleusercontent.com` | Prod OAuth Client ID |
| `GOOGLE_OAUTH_CLIENT_SECRET` | `GOCSPX-xyz789...` | Prod OAuth Secret |
| `GOOGLE_OAUTH_REDIRECT_URL` | `https://api.decent-cloud.org/api/v1/oauth/google/callback` | OAuth callback URL |
| `FRONTEND_URL` | `https://decent-cloud.org` | Frontend base URL |
| `TUNNEL_TOKEN` | `eyJhIjoiZmZi...` | Cloudflare Tunnel token |

---

## Verification

After deployment, the script shows which configuration was loaded:

```
✓ Found .env.dev
✓ Google OAuth credentials loaded
→   Redirect URL: http://localhost:59001/api/v1/oauth/google/callback
→   Frontend URL: http://localhost:59000
```

Or for production:

```
✓ Found .env.prod
✓ Tunnel token loaded
✓ Google OAuth credentials loaded
→   Redirect URL: https://api.decent-cloud.org/api/v1/oauth/google/callback
→   Frontend URL: https://decent-cloud.org
```

---

## Troubleshooting

### "Environment config not found"

**Error:** `Environment config not found: cf/.env.dev`

**Solution:** Create the file by copying from example:
```bash
cp cf/.env.dev.example cf/.env.dev
# Edit cf/.env.dev with your credentials
```

### "TUNNEL_TOKEN not found in production config"

**Error:** `TUNNEL_TOKEN not found in production config`

**Solution:** Add tunnel token to `cf/.env.prod`:
```bash
TUNNEL_TOKEN=your_actual_tunnel_token_here
```

### OAuth "redirect_uri_mismatch"

**Error:** Google OAuth shows redirect URI mismatch

**Solution:**
1. Check `GOOGLE_OAUTH_REDIRECT_URL` in your env file matches exactly
2. Verify Google Cloud Console has the same redirect URI configured
3. Ensure you're using the correct OAuth app (dev vs prod)

### Cookies not secure in production

**Error:** Browser shows cookies without Secure flag

**Solution:** Verify `FRONTEND_URL` starts with `https://` in `cf/.env.prod`
```bash
FRONTEND_URL=https://decent-cloud.org  # Correct
FRONTEND_URL=http://decent-cloud.org   # Wrong!
```

---

## Migration from Old .env

If you have an existing `cf/.env` file:

1. **Create dev config:**
   ```bash
   cp cf/.env cf/.env.dev
   ```

2. **Create prod config:**
   ```bash
   cp cf/.env cf/.env.prod
   ```

3. **Update credentials:**
   - Edit `.env.dev` with development OAuth credentials
   - Edit `.env.prod` with production OAuth credentials and tunnel token

4. **Test:**
   ```bash
   python3 cf/deploy.py deploy dev
   ```

---

## See Also

- [OAuth Authentication Guide](../docs/OAUTH_AUTHENTICATION.md) - OAuth setup and configuration
- [Development Guide](./development.md) - Local development setup
- Main `cf/deploy.py` script - Deployment automation
