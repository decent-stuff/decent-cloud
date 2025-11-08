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
