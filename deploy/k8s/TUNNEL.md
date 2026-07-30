# Cloudflare Tunnel token — generation & rotation

The `cloudflared` Deployment in `decent-cloud.yaml` connects out to Cloudflare
via a **remotely-managed** tunnel. It authenticates with a single **connector
token** stored as the `TUNNEL_TOKEN_PROD` key of the `decent-cloud-secret`
Secret. This doc explains how to obtain and rotate that token.

`cf/tunnel.py` is the single source of truth for tunnel names + hostname→service
routing. It uses only the Python stdlib and talks to the Cloudflare API directly.

> ⚠️ Connector tokens are printed **only once**, at tunnel creation. After that
> Cloudflare does not expose them again — capture the token immediately and store
> it. (Re-running the script on an existing tunnel re-applies DNS/ingress and
> prints an **empty** token; reuse the stored one.)

## Prerequisites (credentials)

| Env var | Where it lives | Scope |
|---|---|---|
| `CF_API_TOKEN` | `dc-secrets` `shared/common` (mirrored as a GitHub Actions secret for CI) | Cloudflare API — tunnel + DNS write on the `decent-cloud.org` zone |
| `CF_ACCOUNT_ID` | `dc-secrets` `shared/common` (mirrored as a GitHub Actions secret for CI) | Cloudflare account id |

Both live in the product repo's AGE-SOPS store (`dc-secrets`), so `cf/tunnel.py`
runs from any shell with the age key after `eval "$(scripts/dc-secrets export
common)"`. In CI the same values come from the GitHub Actions secrets (which
mirror the dc-secrets store). The *connector token itself* (the value `cloudflared`
runs with) lives in the nuc-k3s PGP-SOPS store as the `TUNNEL_TOKEN_PROD` key of
`decent-cloud-secret` (see [SETUP.md §3](./SETUP.md#3-app-secret-decent-cloud-secret)).
You only need `cf/tunnel.py prod` to (re)configure that tunnel's **ingress** to
route to the in-cluster k8s Services — run it once, from CI or locally with the
creds.

## Generate `TUNNEL_TOKEN_PROD` (prod)

Pick whichever path has the CF credentials:

### (a) In a GitHub Actions step that has the secrets

Add a step to a workflow that can read `CF_API_TOKEN` + `CF_ACCOUNT_ID`:

```yaml
- name: Generate prod tunnel token
  env:
    CF_API_TOKEN: ${{ secrets.CF_API_TOKEN }}
    CF_ACCOUNT_ID: ${{ secrets.CF_ACCOUNT_ID }}
  run: python3 cf/tunnel.py prod --json
```

`--json` prints `{"id": "<tunnel-id>", "token": "<connector-token>"}` to stdout.
The token will be masked by GitHub Actions' secret masking only if it is also
stored as a secret; otherwise **capture it from the run logs** the first time
(the run logs of a freshly-created token are the only place it appears). On
subsequent runs the script prints `"token": ""` — reuse the stored token.

### (b) Locally, as the operator

```sh
git clone https://github.com/decent-stuff/decent-cloud && cd decent-cloud
eval "$(scripts/dc-secrets export common)" && python3 cf/tunnel.py prod
```

The connector token is printed to stdout (capture it). Progress/diagnostics go to
stderr.

### What the script does (idempotent)

For `prod` it targets tunnel **`decent-cloud`** (the existing live prod tunnel,
reused rather than creating a duplicate `decent-cloud-prod`) and:

1. **Creates** the tunnel if it does not exist (this is the only step that
   returns a token). Reuses it otherwise.
2. **Applies ingress rules** routing each hostname to its in-cluster Service
   (namespace `apps`, port 80):
   - `decent-cloud.org`        → `http://website.apps.svc.cluster.local:80`
   - `api.decent-cloud.org`    → `http://api.apps.svc.cluster.local:80`
   - `support.decent-cloud.org`→ `http://chatwoot-web.apps.svc.cluster.local:80`
   - catch-all → `http_status:404`
3. **Upserts CNAME DNS records** for the three hostnames →
   `<tunnel-id>.cfargotunnel.com` (proxied), in the `decent-cloud.org` zone.

### Land the token in the cluster Secret

The token must end up in the `TUNNEL_TOKEN_PROD` key of the PGP-SOPS-encrypted
`decent-cloud-secret` in the nuc-k3s repo. Edit the encrypted secret directly
(on a host with the cluster PGP key) and paste the captured token, then apply:

```sh
cd /project/decent-cloud/third_party/nuc-k3s
sops cluster/secrets/decent-cloud-secret.yaml   # set TUNNEL_TOKEN_PROD, save
python3 scripts/manage-secrets.py
```

Full secret workflow: see [SETUP.md](./SETUP.md) §3.

## Generate `TUNNEL_TOKEN_DEV` (dev / local docker-compose)

Same flow with `dev`, which targets tunnel **`decent-cloud-dev`** and routes the
`dev-*.decent-cloud.org` hostnames to the **local docker-compose** service names
+ raw ports (not in-cluster Services):

```sh
CF_API_TOKEN=... CF_ACCOUNT_ID=... python3 cf/tunnel.py dev
```

Store the result as the dev tunnel token (used by the local compose stack, not by
the k3s manifests).

## Rotate the prod token

Connector tokens cannot be re-read after creation. To rotate:

1. **Delete** the tunnel `decent-cloud` (Cloudflare dashboard → Zero Trust →
   Tunnels, or via the API). This invalidates the old token.
2. **Re-create** it (the DNS CNAMEs and ingress are re-applied automatically):
   ```sh
   CF_API_TOKEN=... CF_ACCOUNT_ID=... python3 cf/tunnel.py prod
   ```
   A fresh connector token is printed — capture it.
3. **Update `TUNNEL_TOKEN_PROD`** in the encrypted `decent-cloud-secret`
   (`sops cluster/secrets/decent-cloud-secret.yaml`, replace the value, save) and
   re-apply:
   ```sh
   python3 scripts/manage-secrets.py
   ```
4. **Restart the connector** so it picks up the new token:
   ```sh
   kubectl rollout restart deployment/cloudflared -n apps
   kubectl -n apps rollout status deployment/cloudflared
   ```
5. Verify: `curl -I https://decent-cloud.org/health` returns 200 shortly after.

## See also

- [SETUP.md](./SETUP.md) — consolidated operator runbook (one-time setup + image flow).
- `cf/tunnel.py` — the script this doc describes (single source of truth for routing).
