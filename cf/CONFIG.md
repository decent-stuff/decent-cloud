# Configuration reference — single source of truth for env vars

This is the **authoritative map** of every decent-cloud environment variable: where
it lives in DEV, where it lives in PROD, and the exact command to change + apply it.
If you "can't find where to set X", start here.

> **🚧 Staging migration (2026-08-03):** staging is moving off the local
> docker-compose "dev" stack + the `repo/secrets/shared/` age store onto k8s as
> namespace `dc-stage` (ArgoCD-synced from the k8s repo). A third env, **stage**, now
> exists: sources from `dc-stage-config` ConfigMap + `dc-stage-secret` Secret in
> the k8s repo (same model as prod). The DEV (docker-compose) + age-store content
> below stays accurate **until the operator cutover**; the legacy `scripts/dc-secrets`
> + `repo/secrets/shared/` are **retired pending post-cutover deletion**. Full
> runbook: `docs/MIGRATION-CUTOVER.md`. Audit live stage config with
> `python3 cf/deploy.py config stage`.

For live introspection (what is actually set right now) run:
```
python3 cf/deploy.py config dev     # or: prod
```

---

## TL;DR — where do I change OAuth?

The OAuth vars (`GOOGLE_OAUTH_CLIENT_ID`, `GOOGLE_OAUTH_CLIENT_SECRET`,
`GOOGLE_OAUTH_REDIRECT_URL`) are stored **differently per environment**:

| Var | DEV | PROD |
|-----|-----|------|
| `GOOGLE_OAUTH_CLIENT_ID` | `secrets/shared/dev.yaml` | `dc-config` ConfigMap |
| `GOOGLE_OAUTH_REDIRECT_URL` | `secrets/shared/dev.yaml` | `dc-config` ConfigMap |
| `GOOGLE_OAUTH_CLIENT_SECRET` | `secrets/shared/dev.yaml` | `dc-secret` Secret |

The split is deliberate (12-Factor: a client id + redirect URL are **public
identifiers**, only the client secret is secret) — but it is why "set OAuth" is hard
to find. Recipes:

**DEV** (all 3 in one file):
```bash
./scripts/dc-secrets set shared/dev GOOGLE_OAUTH_CLIENT_SECRET=<new_secret>
python3 cf/deploy.py deploy dev
```

**PROD** (id + redirect in `dc-config`, secret in `dc-secret`):
```bash
cd third_party/k8s
# 1. client id / redirect URL (public) → ConfigMap
$EDITOR cluster/apps/decent-cloud/dc-config.yaml
kubectl apply -f cluster/apps/decent-cloud/dc-config.yaml
# 2. client secret → SOPS Secret
sops cluster/secrets/dc-secret.yaml        # edit GOOGLE_OAUTH_CLIENT_SECRET, save
python3 scripts/manage-secrets.py
# 3. restart dc-api so it re-reads env
kubectl -n dc-prod rollout restart deploy/dc-api
```

> **Also required (separate from cluster config):** the redirect URI must be listed in
> the Google Cloud Console → APIs & Services → Credentials → OAuth client
> `101738308476-f4d276j2...` → **Authorized redirect URIs**. The prod URI is
> `https://api.decent-cloud.org/api/v1/oauth/google/callback`. Google Console changes
> need no restart.

---

## How config is split between dev and prod

Config is **inherently split** because the two envs deploy differently:

- **DEV** = docker-compose, driven by `cf/deploy.py`. Secrets are AGE-SOPS, layered,
  consumed as plain env vars by the compose services. One tool: `scripts/dc-secrets`.
- **PROD** = k3s + ArgoCD GitOps in namespace `dc-prod` (manifests in the
  **`third_party/k8s`** repo at `cluster/apps/decent-cloud/`). Config follows 12-Factor:
  non-secret values in a `dc-config` ConfigMap (plaintext, ArgoCD-synced), secret
  values in a `dc-secret` Secret (PGP-SOPS). Deploy = push to git / re-apply.

A literal single store for both envs will become possible once dev also moves to k8s
(a `dc-dev` namespace is planned). Until then, **this document is the unifying
reference**, and `deploy.py config <env>` reads the live store for whichever env.

### Variable sources at a glance

| Env | Secret store | Non-secret config | Deploy mechanism | Edit tool |
|-----|--------------|-------------------|------------------|-----------|
| dev | `repo/secrets/shared/{common,dev}.yaml` (AGE-SOPS) | same (env vars) | `python3 cf/deploy.py deploy dev` | `scripts/dc-secrets` |
| prod | `third_party/k8s/cluster/secrets/dc-secret.yaml` (PGP-SOPS) | `third_party/k8s/cluster/apps/decent-cloud/dc-config.yaml` (ConfigMap) | ArgoCD GitOps (push to the k8s repo) | `sops` + `kubectl` |

> **Two different SOPS key types.** Dev uses AGE; prod (the k8s repo) uses PGP key
> `FA5814CF1935EE80C454C9F1660DCCF069EC9176` (`encrypted_regex: ^(data|stringData)$`).
> A machine with only one key can only edit that env.

---

## Full variable reference

`cf/deploy.py config <env>` cross-checks the required vars below and **fails loudly** on
any that are missing/empty (per repo convention: never silently skip functionality).

Legend — **DEV source**: `C`=common.yaml, `D`=dev.yaml, `P`=play.yaml (local sidecar).
**PROD source**: `cfg`=dc-config ConfigMap, `sec`=dc-secret Secret, `lit`=inline in
manifest, `—`=not used in this env.

### API / database / encryption

| Var | DEV | PROD | Notes |
|-----|-----|------|-------|
| `DATABASE_URL` | `P` | — | local sidecar PG (`play` only) |
| `API_DATABASE_URL` | `D` | `sec` | prod: `postgres://decent_cloud_prod@192.168.0.2:5432/...` (host PG) |
| `CREDENTIAL_ENCRYPTION_KEY` | `D`,`P` | `sec` | symmetric key — rotating requires re-encrypting existing data |
| `API_PUBLIC_URL` | `D`,`P` | `lit` | prod inline `https://api.decent-cloud.org` |
| `API_SERVER_PORT` | `P` | `lit` | prod inline `59001` |

### Cloudflare (DNS + tunnel)

| Var | DEV | PROD | Notes |
|-----|-----|------|-------|
| `CF_ACCOUNT_ID` | `C` | `lit`?/— | account `ffbdd200090771d2174995002bf0aa7a` |
| `CF_API_TOKEN` | `C` | `sec` | scopes: Account:Read, Cloudflare Tunnel:Edit, Zone:Read, DNS:Edit |
| `CF_DOMAIN` | `C` | `lit` | `decent-cloud.org` (prod inline) |
| `CF_ZONE_ID` | `C` | `cfg` | zone `e5376a5252efc2c724063dec96bbcba3` |
| `CF_GW_PREFIX` | `D` | `lit` | dev=`dev-gw`, prod=`gw` (inline) |
| `TUNNEL_TOKEN` | `D` | — | dev connector token (remote-managed dev tunnel) |
| `CLOUDFLARED_CREDS_JSON` | — | `sec` | prod local-managed tunnel creds (decoded token JSON) |

### Stripe payments

| Var | DEV | PROD | Notes |
|-----|-----|------|-------|
| `STRIPE_SECRET_KEY` | `D` | `sec` | test vs live key per env |
| `STRIPE_PUBLISHABLE_KEY` | `D` | `cfg` | |
| `STRIPE_WEBHOOK_SECRET` | `D` | `sec` | per-webhook-endpoint signing secret |
| `VITE_STRIPE_PUBLISHABLE_KEY` | `D` | — | build-time (website bundle), dev only |
| `INVOICE_SELLER_IBAN` | — | `sec` | prod invoicing |

### Google OAuth

| Var | DEV | PROD | Notes |
|-----|-----|------|-------|
| `GOOGLE_OAUTH_CLIENT_ID` | `D` | `cfg` | public id |
| `GOOGLE_OAUTH_CLIENT_SECRET` | `D` | `sec` | secret |
| `GOOGLE_OAUTH_REDIRECT_URL` | `D` | `cfg` | must match a Google Console authorized URI |

### Frontend / public URLs

| Var | DEV | PROD | Notes |
|-----|-----|------|-------|
| `FRONTEND_URL` | `D`,`P` | `cfg` | prod `https://decent-cloud.org` |
| `CANISTER_ID` | `C` | `lit` | prod inline `ggi4a-wyaaa-aaaai-actqq-cai` |

### Email (MailChannels + DKIM + SMTP)

| Var | DEV | PROD | Notes |
|-----|-----|------|-------|
| `MAILCHANNELS_API_KEY` | `C` | `sec` | |
| `DKIM_DOMAIN` | `C` | `cfg` | prod `decent-cloud.org` |
| `DKIM_SELECTOR` | `C` | `cfg` | prod `mcdkim` |
| `DKIM_PRIVATE_KEY` | `C` | `sec` | |
| `SMTP_ADDRESS` | `D` | `cfg` | prod `smtp.mailchannels.net` |
| `SMTP_USERNAME` | `D` | `cfg` | prod `decentcloud` |
| `SMTP_PASSWORD` | `D` | `sec` | |
| `SMTP_PORT` | `D` | — | dev only (compose port mapping) |

### Chatwoot (support)

| Var | DEV | PROD | Notes |
|-----|-----|------|-------|
| `CHATWOOT_BASE_URL` | `D` | `lit` | prod in-cluster `http://dc-chatwoot-web.dc-prod.svc:80` |
| `CHATWOOT_FRONTEND_URL` | `D` | `lit` | prod `https://support.decent-cloud.org` |
| `CHATWOOT_API_TOKEN` | `D` | `sec` | |
| `CHATWOOT_PLATFORM_API_TOKEN` | `D` | `sec` | |
| `CHATWOOT_HMAC_SECRET` | `D` | `sec` | |
| `CHATWOOT_POSTGRES_PASSWORD` | `D` | `sec` | prod host PG db `chatwoot_prod` |
| `CHATWOOT_SECRET_KEY_BASE` | `D` | `sec` | Rails session key |
| `CHATWOOT_ACCOUNT_ID` | `D` | `lit` | prod inline `1` |
| `CHATWOOT_INBOX_ID` | — | `lit` | prod inline `1` |
| `CHATWOOT_WEBSITE_TOKEN` | `D` | — | dev widget embed |
| `OPENAI_API_KEY` | — | `sec` | chatwoot-worker (answer assist) |

> **Website build (support widget).** Two `VITE_*` vars are injected at website
> build time to load the in-page Chatwoot widget: `VITE_CHATWOOT_BASE_URL`
> (public URL of the Chatwoot instance serving `/packs/js/sdk.js`) and
> `VITE_CHATWOOT_WEBSITE_TOKEN` (the inbox website token). The widget renders
> **only when both are set** — an unset pair yields a console-clean bundle with
> the widget silently gated off (no dead-host fetch, no 404/X-Frame-Options
> error). Mapping:
> - `cf/deploy.py` (docker-compose deploy path): `VITE_CHATWOOT_BASE_URL` ←
>   `CHATWOOT_BASE_URL`, `VITE_CHATWOOT_WEBSITE_TOKEN` ← `CHATWOOT_WEBSITE_TOKEN`
>   (written to gitignored `website/.env.local`); requires **both** or it warns.
> - `.github/workflows/release.yml` (k8s image path): `VITE_CHATWOOT_BASE_URL` ←
>   repo **Variable** `CHATWOOT_BASE_URL`, `VITE_CHATWOOT_WEBSITE_TOKEN` ←
>   Actions **Secret** `CHATWOOT_WEBSITE_TOKEN`. Operator must set both in
>   `decent-stuff/decent-cloud` for a release.yml-built website image to show
>   support chat.
> The live Chatwoot instance is currently `https://dev-support.decent-cloud.org`
> (`CHATWOOT_BASE_URL`); `support.decent-cloud.org` is a dead tunnel and must
> not be used as a hardcoded default.

### Notifications

| Var | DEV | PROD | Notes |
|-----|-----|------|-------|
| `TELEGRAM_BOT_TOKEN` | `D` | `sec` | |
| `TELEGRAM_BOT_USERNAME` | `D` | `lit` | prod inline `DecentCloudBot` |
| `DEFAULT_ESCALATION_USER` | `C` | `cfg` | |
| `TEXTBEE_DEVICE_ID` | `C` | `cfg` | |
| `TEXTBEE_API_KEY` | `C` | `sec` | |
| `TEXTBEE_API_URL` | — | `cfg` | prod (empty = feature off) |

### AI / LLM

| Var | DEV | PROD | Notes |
|-----|-----|------|-------|
| `LLM_API_KEY` | `C` | `sec` | |
| `LLM_API_URL` | `C` | `cfg` | prod `https://api.z.ai/api/anthropic/v1/messages` |
| `LLM_API_MODEL` | `C` | `cfg` | prod `GLM-5.2` |
| `ZAI_API_KEY` | `C` | — | dev tooling |

### Infra / ops (mostly dev tooling, not deployed)

| Var | DEV | PROD | Notes |
|-----|-----|------|-------|
| `GITHUB_API_TOKEN`, `GITHUB_TEST_PAT` | `C` | — | dev/CI |
| `HETZNER_API_TOKEN` | `C` | — | dev (provider tests) |
| `PROXMOX_SSH` | `C` | — | dev |
| `EMAIL_BATCH_SIZE`, `EMAIL_PROCESSOR_INTERVAL_SECS` | `C` | — | dev tuning |
| `PG_HOST/PORT/USER/PASSWORD/DB`, `TEST_DATABASE_URL` | `P` | — | local sidecar |

---

## Edit + apply recipes

### DEV (docker-compose)

```bash
# set one value
./scripts/dc-secrets set shared/dev KEY=VALUE
# interactive edit (common or dev layer)
./scripts/dc-secrets edit shared/dev
# redeploy (rebuilds + compose up)
python3 cf/deploy.py deploy dev
```
Layers merge as `common` + `dev` (or `common` + `play` for local). Prod values must
**never** be put in these files.

### PROD (k3s / ArgoCD, namespace `dc-prod`)

All prod k8s objects live in the **`third_party/k8s`** repo. Non-secret config in
`cluster/apps/decent-cloud/dc-config.yaml` (ConfigMap); secrets in
`cluster/secrets/dc-secret.yaml` (PGP-SOPS).

```bash
cd third_party/k8s

# Non-secret value (ConfigMap) — edit + apply, ArgoCD will converge on next sync
$EDITOR cluster/apps/decent-cloud/dc-config.yaml
kubectl apply -f cluster/apps/decent-cloud/dc-config.yaml

# Secret value (SOPS) — edit, re-apply, restart the consuming pod(s)
sops cluster/secrets/dc-secret.yaml
python3 scripts/manage-secrets.py          # applies all cluster/secrets/*.yaml

# Restart whatever reads the changed key (dc-api reads most; chatwoot reads its own)
kubectl -n dc-prod rollout restart deploy/dc-api deploy/dc-api-sync deploy/dc-chatwoot-web deploy/dc-chatwoot-worker
```

`manage-secrets.py` discovers secrets by globbing `cluster/secrets/*.yaml` (filename
agnostic). Committing + pushing the k8s repo lets ArgoCD reconcile `dc-config` /
the manifests; `dc-secret` itself is applied manually (out-of-band) and never stored
in plaintext in git.

---

## Cloudflare tunnel (prod)

The prod tunnel (`decent-cloud`, id `2b53a68f-95a8-410a-b086-fea100dcb8b5`) is
**local-managed** (`config_src=local`): Cloudflare stores no remote ingress config, so
the routing lives entirely in the cluster as a ConfigMap —
`cluster/apps/decent-cloud/dc-cloudflared-config.yaml`. Connector credentials are the
`CLOUDFLARED_CREDS_JSON` key of `dc-secret` (a compact `{a,s,t}` JSON mounted at
`/etc/cloudflared/creds.json`).

**Renaming a Service no longer requires touching the Cloudflare API.** Just edit the
ConfigMap ingress + restart the cloudflared pod:
```bash
cd third_party/k8s
$EDITOR cluster/apps/decent-cloud/dc-cloudflared-config.yaml   # update host→service mapping
kubectl apply -f cluster/apps/decent-cloud/dc-cloudflared-config.yaml
kubectl -n dc-prod rollout restart deploy/dc-cloudflared
```
The DNS CNAMEs (`decent-cloud.org`, `api.decent-cloud.org`, `support.decent-cloud.org`
→ `<tunnel-id>.cfargotunnel.com`) only change if the tunnel itself is recreated.

DEV uses a **remote-managed** tunnel via `cf/tunnel.py dev` (compose-based) — that path
remains; the prod path in `tunnel.py` is removed (no longer needed under local-managed).

### Recreating the prod tunnel (creds rotation / new tunnel id)

Requires a CF token with Cloudflare Tunnel:Edit + DNS:Edit (the `CF_API_TOKEN` in
`dc-secret` has these). High-level (causes ~30s downtime during the CNAME swap):
1. Create new local-managed tunnel (`POST /accounts/$CF_ACCOUNT_ID/cfd_tunnel` with
   `config_src:local` + a fresh `tunnel_secret`).
2. Build the compact creds JSON, update `dc-secret` key `CLOUDFLARED_CREDS_JSON`
   (SOPS) and the `tunnel:` UUID in `dc-cloudflared-config.yaml`; apply + restart pod.
3. Flip the 3 DNS CNAMEs to the new `<tunnel-id>.cfargotunnel.com`.
4. Verify health, then delete the old tunnel.

---

## See also
- `cf/DEPLOYMENT_CONFIG.md` — deploy topology + runbooks (dev compose, prod k8s).
- `api/.env.example`, `cf/.env.example` — annotated env templates (dev authoring).
- k8s repo `docs/SECRETS_MANAGEMENT_SOPS.md` — SOPS reference (prod).
