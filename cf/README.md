# Cloudflare Deployment

This directory contains Docker and Python scripts for deploying the Decent Cloud website with Cloudflare Tunnel.

> Production deploys to k3s (ArgoCD, namespace `dc-prod`) — see [`DEPLOYMENT_CONFIG.md`](./DEPLOYMENT_CONFIG.md) and [`CONFIG.md`](./CONFIG.md). The prod tunnel is local-managed (its routing lives in a ConfigMap in the k8s repo, not the Cloudflare API).

## Quick Start

```bash
# 1. (one-off, from CI which holds CF_API_TOKEN/CF_ACCOUNT_ID) ensure the tunnel + DNS exist:
python3 cf/tunnel.py dev        # prints the dev tunnel token

# 2. Deploy the local dev stack
python3 cf/deploy.py deploy dev             # Development (local; served over the dev tunnel)
```

> Production deploys via k8s (ArgoCD, namespace `dc-prod`) — see
> [`DEPLOYMENT_CONFIG.md`](./DEPLOYMENT_CONFIG.md). `deploy.py deploy` is dev-only
> (aborts on `prod`); use `deploy.py config <env>` to inspect either env's config.

## Blockchain Validator

The optional blockchain validator (`api-validate`) was part of the now-retired
`docker-compose.prod.yml` stack. Production runs on k8s (see
[`DEPLOYMENT_CONFIG.md`](./DEPLOYMENT_CONFIG.md)), which currently has no
validator Deployment; re-introducing the validator means adding it to the k8s
manifests, not to a compose file. See
[docs/mining-and-validation.md](../docs/mining-and-validation.md) for the
validation concept and economics.

## Files

### Python Scripts

- **tunnel.py** - Idempotent Cloudflare tunnel create-or-get + DNS ingress config (CF API; run from CI). Replaces the old interactive `setup_tunnel.py` (deleted).
- **deploy.py** - Local docker-compose dev-stack deployment (prod is k8s-only)

### Docker Files

- **docker-compose.dev.yml** - Development configuration (the only compose file; prod runs on k8s)
- **Dockerfile** - Builds the docker image for website (assumes native build)

### Configuration

- **.env.example** - Documents what environment variables exist
- Secrets are managed via `scripts/dc-secrets` (SOPS + age encryption)

## Security

- Secrets are encrypted at rest via SOPS + age (managed by `scripts/dc-secrets`)
- `deploy.py` loads secrets from dc-secrets automatically
- Token is **never** passed on command line

## Documentation

See [docker-deployment.md](./docker-deployment.md) for detailed setup instructions and troubleshooting.
