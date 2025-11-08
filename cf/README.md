# Cloudflare Deployment

This directory contains Docker and Python scripts for deploying the Decent Cloud website with Cloudflare Tunnel.

## Quick Start

```bash
# 1. Setup tunnel (interactive)
python3 setup_tunnel.py

# 2. Deploy
python3 deploy_dev.py    # Development
python3 deploy_prod.py   # Production
```

## Files

### Python Scripts

- **setup_tunnel.py** - Interactive setup wizard for Cloudflare Tunnel configuration
- **deploy_dev.py** - Deploy to development environment
- **deploy_prod.py** - Deploy to production environment
- **cf_common.py** - Shared utilities (validation, env loading, Docker operations)

### Docker Files

- **docker-compose.yml** - Base Docker Compose configuration
- **docker-compose.prod.yml** - Production overrides (adds Cloudflare Tunnel)
- **Dockerfile** - Multi-stage build for website

### Configuration

- **.env.example** - Example environment file
- **.env.tunnel.example** - Example tunnel configuration
- **.env** - Your actual configuration (gitignored, created by setup_tunnel.py)

## Security

- Tunnel token is stored in `.env` file (gitignored)
- Token is **never** passed on command line
- Scripts load token from environment file securely

## Documentation

See [docker-deployment.md](./docker-deployment.md) for detailed setup instructions and troubleshooting.
