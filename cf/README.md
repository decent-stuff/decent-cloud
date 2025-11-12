# Cloudflare Deployment

This directory contains Docker and Python scripts for deploying the Decent Cloud website with Cloudflare Tunnel.

## Quick Start

```bash
# 1. Setup tunnel (interactive)
python3 setup_tunnel.py

# 2. Deploy
python3 deploy.py deploy dev    # Development
python3 deploy.py deploy prod   # Production
```

## Blockchain Validator (Optional)

The deployment includes an optional blockchain validator service that earns DCT rewards by validating blocks every 10 minutes.

**By default, the validator is DISABLED** (using docker-compose profiles) to prevent accidental startup without a configured identity.

### Prerequisites

1. **Generate or have an existing validator identity**:
   ```bash
   dc keygen --generate --identity my-validator
   ```

2. **Register as a provider** (requires 0.5 DCT):
   ```bash
   dc provider register --identity my-validator
   ```

3. **Ensure sufficient balance** (0.5 DCT per validation):
   ```bash
   dc account --identity my-validator --balance
   ```

### Setup

1. **Locate your identity directory**

   Identity directories are typically at `~/.dcc/identity/<identity-name>` and contain:
   - `private.pem` (required for validation)
   - `public.pem`

   Example: `~/.dcc/identity/my-identity/`

2. **Enable the validator** by editing `docker-compose.yml`:

   a. Comment out the `profiles: ["validator"]` line in the `api-validate` service

   b. Add your identity mount in the volumes section:
   ```yaml
   api-validate:
     # profiles: ["validator"]  # <-- Comment this out to enable
     volumes:
       - ../data/api-data-prod:/data
       - ~/.dcc/identity/my-identity:/identity:ro  # <-- Add your identity mount
   ```

3. **Deploy with validator**:
   ```bash
   python3 deploy.py deploy prod
   ```

   **Alternative: Use profiles** (keeps base config unchanged):
   ```bash
   docker compose -f docker-compose.yml -f docker-compose.prod.yml --profile validator up -d
   ```

   > **What are profiles?** Docker Compose profiles allow services to be selectively enabled/disabled.
   > The `profiles: ["validator"]` line means the service only starts when explicitly requested
   > via `--profile validator` or when the profiles line is commented out.

### Configuration

Environment variables (set in `.env` or docker-compose override):

- `VALIDATION_INTERVAL_SECS`: Validation frequency in seconds (default: 600 = 10 minutes)
- `VALIDATION_MEMO`: Optional memo for validation transactions
- `NETWORK`: ICP network to use (default: `ic`)

### Monitoring

```bash
# Check validator logs
docker logs decent-cloud-api-validate-prod

# Check validator health
docker ps --filter name=validate
```

### Economics

- **Cost**: 0.5 DCT per validation
- **Reward**: Share of 50 DCT block reward (divided among all validators)
- **Frequency**: Every 10 minutes (one block time)
- **ROI**: Depends on number of active validators

See [docs/mining-and-validation.md](../docs/mining-and-validation.md) for more details.

## Files

### Python Scripts

- **setup_tunnel.py** - Interactive setup wizard for Cloudflare Tunnel configuration
- **deploy.py** - Unified deployment script with native build support (replaces deploy_dev.py and deploy_prod.py)
- **cf_common.py** - Shared utilities (validation, env loading, Docker operations)

### Docker Files

- **docker-compose.yml** - Base Docker Compose configuration
- **docker-compose.prod.yml** - Production overrides (adds Cloudflare Tunnel)
- **Dockerfile** - Builds the docker image for website (assumes native build)

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
