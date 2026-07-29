# Decent Cloud — Kubernetes deployment

Production manifests for the `decent-cloud` stack on the `nuc-k3s` cluster
(k3s + ArgoCD + Traefik + cert-manager + SOPS).

> **Operator runbook:** see **[SETUP.md](./SETUP.md)** — the single source of
> truth for one-time setup + the ongoing image-release flow. Tunnel token
> generation/rotation is in **[TUNNEL.md](./TUNNEL.md)**.

## Layout

This repo owns the app manifests. The cluster-level operator artifacts (ArgoCD
Application CR + PGP-SOPS secrets) live in the **`nuc-k3s` repo clone** at
`third_party/nuc-k3s/` — that is the single source for operator artifacts:

```
deploy/k8s/                                 # owned by THIS repo (synced by ArgoCD)
├── decent-cloud.yaml                       # Deployments + Job + Services
├── decent-cloud-secret.yaml.template       # documents every secret key (NO real values)
├── SETUP.md                                # consolidated operator runbook
├── TUNNEL.md                               # CF tunnel token generation + rotation
└── README.md                               # this file

third_party/nuc-k3s/cluster/                # operator artifacts (the nuc-k3s repo)
├── argocd/application-decent-cloud.yaml    # ArgoCD Application CR
└── secrets/
    ├── decent-cloud-secret.yaml.template           # 37 keys (docs; operator fills + encrypts)
    └── forgejo-registry-secret.yaml.template       # dockerconfigjson for git.kalaj.org
```

**Why split?** Decent-cloud owns its own manifests (`deploy/k8s/`). The nuc-k3s
cluster repo owns cluster-wide concerns: the ArgoCD Application CR that points at
this repo, and the PGP-SOPS-encrypted secret (the cluster's SOPS key is PGP,
while this repo's own `secrets/` use AGE — different key types, so the live
cluster secret must live in nuc-k3s). The secret values are generated from this
repo's AGE-SOPS store by [`scripts/gen-prod-secret.py`](../../scripts/gen-prod-secret.py).

## Services (all in namespace `apps`)

| Service | Image | Listens | Service port |
|---|---|---|---|
| `api` | `git.kalaj.org/decent-stuff/decent-cloud-api:<tag>` | 59001 | 80 → http |
| `api-sync` | same as api (`api-server sync`) | — | — |
| `website` | `git.kalaj.org/decent-stuff/decent-cloud-website:<tag>` | **59010** | 80 → http |
| `chatwoot-web` | `chatwoot/chatwoot:v4.8.0` | 59102 | 80 → http |
| `chatwoot-worker` | `chatwoot/chatwoot:v4.8.0` (sidekiq) | — | — |
| `chatwoot-migrate` | `chatwoot/chatwoot:v4.8.0` (Job, ArgoCD Sync hook) | — | — |
| `decent-cloud-redis` | `redis:8-alpine` | 6379 | 6379 → redis |
| `cloudflared` | `cloudflare/cloudflared:latest` | — (outbound) | — |

No Postgres pod: pods reach the host directly at `192.168.0.2:5432` (DBs
`decent_cloud_prod` + `chatwoot_prod` already exist there).

No Ingress: `decent-cloud.org` is a separate zone exposed by the `cloudflared`
Deployment (a Cloudflare Tunnel). Tunnel ingress is configured remotely via the
Cloudflare API by `cf/tunnel.py prod` (see TUNNEL.md).

## ArgoCD wiring

The Application CR
([`third_party/nuc-k3s/cluster/argocd/application-decent-cloud.yaml`](../../../third_party/nuc-k3s/cluster/argocd/application-decent-cloud.yaml))
points at `github.com/decent-stuff/decent-cloud` path `deploy/k8s`, destination
namespace `apps`, auto-sync `selfHeal + prune`, `syncOptions:
[CreateNamespace=true, ApplyOutOfSyncOnly=true]`, with `ignoreDifferences` for
every Deployment `/status` and every Service `clusterIP`/`nodePort` (mimics the
other Application CRs in the cluster).

## Image tag flow

See [SETUP.md §8](./SETUP.md#8-release--image-update-flow-repeatable). In short:
a `vX.Y.Z` tag triggers the `deploy-prod` job in `.github/workflows/release.yml`,
which builds + pushes both images to Forgejo, bumps the two image lines tagged
`# deploy-prod:api` / `# deploy-prod:website` in `deploy/k8s/decent-cloud.yaml`,
and pushes to `main`; ArgoCD auto-syncs the new tags.

## Conventions matched (nuc-k3s)

- namespace `apps`; `enableServiceLinks: false`; labels
  `app.kubernetes.io/name` + `app.kubernetes.io/component`.
- `resources.requests/limits`; readiness/liveness/startup probes;
  `strategy: Recreate` for singletons.
- `hostPath` volumes → `/home/sat/apps/decent-cloud/<dir>`
  (`type: DirectoryOrCreate`).
- Services: ClusterIP, port 80 → named `targetPort`.
- Secrets via `valueFrom.secretKeyRef` from `decent-cloud-secret`.
- Migration as an ArgoCD `Sync` hook Job (re-runs each sync, idempotent).
