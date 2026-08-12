# Prod Deployment Runbook — Going Live with Hetzner Resell

**Date:** 2026-08-12
**Status:** Operator approved. Awaiting prod code deployment.
**Priority:** #1 — prod buy flow is BROKEN until latest `main` is deployed.

## Context

The marketplace buy flow was verified end-to-end against a real Hetzner cx23 VM and merged
(PR #479, commit `05849932`). The operator approved going public on 2026-08-12. Prod already has
2 live Hetzner cloud-resell offerings (ids 11, 12). However, **prod is running code from before
PR #479** — the wallet-auto-accept bug means wallet-paid contracts get stuck at `requested`
forever. Deploying latest `main` is the critical prerequisite.

## Prod state (verified 2026-08-12)

- **API:** `https://api.decent-cloud.org` → health 200, environment `prod`
- **Offerings:** 2 public, in-stock, `provisioner_type=hetzner`:
  - **id 11** "Hetzner CX22 (resold)" — provider `1ed6136d…` (DC_PROD_RESELLER_PUBKEY),
    `server_type=cx23`, `location=fsn1`, `image=ubuntu-22.04`, $6.82/mo. Name says CX22 but
    config is CX23 (correct type, stale name). Missing hardware specs (cores/ram/disk = NULL).
  - **id 12** "Basic Linux VPS" — provider `1570f163…`, `server_type=cax11` (ARM),
    `location=fsn1`, `image=ubuntu-24.04`, $7.00/mo. Also missing hardware specs.
- **Stats:** `total_providers: 17` (inflated — pre-#482 honest-stats fix not deployed).
- **Code:** pre-#479 (confirmed by inflated stats + no version string in health).

## Steps

### Step 1 — Deploy latest `main` to prod (CRITICAL, operator action)

The agent container cannot reach the k8s repo. The operator must:

```bash
# In the k8s repo (third_party/k8s or wherever it's cloned):
# 1. Build + push the API image
docker build -t git.kalaj.org/decent-stuff/decent-cloud-api:$(git -C /project/decent-cloud/repo rev-parse --short HEAD) /project/decent-cloud/repo
docker push git.kalaj.org/decent-stuff/decent-cloud-api:$(git -C /project/decent-cloud/repo rev-parse --short HEAD)

# 2. Bump the image tag in the k8s base
# Edit cluster/apps/decent-cloud/base/dc-api.yaml → image: git.kalaj.org/decent-stuff/decent-cloud-api:<sha>

# 3. Commit + push → ArgoCD auto-syncs
git add -A && git commit -m "deploy: api $(git -C /project/decent-cloud/repo rev-parse --short HEAD)"
git push

# 4. Force ArgoCD refresh
kubectl -n argocd patch application decent-cloud --type=merge -p '{"metadata":{"annotations":{"argocd.argoproj.io/refresh":"normal"}}}'

# 5. Wait for rollout
kubectl -n dc-prod rollout status deployment/dc-api --timeout=300s

# 6. Verify
curl https://api.decent-cloud.org/api/v1/health   # → environment: prod
curl https://api.decent-cloud.org/api/v1/stats     # → total_providers should be ≤2 (honest stats)
```

**Alternative:** `python3 cf/deploy.py deploy stage` builds + pushes the stage image. For prod,
use the same flow against the prod overlay.

### Step 2 — Fix offering details (agent can do this via API)

Offering 11 has a stale name ("CX22" but server_type is cx23) and missing hardware specs.
After prod code is deployed, update via signed API request as the reseller identity:

```bash
# Load the reseller identity from the seed
SEED="$(scripts/dc-secrets get shared/env DC_PROD_RESELLER_SEED)"

# Derive keypair (reuse tools/e2e-real-deployments/src/crypto.js)
# Sign PUT /api/v1/providers/<pubkey>/offerings/<offering_id> with updated fields:
#   offer_name: "Hetzner CX23 (resold)"     # was "CX22"
#   provisioner_config.image: "ubuntu-24.04" # was 22.04
#   cores: 2, ram_mb: 4096, disk_gb: 40      # was NULL (cx23 = 2 vCPU / 4GB / 40GB)
```

The exact CX23 specs (from Hetzner API): 2 vCPU, 4.0 GB RAM, 40.0 GB disk, 20 TB traffic.

### Step 3 — Verify the buy flow against prod (after deploy)

```bash
# 1. Create a test buyer identity
api-cli identity generate --name prod-test-buyer

# 2. Register the buyer account
api-cli --api-url https://api.decent-cloud.org --env prod account register --identity prod-test-buyer

# 3. (In prod, email verification is required — the buyer must verify email before renting)

# 4. Rent offering 11
api-cli --api-url https://api.decent-cloud.org contract create \
  --identity prod-test-buyer --offering-id 11 \
  --ssh-pubkey "$(cat ~/.ssh/id_ed25519.pub)"

# 5. Wait for provisioning (~60s for cx23)
api-cli --api-url https://api.decent-cloud.org contract wait <ID> \
  --state active --timeout 180 --identity prod-test-buyer

# 6. Verify SSH access
api-cli --api-url https://api.decent-cloud.org contract get <ID> --identity prod-test-buyer
# → provisioning_instance_details should have {connection_type: "direct_ssh", public_ip, ssh_port: 22}
ssh -o StrictHostKeyChecking=no root@<public_ip>

# 7. Cancel (cleanup — MINIMIZE CLOUD SPENDING)
api-cli --api-url https://api.decent-cloud.org contract cancel <ID> --identity prod-test-buyer
```

## Known issues in prod (fix after deploy)

1. **Offering 11 name mismatch**: "CX22" in name, cx23 in config. Cosmetic but misleading.
2. **Missing hardware specs**: Both offerings have `cores/ram_mb/disk_gb = NULL`. Buyers can't
   see what they're getting. Fix via signed API update.
3. **Offering 11 image**: `ubuntu-22.04` (not latest). Consider updating to `ubuntu-24.04`.
4. **Inflated stats**: `total_providers: 17` includes seed/test accounts. The #482 honest-stats
   fix resolves this once deployed.
5. **EMAIL-GATE**: Buyers must verify email before renting in prod (operator decided to keep
   this gate — anti-Sybil). Ensure the email verification flow works end-to-end in prod.

## Blockers

- **k8s repo access**: The agent container cannot reach the k8s repo or deploy via ArgoCD.
  The operator must perform Step 1 (deploy latest main to prod).
- **Email verification in prod**: The dev bypass (`is_production_env()`) does NOT apply in prod.
  Buyers must complete email verification before they can rent. Ensure the email flow (SMTP,
  MailChannels) is configured in prod.
