# Decent Cloud Deployment Architecture

## Overview

Decent Cloud uses a tunnel-based deployment for both the website and API, avoiding Cloudflare Workers entirely.

## Architecture

```
decent-cloud.org (website)
    ↓ Cloudflare Tunnel
    ↓ nginx container (port 3000)
    ↓ SvelteKit static build

api.decent-cloud.org (API)
    ↓ Cloudflare Tunnel
    ↓ poem API server (port 8080)
    ↓ Rust binary serving /api/v1/* endpoints

Validator (optional)
    ↓ Runs dc CLI binary
    ↓ Periodic blockchain validation (every 10 min)
    ↓ Signs latest block hash with provider identity
```

## Components

### 1. Website (decent-cloud.org)
- **Technology**: SvelteKit static export
- **Container**: nginx serving static files
- **Port**: 3000 (internal), 59100 (host)
- **Health**: `/health` endpoint
- **Deployment**: Docker container + Cloudflare Tunnel

### 2. API (api.decent-cloud.org)
- **Technology**: Rust poem web framework
- **Container**: Debian slim with Rust binaries (api-server, dc)
- **Port**: 8080 (internal), 59001 (host dev) 59101 (host prod)
- **Health**: `/api/v1/health` endpoint
- **Deployment**: Docker container + Cloudflare Tunnel
- **Services**:
  - `api-serve`: Serves HTTP API
  - `api-sync`: Syncs blockchain data periodically
  - `api-validate`: (Optional) Validates blockchain every 10 minutes

### 3. Validator (Optional)
- **Technology**: dc CLI binary
- **Container**: Same as API (reuses api image)
- **Purpose**: Blockchain validation for earning DCT rewards
- **Frequency**: Every 10 minutes (configurable)
- **Requirements**:
  - Validator identity (private key) mounted from host
  - Minimum 0.5 DCT balance for validation fees
  - Registered provider on the network

## Deployment Files

### Docker Compose
- `cf/docker-compose.dev.yml` - Development configuration
- `cf/docker-compose.prod.yml` - Production configuration

### Dockerfiles
- `cf/Dockerfile` - Build for website (assumes website built natively and then added to the image)
- `api/Dockerfile` - Build for API (assumes binary built natively and then added to the image)

### Python Scripts
- `cf/setup_tunnel.py` - Interactive tunnel configuration wizard
- `cf/deploy.py` - Deploy environment

## Cloudflare Tunnel Configuration

The tunnel must be configured in Cloudflare dashboard with two ingress rules:

1. **Website**: `decent-cloud.org` → `http://website:3000`
2. **API**: `api.decent-cloud.org` → `http://api:8080`

## Current Status

### ✅ Completed
- Website Docker container with nginx serving static SvelteKit build
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
```

## Notes

- **No Cloudflare Workers**: This project does NOT use Workers - everything runs in local containers accessed via tunnel
- **No wrangler**: All wrangler references have been removed
- **Clean separation**: Website and API are separate services with separate tunnels
- **Reference**: See `../icp-cc/poem-backend` for working example of tunnel-based API deployment
