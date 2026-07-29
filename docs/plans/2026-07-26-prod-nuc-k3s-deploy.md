# Plan: Prod → NUC k3s via CI on release tags; Dev → local + CF tunnel

## Goal

1. **Prod** is deployed to the operator's NUC (`192.168.0.2`, `nuc`) on its existing
   **k3s** cluster, from **CI on a release tag** (`v*.*.*`). Images are pushed to the
   NUC's **Forgejo** registry and reconciled by **ArgoCD** (GitOps).
2. **Dev** stays a local `docker compose` stack served over a dedicated Cloudflare
   tunnel (`TUNNEL_TOKEN_DEV`).

This **supersedes** the earlier Hetzner mail-VM docker-compose plan (abandoned: the 4 GB
mail VM could not safely co-host the prod stack). The mail VM (`204.168.149.118`) is
unaffected and continues to run only mail + dc-postgres.

## Architecture decision (confirmed with user 2026-07-26)

- Prod target: NUC, 32 GB RAM, k3s already running (Traefik ingress, ExternalDNS,
  cert-manager, ArgoCD, SOPS secrets). Decent-cloud joins the cluster in namespace
  `apps`, matching the conventions of co-located apps (twenty, forgejo, vikunja, …).
- CI runs on the existing **self-hosted** GitHub runner at `192.168.0.13` (same LAN →
  can reach the NUC and Forgejo; no public exposure of the k3s API).
- **Postgres** is NOT containerized: prod reuses the host **pg14** on `192.168.0.2:5432`
  where `decent_cloud_prod` + `chatwoot_prod` databases already exist and are migrated.
  Pods reach the host LAN IP directly (as other cluster apps do).
- **Public exposure** for `decent-cloud.org` uses an **in-cluster `cloudflared`
  Deployment** (reusing the existing live prod tunnel token), routing the public
  hostnames to in-cluster Services. This keeps the `decent-cloud.org` zone decoupled
  from the cluster's `*.kalaj.org` setup.

## Components (implemented)

| Area | File | Purpose |
|------|------|---------|
| Manifests | `deploy/k8s/decent-cloud.yaml` | 7 Deployments (api, api-sync, website, chatwoot-web, chatwoot-worker, redis, cloudflared) + chatwoot-migrate Job (ArgoCD sync-hook) + 4 Services, ns `apps` |
| Secret template | `deploy/k8s/decent-cloud-secret.yaml.template` | Documents the 37 prod secret keys |
| Tunnel mgmt | `cf/tunnel.py` | Idempotent CF tunnel create-or-get + DNS ingress (stdlib `urllib`, zero deps). Prod reuses the live `decent-cloud` tunnel; ingress targets in-cluster FQDNs |
| Secret gen | `scripts/gen-prod-secret.py` | Emits the 37-key prod k8s Secret from the AGE `dc-secrets` store; builds `API_DATABASE_URL` as `decent_cloud_prod`@`192.168.0.2`; reuses `TUNNEL_TOKEN` as `TUNNEL_TOKEN_PROD`; dies loud on any missing REQUIRED key |
| CI | `.github/workflows/release.yml` (`deploy-prod` job) | On `v*` tag, self-hosted runner: build api+website images → push `git.kalaj.org/decent-stuff/decent-cloud-{api,website}:<tag>` → bump image tags in the manifest → commit → ArgoCD auto-syncs |
| Operator docs | `deploy/k8s/SETUP.md`, `deploy/k8s/TUNNEL.md`, `deploy/k8s/README.md` | Cluster one-time setup + tunnel generate/rotate |
| Operator artifacts | `third_party/nuc-k3s/cluster/argocd/application-decent-cloud.yaml`, `cluster/secrets/*.template` | ArgoCD Application CR + SOPS-PGP secret templates (committed in the nuc-k3s clone) |
| Dev | `cf/docker-compose.dev.yml` | Cloudflared reads `${TUNNEL_TOKEN_DEV}` (split from prod) |

## CI deploy flow

```
v*.*.* tag
  → deploy-prod (runs-on: self-hosted @ 192.168.0.13)
      cargo build --release --bin api-server --bin dc   (SQLX_OFFLINE)
      (cd website && npm ci && npm run build)
      docker build api+website → push git.kalaj.org/decent-stuff/decent-cloud-{api,website}:<tag>
      bump image tags in deploy/k8s/decent-cloud.yaml → git commit+push
  → ArgoCD auto-sync (selfHeal, prune) reconciles the cluster
```

## Dev

Local `docker compose` (unchanged native build path) + a dedicated `TUNNEL_TOKEN_DEV`.
The shared `TUNNEL_TOKEN` (now prod-only) is reused as `TUNNEL_TOKEN_PROD` so the live
prod tunnel keeps its connector token.

## Status

- **Implemented + verified locally**: both Docker images proven healthy end-to-end
  (website `/health` 200; api `/api/v1/health` 200 against real postgres); manifests
  `kubeconform`-valid; `cf/tunnel.py` + `gen-prod-secret.py` unit-tested (pytest green);
  images built + pushed + pullable from Forgejo.
- **Live cutover pending**: in-cluster pods crash-looped until the prod DB role passwords
  (`decent_cloud_prod`, `chatwoot_prod`) were synced between the host DB and `dc-secrets`;
  then patch the in-cluster secret, restart api/api-sync, run chatwoot-migrate, flip the
  tunnel ingress from the old compose targets to the k8s FQDNs, and retire the old compose
  cloudflared. See `deploy/k8s/SETUP.md`.

## Secrets

- GitHub repo secrets: `CF_ACCOUNT_ID`, `CF_API_TOKEN`, `DC_REPO_WRITE`, `SOPS_AGE_KEY`,
  `FORGEJO_OWNER`, `FORGEJO_USER`, `FORGEJO_TOKEN`.
- `dc-secrets` (AGE, committed encrypted): `PROD_POSTGRES_PASSWORD`,
  `CHATWOOT_POSTGRES_PASSWORD`, `TUNNEL_TOKEN` (= prod connector token).
- Cluster secrets (PGP-SOPS, committed encrypted in the nuc-k3s clone): `decent-cloud-secret`
  (37 keys), `forgejo-registry-secret` (imagePullSecret).
