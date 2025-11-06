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
- **Port**: 8080 (internal), 58100 (host)
- **Health**: `/api/v1/health` endpoint
- **Deployment**: Docker container + Cloudflare Tunnel
- **Endpoints**: `/api/v1/canister/*` - ICP canister proxy endpoints

## Deployment Files

### Docker Compose
- `cf/docker-compose.yml` - Base configuration for website
- `cf/docker-compose.prod.yml` - Production overrides (adds tunnel)
- `cf/docker-compose.dev.yml` - Development overrides

### Dockerfiles
- `cf/Dockerfile.prod` - Multi-stage build for website (Next.js + nginx)
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
- Website Docker container with nginx
- Tunnel deployment scripts
- Removed Cloudflare Worker dependencies
- Removed wrangler.toml configurations

### 🚧 In Progress
- Website tunnel routing (Worker still intercepting traffic)
- Poem API server creation

### 📋 TODO
1. **Undeploy Cloudflare Worker** that's intercepting decent-cloud.org
   - Go to Cloudflare Dashboard → Workers & Pages
   - Find and delete/disable the `decent-cloud-api` worker
   - Verify decent-cloud.org routes to tunnel

2. **Create Poem API Server** (in `api/` directory)
   - Copy structure from `icp-cc/poem-backend`
   - Implement `/api/v1/canister/*` endpoints
   - Add Docker deployment configuration
   - Add to docker-compose files

3. **Configure Tunnel for API**
   - Update tunnel configuration for api.decent-cloud.org
   - Test API endpoints

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
# Local container (once created)
curl http://localhost:58100/api/v1/health

# Production
curl https://api.decent-cloud.org/api/v1/health
```

## Notes

- **No Cloudflare Workers**: This project does NOT use Workers - everything runs in local containers accessed via tunnel
- **No wrangler**: All wrangler references have been removed
- **Clean separation**: Website and API are separate services with separate tunnels
- **Reference**: See `../icp-cc/poem-backend` for working example of tunnel-based API deployment
