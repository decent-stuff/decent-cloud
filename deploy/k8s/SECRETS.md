# Secrets: where they live and how to rotate them

One secret store per trust boundary, owned by the consumer that reads it. **Production
secrets are never committed to this (public) repository** — they live in the private
cluster repo. Dev/local secrets are here, AGE-encrypted.

| Environment | Secret store | Encryption | Location |
|-------------|--------------|------------|----------|
| **prod** (k8s) | cluster store, sole source | PGP-SOPS (key `FA5814CF1935EE80C454C9F1660DCCF069EC9176`) | `k8s` repo → `cluster/secrets/decent-cloud-secret.yaml` (private) |
| **dev** (docker-compose) | dc-secrets `shared/dev` layer | AGE-SOPS (repo `.age-identity`) | `secrets/shared/dev.yaml` (+ `common.yaml`) |
| **common** (shared by dev+local) | dc-secrets `shared/common` layer | AGE-SOPS | `secrets/shared/common.yaml` |
| **play** (local cargo/npm loop) | dc-secrets `shared/play` layer | AGE-SOPS | `secrets/shared/play.yaml` |

> The `prod` layer was **removed** from this repo on purpose (it is public). Do not
> re-add prod secrets here. Edit them in the k8s cluster store instead.

## Config vs Secret (12-Factor)

In prod (k8s), non-secret **configuration** is deliberately split out of the Secret
into a `ConfigMap` (`decent-cloud-config`, defined inline in `deploy/k8s/decent-cloud.yaml`).
True **secrets** stay in the `decent-cloud-secret` Secret. This means:

- Changing a **config** value (URL, model name, zone id, Stripe *publishable* key,
  DKIM domain/selector, SMTP host/user, Twilio/Textbee ids) = edit the ConfigMap in
  `deploy/k8s/decent-cloud.yaml`, push to `main`, ArgoCD syncs, pod picks it up on
  restart. **No secret re-apply/rotation needed.**
- Changing a **secret** value (DB url, *secret* keys, tokens, passwords) = rotate in
  the k8s PGP store + `manage-secrets.py` (see below).

> Note: the Stripe **publishable** key (`pk_*`) is ALSO baked into the website image at
> build time (Vite). Rotating it means updating the ConfigMap value AND rebuilding the
> website image (cut a release tag) so both the api-server env and the browser bundle agree.

---

## How to rotate — PROD (k8s)

Prod secrets live ONLY in `k8s/cluster/secrets/decent-cloud-secret.yaml`. The running
pods read a Kubernetes `Secret` that `manage-secrets.py` materializes from that file.

```bash
cd /project/decent-cloud/third_party/k8s
# 1. edit the decrypted values in $EDITOR (PGP key must be unlocked)
sops cluster/secrets/decent-cloud-secret.yaml
# 2. re-apply to the cluster + verify
python3 scripts/manage-secrets.py
# 3. restart the pods that read the changed key(s)
kubectl -n apps rollout restart deploy/api deploy/api-sync deploy/chatwoot-web deploy/chatwoot-worker
# (restart deploy/cloudflared too if you changed TUNNEL_TOKEN_PROD)
kubectl -n apps rollout status deploy/api   # wait for 1/1
```

> The website's publishable key (Stripe `pk_*`) is **baked at image build time**, not read
> from the Secret. Changing it needs a new website image + tag bump (see "Website keys" below).

## How to rotate — DEV (docker-compose) and LOCAL (play)

Dev/local secrets live in this repo's dc-secrets AGE store. Read it with the repo key
(agent container note: unset/override `DC_SECRETS_DIR` so it points at `repo/secrets`, not
the outer store — `export DC_SECRETS_DIR=/project/decent-cloud/repo/secrets`).

```bash
cd /project/decent-cloud/repo
# one key at a time
./scripts/dc-secrets set shared/dev STRIPE_SECRET_KEY=sk_test_xxx
# …or open the whole layer in $EDITOR
./scripts/dc-secrets edit shared/dev
# then re-deploy dev (docker-compose)
python3 cf/deploy.py deploy dev
```

---

## Stripe (the common one to rotate)

There are **three Stripe secrets per environment** (four in dev — the publishable key is
duplicated for the Vite build):

| Key | Used by | Env | Stripe mode | Store |
|-----|---------|-----|-------------|-------|
| `STRIPE_SECRET_KEY` | API (charges, server-side) | prod | **LIVE** (`sk_live_…`) | k8s PGP store |
| `STRIPE_PUBLISHABLE_KEY` | website (baked at build) | prod | **LIVE** (`pk_live_…`) | k8s PGP store |
| `STRIPE_WEBHOOK_SECRET` | API (verify webhook signatures) | prod | **LIVE** (`whsec_…`) | k8s PGP store |
| `STRIPE_SECRET_KEY` | API | dev | **TEST** (`sk_test_…`) | `secrets/shared/dev.yaml` |
| `STRIPE_PUBLISHABLE_KEY` / `VITE_STRIPE_PUBLISHABLE_KEY` | website build | dev | **TEST** (`pk_test_…`) | `secrets/shared/dev.yaml` |
| `STRIPE_WEBHOOK_SECRET` | API | dev | **TEST** (`whsec_…`) | `secrets/shared/dev.yaml` |

### Steps

1. **Get the new values** from the Stripe dashboard (`dashboard.stripe.com`):
   - Toggle the **mode** in the top-right: **Live** for prod, **Test** for dev.
   - *Secret + publishable keys*: **Developers → API keys** → reveal the **Secret key** (`sk_…`); copy the **Publishable key** (`pk_…`).
   - *Webhook signing secret*: **Developers → Webhooks** → open the endpoint for this env → **Signing secret** (`whsec_…`). The endpoint URL is:
     - prod: `https://api.decent-cloud.org/api/v1/webhooks/stripe`
     - dev:  `https://dev-api.decent-cloud.org/api/v1/webhooks/stripe`
2. **Write them to the store** (prod → k8s; dev → dc-secrets `shared/dev`), per the
   flows above.
3. **Redeploy**:
   - API keys (`STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`): restart the api pod — takes effect immediately.
   - Publishable key (`STRIPE_PUBLISHABLE_KEY`): needs a **new website image + tag bump** (it is compiled into the static build). For prod that is a release (`v*` tag → CI builds/pushes the website image → ArgoCD syncs); for dev, `cf/deploy.py deploy dev` rebuilds the website.
4. **Roll/revoke the old key** in Stripe: Developers → API keys → **Roll** the secret key. Confirm webhooks still verify after the cutover.

---

## Per-key rotation index (all secrets)

| Key | prod lives in | dev lives in | Rotate how | Restart / redeploy |
|-----|---------------|--------------|------------|--------------------|
| `STRIPE_SECRET_KEY` | k8s | `shared/dev` | Stripe dashboard (Live/Test) | api pod |
| `STRIPE_PUBLISHABLE_KEY` | k8s | `shared/dev` | Stripe dashboard | **website rebuild** |
| `STRIPE_WEBHOOK_SECRET` | k8s | `shared/dev` | Stripe webhook endpoint | api pod |
| `API_DATABASE_URL` (embeds DB pw) | k8s | `shared/dev` | `ALTER ROLE <user> WITH PASSWORD …` on host PG `192.168.0.2:5432`, then update the DSN | api + api-sync pods |
| `CHATWOOT_POSTGRES_PASSWORD` | k8s | `shared/dev` | `ALTER ROLE chatwoot_prod/dev …` | chatwoot-web + worker (+ re-run migrate) |
| `CF_API_TOKEN`, `CF_ACCOUNT_ID`, `CF_ZONE_ID` | k8s (CF_API_TOKEN/ZONE) | `shared/common` (all three) | Cloudflare dashboard → My Profile → API Tokens | api pod (DNS mgmt); `cf/tunnel.py` reads CF_API_TOKEN/ACCOUNT_ID |
| `TUNNEL_TOKEN_PROD` / `TUNNEL_TOKEN` | k8s (PROD) | `shared/dev` (dev) | `python3 cf/tunnel.py prod|dev` (regenerate connector token) | cloudflared pod |
| `LLM_API_KEY` | k8s | `shared/common` | provider console | api pod |
| `DKIM_PRIVATE_KEY` (+ `DKIM_SELECTOR/DOMAIN`) | k8s | `shared/common` | `openssl genrsa`/`dkim-keygen`; publish the public key in the zone's DKIM TXT | api pod (email signing) |
| `GOOGLE_OAUTH_CLIENT_SECRET` (+ `CLIENT_ID`, `REDIRECT_URL`) | k8s | `shared/dev` | Google Cloud Console → Credentials | api pod |
| `TELEGRAM_BOT_TOKEN` (+ `BOT_USERNAME`) | k8s | `shared/dev` | @BotFather → /revoke / /token | api pod (+ re-register webhook) |
| `SMTP_PASSWORD` (+ `SMTP_ADDRESS/USERNAME`) | k8s | `shared/dev` | SMTP provider | api pod |
| `MAILCHANNELS_API_KEY` | k8s | `shared/common` | MailChannels dashboard | api pod |
| `CREDENTIAL_ENCRYPTION_KEY` | k8s | `shared/dev` (+ `shared/play`) | `openssl rand -hex 32` | **caution: re-encryption of app-side data required** — api pod |
| `INVOICE_SELLER_IBAN` | k8s | — | bank | api pod |
| `CHATWOOT_*` tokens (`API_TOKEN`, `PLATFORM_API_TOKEN`, `HMAC_SECRET`, `SECRET_KEY_BASE`) | k8s | `shared/dev` | Chatwoot admin (per-token) | chatwoot-web + worker |

### Notes
- **prod** values: also run `python3 scripts/manage-secrets.py` in `k8s` after editing, then `kubectl rollout restart`.
- **dev** values: `cf/deploy.py deploy dev` re-reads `dc-secrets export dev` on each run, so a redeploy picks up new values.
- Always roll/revoke the **old** credential at its source after the new one is live and verified.
