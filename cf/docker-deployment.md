# Docker Deployment with Cloudflare Tunnel

This guide explains how to deploy the Decent Cloud website using Docker Compose with Cloudflare Tunnel for secure external access.

## Overview

The deployment consists of:
- **website**: Static Next.js website served via nginx (this deployment)
- **cloudflared**: Cloudflare Tunnel connector for secure external access
- **cloudflare-api**: Cloudflare Workers API (separate deployment, see `cloudflare-api/` directory)

## Architecture

### Static Website (This Deployment)

```
Internet
    ↓
Cloudflare Network
    ↓
Cloudflare Tunnel (cloudflared container)
    ↓
decent-cloud.example.com → website:8080 (Docker internal network)
                              ↓
                        nginx serving static files
```

### Complete System Architecture

```
Web Clients
    ↓
Cloudflare Network
    ├─→ Static Website (via Tunnel) → Docker container (this deployment)
    └─→ API Endpoints (via Workers) → Cloudflare Workers + D1 + ICP Canister
```

**Port Configuration:**
- **Container internal port**: 3000 (nginx)
- **Host port** (dev, local testing): 59000 → 3000
- **Host port** (prod, local testing): 59100 → 3000
- **Tunnel service URL**: `website:3000` (uses Docker network, NOT host port)

**Note:** Dev and prod use different host ports (59000 vs 59100) so both can run simultaneously on the same machine.

**Benefits:**
- No firewall ports need to be opened
- Built-in DDoS protection via Cloudflare
- Automatic TLS/SSL encryption
- No public IP exposure
- Everything runs in containers
- Static website and API served through same domain (different paths/Workers routing)

## Prerequisites

1. A Cloudflare account with access to your domain
2. Docker and Docker Compose installed

## Quick Setup with Python Scripts (Recommended)

Python scripts are provided for automated deployment. They handle validation, environment setup, and provide clear feedback.

### Prerequisites

- Python 3.10 or higher
- Docker and Docker Compose
- A Cloudflare account with access to your domain

### Quick Start

```bash
cd cf

# 1. Run interactive setup (guides you through tunnel creation)
python3 setup_tunnel.py

# 2. Deploy to development
python3 deploy_dev.py

# Or deploy to production
python3 deploy_prod.py
```

The scripts will:
- Validate Docker installation
- Check for tunnel token (stored in `.env`, not on command line)
- Build and start containers
- Verify tunnel connection
- Provide helpful feedback and troubleshooting

## Manual Setup (Alternative)

If you prefer manual setup, follow these steps:

### Step 1: Create a Remotely-Managed Tunnel in Cloudflare Dashboard

1. Go to [Cloudflare Zero Trust](https://one.dash.cloudflare.com/)
2. Navigate to **Networks** > **Connectors** > **Cloudflare Tunnels**
3. Click **Create a tunnel**
4. Choose **Cloudflared** and click **Next**
5. Enter tunnel name: `decent-cloud-website`
6. Click **Save tunnel**

### Step 2: Configure Public Hostname

1. In the tunnel configuration page, go to the **Public Hostname** tab
2. Click **Add a public hostname**
3. Configure:
   - **Subdomain**: your choice (e.g., `decent-cloud` or `app`)
   - **Domain**: your domain (e.g., `example.com`)
   - **Service Type**: `HTTP`
   - **URL**: `website:3000` ⚠️ **Use port 3000, NOT 59000 or 59100!**
4. Click **Save hostname**

**CRITICAL:** The service URL must be `website:3000` (internal container port). Do NOT use host ports (59000/59100) - those are only for local access.

### Step 3: Get the Tunnel Token

1. In the tunnel page, select **Configure** (or **Edit**)
2. Choose **Docker** as the environment
3. Copy the installation command shown in the dashboard
4. Extract just the token value (the long string starting with `eyJhIjoiNWFi...`)

The command looks like:
```bash
docker run cloudflare/cloudflared:latest tunnel --no-autoupdate run --token eyJhIjoiNWFi...
```

### Step 4: Save Token Locally

Create `.env` file in the `cf/` directory:

```bash
cd cf

# Copy the example file
cp .env.tunnel.example .env

# Edit and add your token
nano .env
```

Add your token:
```bash
export TUNNEL_TOKEN=eyJhIjoiNWFi...your-actual-token-here
```

**Important:** This file is gitignored and contains secrets - never commit it!

### Step 5: Start Services

Using Python scripts (recommended):
```bash
cd cf

# Development
python3 deploy_dev.py

# Production
python3 deploy_prod.py
```

Or manually with docker compose:
```bash
cd cf

# Load the tunnel token and start services
source .env && docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d
```

### Step 6: Verify Deployment

```bash
# Check service health
docker compose -f docker-compose.yml -f docker-compose.prod.yml ps

# View logs
docker compose -f docker-compose.yml -f docker-compose.prod.yml logs -f

# Test the website endpoint
curl https://your-subdomain.your-domain.com/health

# Expected response: healthy
```

## Management

### View Logs

```bash
# All services
docker compose -f docker-compose.yml -f docker-compose.prod.yml logs -f

# Website only
docker compose -f docker-compose.yml -f docker-compose.prod.yml logs -f website

# Cloudflared only
docker compose -f docker-compose.yml -f docker-compose.prod.yml logs -f cloudflared
```

### Restart Services

```bash
# Restart all
export $(cat .env.tunnel | xargs) && docker compose -f docker-compose.yml -f docker-compose.prod.yml restart

# Restart specific service
export $(cat .env.tunnel | xargs) && docker compose -f docker-compose.yml -f docker-compose.prod.yml restart website
```

### Stop Services

```bash
docker compose -f docker-compose.yml -f docker-compose.prod.yml down
```

### Update and Rebuild

```bash
# Pull latest code changes
git pull

# Rebuild and restart
export $(cat .env.tunnel | xargs) && docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d --build
```

## Monitoring

### Health Checks

The website service includes a built-in health check that runs every 30 seconds:

```bash
docker compose -f docker-compose.yml -f docker-compose.prod.yml ps  # Shows health status
```

### Tunnel Status

Check tunnel status in the Cloudflare dashboard:
1. Go to [Cloudflare Zero Trust](https://one.dash.cloudflare.com/)
2. Navigate to **Networks** > **Connectors** > **Cloudflare Tunnels**
3. Your tunnel should show as "Healthy" with active connections

## Troubleshooting

### Website Service Won't Start

```bash
# Check logs for errors
docker compose -f docker-compose.yml -f docker-compose.prod.yml logs website

# Check environment variables
docker compose -f docker-compose.yml -f docker-compose.prod.yml config
```

### Tunnel Connection Issues

```bash
# View cloudflared logs
docker compose -f docker-compose.yml -f docker-compose.prod.yml logs cloudflared

# Common issues:
# 1. Invalid token - verify TUNNEL_TOKEN in .env.tunnel
# 2. Token not loaded - ensure you run: export $(cat .env.tunnel | xargs)
# 3. Network issues - check your firewall allows outbound connections to Cloudflare
```

### "Unable to reach the origin service" Error

```
ERR error="Unable to reach the origin service... dial tcp 172.24.0.2:59XXX: connect: connection refused"
```

This means the tunnel configuration in Cloudflare dashboard is pointing to the wrong port.

**Fix:**
1. Go to Cloudflare dashboard: Networks > Tunnels > your tunnel
2. Edit the Public Hostname
3. Change Service URL to `website:3000` (NOT 59000 or 59100)
4. Save and wait a few seconds for the tunnel to reconnect

**Why:** Ports 59000/59100 are host ports for local testing. The tunnel connects via Docker's internal network and must use the container's internal port 3000.

### "TUNNEL_TOKEN not found" Error

```bash
# Make sure to export the environment variable before running docker compose
source .env
docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d
```

### DNS Not Resolving

```bash
# Check DNS propagation (may take a few minutes)
nslookup your-subdomain.your-domain.com

# Verify public hostname configuration in Cloudflare dashboard
# Networks > Tunnels > decent-cloud-website > Public Hostname tab
```

### Port Already in Use

If dev/prod ports conflict with other services, edit the respective compose file:

**Development** (`docker-compose.dev.yml`):
```yaml
services:
  website:
    ports:
      - "59001:3000"  # Changed dev port from 59000 to 59001
```

**Production** (`docker-compose.prod.yml`):
```yaml
services:
  website:
    ports:
      - "59101:3000"  # Changed prod port from 59100 to 59101
```

## Security Considerations

1. **Token Protection**: The `.env.tunnel` file contains a secret token and should be excluded from git via `.gitignore`

2. **Cloudflare Protection**: All traffic goes through Cloudflare's network, providing:
   - DDoS protection
   - Web Application Firewall (WAF)
   - Rate limiting
   - TLS encryption

3. **Container Security**: The nginx service runs with minimal permissions

4. **Token Rotation**: You can regenerate the tunnel token at any time from the Cloudflare dashboard

## Production Recommendations

1. **Monitoring**: Set up Cloudflare Access logs and website monitoring
2. **Updates**: Keep Docker images updated with `docker compose pull`
3. **Secrets Management**: Consider using Docker secrets for production
4. **Rate Limiting**: Configure Cloudflare rate limiting rules for protection
5. **Health Alerts**: Enable Cloudflare tunnel health notifications

## Local Testing Without Tunnel

To test the website locally without the tunnel:

**Development:**
```bash
# Start just the website service
docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d website

# Access at http://localhost:59000
curl http://localhost:59000/health
```

**Production:**
```bash
# Start just the website service
docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d website

# Access at http://localhost:59100
curl http://localhost:59100/health
```

## Cleanup

To completely remove the deployment:

```bash
# Stop and remove containers
docker compose -f docker-compose.yml -f docker-compose.prod.yml down

# Remove images
docker rmi decent-cloud-website:latest

# Delete tunnel from Cloudflare dashboard
# Networks > Tunnels > decent-cloud-website > Delete
```

## Python Scripts Reference

### setup_tunnel.py

Interactive setup wizard that guides you through:
- Creating a Cloudflare Tunnel
- Configuring public hostname
- Saving the tunnel token to `.env`

**Usage:**
```bash
cd cf
python3 setup_tunnel.py
```

**Security:** Token is saved to `.env` file (gitignored), never passed on command line.

### deploy_dev.py

Deploys to development environment:
- Loads token from `.env` file
- Validates Docker installation
- Builds and starts containers
- Verifies tunnel connection
- Provides troubleshooting feedback

**Usage:**
```bash
cd cf
python3 deploy_dev.py
```

### deploy_prod.py

Deploys to production environment with additional checks:
- Same features as dev deployment
- Production-specific configuration
- Clearer verification steps

**Usage:**
```bash
cd cf
python3 deploy_prod.py
```

### cf_common.py

Shared utility module used by all scripts:
- Color output helpers
- Docker validation
- Environment file parsing
- Token loading (never from command line)
- Docker Compose execution wrapper

## Quick Reference Commands

Using Python scripts (recommended):
```bash
cd cf

# Setup
python3 setup_tunnel.py

# Deploy dev
python3 deploy_dev.py

# Deploy prod
python3 deploy_prod.py
```

Manual docker compose commands:
```bash
cd cf

# Start with token from file
source .env && docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d

# View all logs
docker compose -f docker-compose.yml -f docker-compose.prod.yml logs -f

# Test endpoint
curl https://your-subdomain.your-domain.com/health

# Stop everything
docker compose -f docker-compose.yml -f docker-compose.prod.yml down

# Rebuild after code changes
source .env && docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d --build
```

## Alternative: Using Docker Secrets (Production)

For production environments, use Docker secrets instead of environment variables:

```yaml
# docker-compose.prod.yml
services:
  cloudflared:
    secrets:
      - tunnel_token
    command: tunnel --no-autoupdate run --token $(cat /run/secrets/tunnel_token)

secrets:
  tunnel_token:
    file: ./secrets/tunnel_token.txt
```
