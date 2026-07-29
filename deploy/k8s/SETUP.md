# Decent Cloud — k3s production setup runbook

Consolidated operator runbook for bringing up `decent-cloud` on the `nuc-k3s`
cluster (k3s + ArgoCD + SOPS) and the ongoing image-release flow. This is the
**single source of truth**; everything else under `deploy/k8s/` is referenced
from here.

**Split of concerns.** The product repo (`decent-stuff/decent-cloud`) owns the
app manifests under `deploy/k8s/`. The cluster repo (`nuc-k3s`) owns the
cluster-level artifacts: the ArgoCD Application CR and the PGP-SOPS-encrypted
secrets (the cluster's SOPS key is PGP
`FA5814CF1935EE80C454C9F1660DCCF069EC9176`; the product repo's own `secrets/`
use AGE — different key types, so the live cluster secret must live in nuc-k3s).
Those operator artifacts are committed in `third_party/nuc-k3s/cluster/`.

Work through steps 1–6 once. Step 8 is the repeatable release flow.

---

## 1. Forgejo packages + imagePullSecret

The api/api-sync/website images live in the Forgejo OCI registry under owner
**`decent-stuff`**:

- `git.kalaj.org/decent-stuff/decent-cloud-api:<tag>`
- `git.kalaj.org/decent-stuff/decent-cloud-website:<tag>`

(CI rewrites these lines on each release — the `v0.1.0` in the manifest is a
pre-release placeholder.)

Build the `forgejo-registry-secret` (a `kubernetes.io/dockerconfigjson` Secret)
from the template
[`third_party/nuc-k3s/cluster/secrets/forgejo-registry-secret.yaml.template`](../../../third_party/nuc-k3s/cluster/secrets/forgejo-registry-secret.yaml.template):

```sh
# auth = base64("<user>:<token>"); the .dockerconfigjson value is the JSON object
python3 - <<'PY'
import base64, json
user, token = "sat", "FORGEJO_TOKEN"          # fill the token
auth = base64.b64encode(f"{user}:{token}".encode()).decode()
print(json.dumps({"auths":{"git.kalaj.org":{"auth":auth,"username":user,"password":token}}}))
PY
```

Paste the printed JSON as `.dockerconfigjson`, then encrypt + apply:

```sh
cd /project/decent-cloud/third_party/nuc-k3s
sops --encrypt --pgp FA5814CF1935EE80C454C9F1660DCCF069EC9176 \
  --in-place cluster/secrets/forgejo-registry-secret.yaml
python3 scripts/manage-secrets.py
```

The api/api-sync/website Deployments already reference it via
`imagePullSecrets: [{name: forgejo-registry-secret}]` (chatwoot/redis/cloudflared
use upstream public images → no pull secret).

## 2. Cloudflare tunnel token

Generate `TUNNEL_TOKEN_PROD` (needs `CF_API_TOKEN` + `CF_ACCOUNT_ID`, which are
GitHub-only — not in `dc-secrets`). Full instructions: **[TUNNEL.md](./TUNNEL.md)**.

```sh
CF_API_TOKEN=... CF_ACCOUNT_ID=... python3 cf/tunnel.py prod   # prints connector token once
```

This creates tunnel `decent-cloud-prod` + CNAMEs (`decent-cloud.org`,
`api.decent-cloud.org`, `support.decent-cloud.org` → `<tunnel>.cfargotunnel.com`)
and ingress rules to the in-cluster Services. **Capture the token** — it is
printed only at creation.

## 3. App secret (`decent-cloud-secret`)

Generate the plaintext Secret from the product repo's AGE-SOPS store, drop in the
real tunnel token from step 2, then PGP-encrypt for the cluster:

```sh
cd /project/decent-cloud/repo
# 1. Emit all 37 keys (TUNNEL_TOKEN_PROD is reused from dc-secrets TUNNEL_TOKEN; cross-checked vs template)
scripts/gen-prod-secret.py > /tmp/dc.yaml
#    (optional) only if you created a NEW dedicated prod tunnel with 'cf/tunnel.py prod':
#    edit TUNNEL_TOKEN_PROD in /tmp/dc.yaml to the new token it printed.
# 2. PGP-encrypt straight into the nuc-k3s cluster secrets dir, then clean up
sops --encrypt --pgp FA5814CF1935EE80C454C9F1660DCCF069EC9176 /tmp/dc.yaml \
  > /project/decent-cloud/third_party/nuc-k3s/cluster/secrets/decent-cloud-secret.yaml
rm /tmp/dc.yaml
```

Apply to the cluster (run from the nuc-k3s repo root):

```sh
cd /project/decent-cloud/third_party/nuc-k3s
python3 scripts/manage-secrets.py            # decrypts + applies every SOPS secret in cluster/secrets/
# or, to also re-sync the ArgoCD apps that depend on secrets:
scripts/apply-secrets-and-sync.sh
```

Keys are documented in
[`cluster/secrets/decent-cloud-secret.yaml.template`](../../../third_party/nuc-k3s/cluster/secrets/decent-cloud-secret.yaml.template)
(nuc-k3s) / [`decent-cloud-secret.yaml.template`](./decent-cloud-secret.yaml.template)
(this repo). To **rotate** the tunnel token later see [TUNNEL.md](./TUNNEL.md#rotate-the-prod-token).

## 4. ArgoCD Application CR

Commit the Application CR into the nuc-k3s repo (it already lives in the clone at
[`third_party/nuc-k3s/cluster/argocd/application-decent-cloud.yaml`](../../../third_party/nuc-k3s/cluster/argocd/application-decent-cloud.yaml)):

- `source.repoURL`: `https://github.com/decent-stuff/decent-cloud.git` (public → HTTPS, no deploy key)
- `source.path`: `deploy/k8s`, `targetRevision`: `main`
- `destination.namespace`: `apps`
- `syncPolicy.automated`: `{selfHeal: true, prune: true}`
- `syncOptions`: `[CreateNamespace=true, ApplyOutOfSyncOnly=true]`

ArgoCD then syncs `deploy/k8s` from decent-cloud `main` and reconciles all
resources in namespace `apps`.

## 5. Node hostPath directories (on the k3s node)

```sh
sudo mkdir -p /home/sat/apps/decent-cloud/api-data/ledger
sudo mkdir -p /home/sat/apps/decent-cloud/redis
sudo chown -R 1000:1000 /home/sat/apps/decent-cloud/api-data   # api runs as UID 1000
```

## 6. GitHub repo secrets (`decent-stuff/decent-cloud`)

Required by the `deploy-prod` job in `.github/workflows/release.yml` (+ tunnel
generation). **Current state (all set):**

| Secret | Purpose | Set? |
|---|---|---|
| `FORGEJO_OWNER` | image owner/namespace (`decent-stuff`) | ✅ |
| `FORGEJO_USER` | docker login user (`sat`) | ✅ |
| `FORGEJO_TOKEN` | robot token with package write rights | ✅ |
| `DC_REPO_WRITE` | PAT to push the image-tag bump commit to `main` | ✅ |
| `CF_API_TOKEN` | Cloudflare API token (tunnel + DNS) | ✅ |
| `CF_ACCOUNT_ID` | Cloudflare account id (tunnel) | ✅ |
| `SOPS_AGE_KEY` | AGE key for the product repo's `dc-secrets` store | ✅ |

(`CF_API_TOKEN` + `CF_ACCOUNT_ID` are GitHub-only — they are intentionally NOT in
`dc-secrets`, so the tunnel token must be generated by an operator or a CI step
holding them; see [TUNNEL.md](./TUNNEL.md).)

## 7. Databases already exist

No DB provisioning. Both databases already exist on the host Postgres
(`192.168.0.2:5432`) with data:

- `decent_cloud_prod` — central API DB (user from `PG_USER`, password `PG_PASSWORD`)
- `chatwoot_prod` — Chatwoot DB (password `CHATWOOT_POSTGRES_PASSWORD`)

Pods reach the host directly; there is no Postgres pod.

## 8. Release / image-update flow (repeatable)

```
git tag vX.Y.Z  ──▶  release.yml  ──▶  deploy-prod job
                                        ├─ build api + website
                                        ├─ docker push git.kalaj.org/decent-stuff/...
                                        ├─ bump image tags in deploy/k8s/decent-cloud.yaml
                                        └─ git push (DC_REPO_WRITE)
                                                        │
                                  ArgoCD sees new main ──▶ sync new tags ──▶ rolling update
```

1. Cut a `vX.Y.Z` tag.
2. CI `deploy-prod` builds `decent-cloud-api` + `decent-cloud-website`, pushes
   them to `git.kalaj.org/decent-stuff/...`, bumps the two image lines tagged
   `# deploy-prod:api` / `# deploy-prod:website` in `deploy/k8s/decent-cloud.yaml`,
   and pushes to `main`.
3. ArgoCD auto-syncs the new tags → rolling update of `api`/`api-sync`/`website`.

## 9. Verify

```sh
kubectl -n apps get pods -l app.kubernetes.io/name in (api,api-sync,website,chatwoot-web,chatwoot-worker,decent-cloud-redis,cloudflared)
# (or simply)
kubectl -n apps get pods,svc -l app.kubernetes.io/name
curl -i https://decent-cloud.org/health            # website (nginx) health
curl -i https://api.decent-cloud.org/api/v1/health # API health
# support portal
curl -i https://support.decent-cloud.org/api       # Chatwoot (expect 200/redirect)
```

All three public hostnames are served by the `cloudflared` tunnel; if any 5xx,
check `kubectl -n apps logs deploy/cloudflared` and the tunnel ingress in
[TUNNEL.md](./TUNNEL.md).

---

## See also

- [`decent-cloud.yaml`](./decent-cloud.yaml) — the manifests ArgoCD syncs.
- [`TUNNEL.md`](./TUNNEL.md) — tunnel token generation & rotation.
- [`decent-cloud-secret.yaml.template`](./decent-cloud-secret.yaml.template) — the 37 secret keys (no values).
- [`README.md`](./README.md) — layout + service/port reference.
