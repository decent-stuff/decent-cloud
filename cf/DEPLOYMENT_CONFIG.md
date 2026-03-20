# Deployment Configuration Guide

This guide explains how to manage secrets and environment-specific settings for dev and production deployments.

## Overview

Secrets are managed via `scripts/dc-secrets` (SOPS + age encryption). The deployment script (`cf/deploy.py`) loads secrets from dc-secrets automatically -- no manual `.env` file management needed.

The `.env.example` files remain as documentation of what variables exist.

---

## Setup Instructions

### 1. Initialize dc-secrets

```bash
scripts/dc-secrets init
```

This sets up SOPS with age encryption for the repository.

### 2. Set Credentials

```bash
# Set shared environment variables
scripts/dc-secrets set shared/env GOOGLE_OAUTH_CLIENT_ID=your_client_id
scripts/dc-secrets set shared/env GOOGLE_OAUTH_CLIENT_SECRET=your_client_secret
scripts/dc-secrets set shared/env GOOGLE_OAUTH_REDIRECT_URL=https://api.decent-cloud.org/api/v1/oauth/google/callback
scripts/dc-secrets set shared/env FRONTEND_URL=https://decent-cloud.org
scripts/dc-secrets set shared/env TUNNEL_TOKEN=your_tunnel_token
```

### 3. View and Edit Credentials

```bash
# List all secrets in a group
scripts/dc-secrets list shared/env

# Edit secrets interactively
scripts/dc-secrets edit shared/env
```

---

## Deployment

### Deploy to Development

```bash
python3 cf/deploy.py deploy dev
```

This command:
- Loads secrets from dc-secrets automatically
- Builds website with dev API URL (`http://localhost:59001`)
- Starts services with development settings
- OAuth uses HTTP cookies (not secure)

### Deploy to Production

```bash
python3 cf/deploy.py deploy prod
```

This command:
- Loads secrets from dc-secrets automatically
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
- `http://` -> Secure cookies **disabled** (works over HTTP)
- `https://` -> Secure cookies **enabled** (requires HTTPS)

### Encryption

All secrets are encrypted at rest using SOPS + age. Only users with the correct age key can decrypt them. Encrypted secret files can be safely committed to version control.

---

## Environment Variables Reference

| Variable | Example | Description |
|----------|---------|-------------|
| `GOOGLE_OAUTH_CLIENT_ID` | `123456.apps.googleusercontent.com` | OAuth Client ID |
| `GOOGLE_OAUTH_CLIENT_SECRET` | `GOCSPX-abc123...` | OAuth Secret |
| `GOOGLE_OAUTH_REDIRECT_URL` | `https://api.decent-cloud.org/api/v1/oauth/google/callback` | OAuth callback URL |
| `FRONTEND_URL` | `https://decent-cloud.org` | Frontend base URL |
| `TUNNEL_TOKEN` | `eyJhIjoiZmZi...` | Cloudflare Tunnel token |

---

## Troubleshooting

### OAuth "redirect_uri_mismatch"

**Error:** Google OAuth shows redirect URI mismatch

**Solution:**
1. Check `GOOGLE_OAUTH_REDIRECT_URL` via `scripts/dc-secrets list shared/env`
2. Verify Google Cloud Console has the same redirect URI configured
3. Ensure you're using the correct OAuth app (dev vs prod)

### Cookies not secure in production

**Error:** Browser shows cookies without Secure flag

**Solution:** Verify `FRONTEND_URL` starts with `https://`:
```bash
scripts/dc-secrets list shared/env  # check FRONTEND_URL value
scripts/dc-secrets set shared/env FRONTEND_URL=https://decent-cloud.org
```

---

## See Also

- [OAuth Authentication Guide](../docs/OAUTH_AUTHENTICATION.md) - OAuth setup and configuration
- [Development Guide](./development.md) - Local development setup
- Main `cf/deploy.py` script - Deployment automation
