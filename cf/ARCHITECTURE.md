# Decent Cloud Deployment Architecture

## Overview

Decent Cloud uses a tunnel-based deployment for both the website and API, avoiding Cloudflare Workers entirely.

## Architecture

```
decent-cloud.org (website)
    ↓ Cloudflare Tunnel
    ↓ nginx container (port 3000)
    ↓ Next.js static build

api.decent-cloud.org (API)
    ↓ Cloudflare Tunnel
    ↓ poem API server (port 8080)
    ↓ Rust binary serving /api/v1/* endpoints
```

## Components

### 1. Website (decent-cloud.org)
- **Technology**: Next.js static export
- **Container**: nginx serving static files
- **Port**: 3000 (internal), 59100 (host)
- **Health**: `/health` endpoint
- **Deployment**: Docker container + Cloudflare Tunnel

### 2. API (api.decent-cloud.org)
- **Technology**: Rust poem web framework
- **Container**: Debian slim with Rust binary
- **Port**: 8080 (internal), 59001 (host dev) 59101 (host prod)
- **Health**: `/api/v1/health` endpoint
- **Deployment**: Docker container + Cloudflare Tunnel
- **Endpoints**: `/api/v1/canister/*` - ICP canister proxy endpoints

## Deployment Files

### Docker Compose
- `cf/docker-compose.yml` - Base configuration for website
- `cf/docker-compose.prod.yml` - Production overrides (adds tunnel)
- `cf/docker-compose.dev.yml` - Development overrides

### Dockerfiles
- `cf/Dockerfile` - Multi-stage build for website (Next.js + nginx)
- `api/Dockerfile` - Multi-stage build for API (Rust + poem)

### Python Scripts
- `cf/setup_tunnel.py` - Interactive tunnel configuration wizard
- `cf/deploy_prod.py` - Deploy production environment
- `cf/deploy_dev.py` - Deploy development environment
- `cf/cf_common.py` - Shared utilities

## Cloudflare Tunnel Configuration

The tunnel must be configured in Cloudflare dashboard with two ingress rules:

1. **Website**: `decent-cloud.org` → `http://website:3000`
2. **API**: `api.decent-cloud.org` → `http://api:8080`

## Current Status

### ✅ Completed
- Website Docker container with nginx serving static Next.js build
- API Docker container with poem serving REST endpoints
- Tunnel deployment scripts for production
- Removed all Cloudflare Worker dependencies
- Removed all wrangler.toml configurations
- Website accessible at https://decent-cloud.org
- API running locally at http://localhost:59101

### 📋 TODO
1. **Configure Tunnel for API** in Cloudflare Dashboard
   - Go to Zero Trust → Access → Tunnels
   - Edit your tunnel
   - Add new ingress rule:
     - Hostname: `api.decent-cloud.org`
     - Service: `http://api:8080`
   - This will make API accessible at https://api.decent-cloud.org

2. **Implement Full Canister Proxy Logic** (in `api/src/main.rs`)
   - Add ICP agent integration
   - Implement actual canister method calls
   - Add proper error handling and retries
   - Match all endpoints from `website/lib/cf-service.ts`

## Testing

### Website
```bash
# Local container
curl http://localhost:59100/
curl http://localhost:59100/health

# Production (after Worker removed)
curl https://decent-cloud.org/
curl https://decent-cloud.org/health
```

### API
```bash
# Local container
curl http://localhost:59101/api/v1/health

# Production (after tunnel configured)
curl https://api.decent-cloud.org/api/v1/health

# Test canister proxy endpoint
curl -X POST http://localhost:59101/api/v1/canister/test_method \
  -H "Content-Type: application/json" \
  -d '{"args": []}'
```

## Notes

- **No Cloudflare Workers**: This project does NOT use Workers - everything runs in local containers accessed via tunnel
- **No wrangler**: All wrangler references have been removed
- **Clean separation**: Website and API are separate services with separate tunnels
- **Reference**: See `../icp-cc/poem-backend` for working example of tunnel-based API deployment
