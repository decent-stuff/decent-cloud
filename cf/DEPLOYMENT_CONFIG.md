# Deployment Configuration Guide

How decent-cloud is configured and deployed across **dev** and **prod**. For the
authoritative per-variable reference (where each env var lives, by environment),
see **[CONFIG.md](./CONFIG.md)** — this guide covers the *mechanism*.

Live introspection from this repo:

```bash
python3 cf/deploy.py config dev    # dev: reads dc-secrets age-SOPS layers
python3 cf/deploy.py config prod   # prod: reads live k8s dc-config ConfigMap + dc-secret
```

---

## Two deploy targets

| | **Dev** | **Prod** |
|---|---|---|
| Runtime | docker-compose (this repo, `cf/`) | k8s cluster (NUC), namespace `dc-prod` |
| Owner of deploy | `cf/deploy.py` (this repo) | ArgoCD GitOps (auto-sync, selfHeal+prune) |
| Manifests | `cf/docker-compose*.yml` (this repo) | `cluster/apps/decent-cloud/` in **`sasa-tomic/nuc-k3s`** repo |
| Secrets store | `secrets/shared/*.yaml` (this repo, SOPS **age**) | `cluster/secrets/dc-secret.yaml` (k8s repo, SOPS **PGP**) + `dc-config` ConfigMap |
| Secrets tool | `scripts/dc-secrets` (set/edit/export/list) | `sops` + `python3 scripts/manage-secrets.py` |
| Tunnel | remote-managed via `cf/tunnel.py dev` | local-managed (`dc-cloudflared-config` ConfigMap in the k8s repo) |

Dev and prod use **different** secret backends on purpose: the cluster's SOPS key
is PGP; this repo's own `secrets/` use age. Prod credentials live **only** in the
private k8s cluster repo and are never committed to this public repo.

---

## Dev

### Configure

```bash
scripts/dc-secrets set shared/dev GOOGLE_OAUTH_CLIENT_ID=...
scripts/dc-secrets edit shared/dev        # interactive, $EDITOR
scripts/dc-secrets list shared/dev
```

Layers: `shared/common.yaml` (env-agnostic) merged with `shared/dev.yaml` (dev
deploy). (There is also `shared/play.yaml` for the local API-server + Postgres
sidecar flow — see `api/.env.example` + the local-dev section of AGENTS.md.)

### Deploy

```bash
python3 cf/deploy.py deploy dev
```

Builds the website + API natively, brings up docker-compose, and (if
`TUNNEL_TOKEN` is present) registers the Telegram webhook. Dev tunnel is
remote-managed; recreate/rotate with `python3 cf/tunnel.py dev`.

---

## Prod

Prod is **not** deployed from this repo. ArgoCD watches the `sasa-tomic/nuc-k3s`
repo at path `cluster/apps/decent-cloud/` and reconciles into namespace `dc-prod`.

### Configure

Non-secret values (public ids, URLs, model names) → `dc-config` ConfigMap:

```bash
# in the k8s repo checkout:
$EDITOR cluster/apps/decent-cloud/dc-config.yaml     # edit values directly (plaintext)
kubectl -n dc-prod apply -f cluster/apps/decent-cloud/dc-config.yaml
```

Secrets → `dc-secret` (PGP-SOPS):

```bash
cd /path/to/nuc-k3s
sops cluster/secrets/dc-secret.yaml                  # edit, save (re-encrypts in place)
python3 scripts/manage-secrets.py                    # re-apply to cluster
kubectl -n dc-prod rollout restart deploy/dc-api deploy/dc-api-sync \
    deploy/dc-chatwoot-web deploy/dc-chatwoot-worker # restart pods that consume the changed key
```

`dc-config` (ConfigMap) changes need no restart for values consumed via
`configMapKeyRef`? **No** — they do, because env vars are resolved at pod start.
Restart the consuming deployment after a `dc-config` change too.

For the exact key → file → deployment mapping, run `python3 cf/deploy.py config
prod` or see [CONFIG.md](./CONFIG.md).

### Tunnel (local-managed)

The prod tunnel (`decent-cloud`) is **local-managed**: its ingress routing lives
in the `dc-cloudflared-config` ConfigMap (`cluster/apps/decent-cloud/dc-cloudflared-config.yaml`),
**not** in the Cloudflare API. Consequences:

- Renaming a Service = edit the ConfigMap + `kubectl -n dc-prod rollout restart
  deploy/dc-cloudflared`. No `tunnel.py`, no DNS change (CNAMEs target the stable
  `<tunnel-id>.cfargotunnel.com`).
- Recreating the tunnel (new connector) is the only thing that needs the
  Cloudflare API + a DNS CNAME flip.

### Image bump (release flow)

Releases are tagged (`vX.Y.Z`). The `release.yml` workflow builds the api +
website images, pushes them to the Forgejo registry, then bumps the image tags in
the k8s repo manifests:

- **CI (automatic):** clones `sasa-tomic/nuc-k3s` and runs
  `scripts/bump_app_images.py --app decent-cloud --tag vX.Y.Z --push`. Requires
  the `NUC_K3S_REPO_WRITE` GitHub secret (PAT with write access to the k8s repo). If
  the secret is absent, the step fails loudly with the manual fallback below.
- **Manual (operator):** from a k8s repo checkout —

  ```bash
  python3 scripts/bump_app_images.py --app decent-cloud --tag vX.Y.Z --push
  # (omit --push to review the diff first)
  ```

Both paths rewrite every `image:` line tagged with `# deploy-prod:<component>`
(`api` ×2 in `dc-api.yaml`, `website` ×1 in `dc-website.yaml`). ArgoCD then
auto-syncs the new tags.

---

## Security notes

### Separate OAuth apps

Always use **separate** Google OAuth clients for dev and prod. Redirect URIs:
- Dev: `http://localhost:59001/api/v1/oauth/google/callback`
- Prod: `https://api.decent-cloud.org/api/v1/oauth/google/callback`

Google rejects tokens if the redirect URI doesn't match **exactly**. The redirect
URI must also be listed in the Google Cloud Console → API credentials → Authorized
redirect URIs for the OAuth client (cluster config alone is not enough).

### Cookie security

The API enables the `Secure` cookie flag based on `FRONTEND_URL`:
- `http://` → Secure cookies **disabled** (dev)
- `https://` → Secure cookies **enabled** (prod)

### Encryption at rest

Both secret backends encrypt at rest: dev uses SOPS + age; prod uses SOPS + the
cluster PGP key. Encrypted files are safe to commit to their respective repos.

---

## Troubleshooting

### OAuth `redirect_uri_mismatch`

1. Check the configured value: `python3 cf/deploy.py config <env>` (look for
   `GOOGLE_OAUTH_REDIRECT_URL`), or `scripts/dc-secrets list shared/dev` (dev).
2. Verify the **same** URI is in Google Cloud Console → the OAuth client's
   Authorized redirect URIs (dev client vs prod client — don't mix them).
3. Watch for `http://` vs `https://`, trailing slash, and `/auth/` vs `/oauth/`.

### Cookies not secure in production

`FRONTEND_URL` (in `dc-config` for prod) must start with `https://`.

---

## See also

- [CONFIG.md](./CONFIG.md) — authoritative per-variable reference (dev + prod sources)
- [OAuth Authentication Guide](../docs/OAUTH_AUTHENTICATION.md)
- `api/.env.example`, `cf/.env.example` — variable documentation templates
- k8s repo: `cluster/apps/decent-cloud/` (manifests), `cluster/secrets/dc-secret.yaml` (prod secrets), `docs/SECRETS_MANAGEMENT_SOPS.md` (SOPS reference)
