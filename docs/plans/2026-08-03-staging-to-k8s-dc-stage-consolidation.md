# Staging → k8s (dc-stage): eliminate the dual secret stores

**Created:** 2026-08-03
**Status:** Agreed — do next
**Related:** `cf/CONFIG.md` (per-var secret reference), `cf/DEPLOYMENT_CONFIG.md`,
GitHub issues #451, #452, #453

## Goal

One secret store for the whole project. The shared staging environment (today
called "dev") moves off the local docker-compose stack + the
`repo/secrets/shared/` age-SOPS store and onto the k8s cluster (namespace
`dc-stage`), with its secrets in the nuc-k3s repo (PGP-SOPS) alongside prod.
After that, `repo/secrets/shared/` and `scripts/dc-secrets` are deleted and all
now-obsolete dev/prod compose code is cleaned up.

**Naming** (rename for clarity, agreed):

| tier | namespace | concept | secrets |
|------|-----------|---------|---------|
| production | `dc-prod` (exists) | prod | `dc-secret` + `dc-config` |
| staging | `dc-stage` (new) | the shared staging env (was "dev") | `dc-stage-secret` + `dc-stage-config` |
| dev | local docker-compose | **merged** old `play` + old `dev` local loop (AI agent) | plaintext local config (no SOPS) |

The slim local compose stays for the AI agent to iterate quickly. The *full*
"dev deployment" (Chatwoot/Redis/cloudflared/the works) is what becomes `dc-stage`
on k8s and is removed locally.

**`play` + `dev` merge into one local env.** The old dc-secrets had three layers;
after the split:
- `play.yaml` (local PG sidecar: ports, `DATABASE_URL`, test `PG_*` creds, test
  `CREDENTIAL_ENCRYPTION_KEY`, `CANISTER_ID`) — all **local/non-secret** → folds
  into the single local dev config (plaintext, committed).
- `dev.yaml` **real** creds (`CHATWOOT_*`, `STRIPE_*`, `GOOGLE_OAUTH_*`,
  `SMTP_*`, `TELEGRAM_*`, `TUNNEL_TOKEN`) — these exercise real external services
  → migrate to `dc-stage-secret` (staging is where those flows run, not local).
- `common.yaml` env-agnostic infra keys (`CF_*`, `LLM_*`, `DKIM_*`, etc.) →
  migrate to `dc-stage-secret` where staging needs them; prod already has its own.
- Net: local dev needs NO SOPS store. Its config is plaintext env vars set
  directly in the slim docker-compose service definitions (DB URL/password,
  ports, test `CREDENTIAL_ENCRYPTION_KEY`, `CANISTER_ID`, `RATE_LIMIT_ENABLED`,
  dummy `STRIPE_WEBHOOK_SECRET`) — committed, no `.env.local`, no SOPS, nothing
  gitignored.

## Local dev needs no secrets (verified)

Empirically confirmed from `scripts/dev-server.sh` (already runs the local
stack for e2e): the API + website + Postgres run on plaintext/test values only —

```
DATABASE_URL=<local PG>  API_SERVER_PORT  FRONTEND_URL=http://localhost:…
SQLX_OFFLINE=true  CANISTER_ID=ggi4a-wyaaa-aaaai-actqq-cai
RATE_LIMIT_ENABLED=false  STRIPE_WEBHOOK_SECRET=whsec_test_secret   # dummy
```

`dev-server.sh:113-119` only *optionally* enriches from `dc-secrets export play`
and prints "will run WITHOUT SOPS secrets" + continues if absent — so the core
loop already works secret-free. The API **starts** without Stripe/OAuth/Chatwoot/
SMTP creds: those are lazy (OAuth errors only when the flow is invoked; other
features `tracing::warn!` and disable, per the repo's "BE LOUD ABOUT
MISCONFIGURATIONS" rule).

Conclusion: local iteration (API/website/e2e with `--skip-payment`, seed-phrase
auth) needs **no secrets at all**. Integration flows that need real test-tier
creds (live Stripe checkout, Google OAuth, email sending) are exercised in
`dc-stage`, not locally. Therefore secrets are dropped **officially and
completely** for local dev — any needed variables (DB URL/pass, etc.) become
plaintext docker-compose env vars.

## Why (the problem)

Two independent secret stores coexist today:

| store | env | repo | key type |
|-------|-----|------|----------|
| `repo/secrets/shared/{common,dev,play}.yaml` | staging ("dev") | product repo (`repo/`) | age |
| `third_party/k8s/cluster/secrets/dc-secret.yaml` | prod | nuc-k3s repo | PGP |

Symptoms that make this untenable:

- **Two sources of truth** for keys that should be identical. Five keys are
  genuinely duplicated across both stores and can drift: `CF_API_TOKEN`,
  `DKIM_PRIVATE_KEY`, `LLM_API_KEY`, `MAILCHANNELS_API_KEY`, `TEXTBEE_API_KEY`.
- **The age key is already broken on the dev host.** `common.yaml` + `play.yaml`
  were re-encrypted by `897a90e5` to recipient `age1vdj457…`, which the local
  `.age-identity` does not match → "no identity matched any of the recipients".
  Only `dev.yaml` still decrypts. The staging store is *partially unreadable*.
- **Scattered tooling.** 25 tracked files reference `dc-secrets` / `secrets/shared`
  (AGENTS.md, `cf/{CONFIG,DEPLOYMENT_CONFIG}.md`, `cf/deploy.py`,
  `cf/docker-compose.dev.yml`, `api/.env.example`, `scripts/{dc-secrets,
  dev-server.sh,test_dc_secrets.py}`, `docs/*`, e2e setup docs…).
- prod already proved the model: PGP-SOPS in nuc-k3s, applied by
  `manage-secrets.py`, edit via `sops cluster/secrets/dc-secret.yaml`. Staging
  should match.

## Current staging topology (what is being replaced)

Local docker-compose, driven by `cf/deploy.py deploy dev`:

- `cf/docker-compose.dev.yml` services: `api-serve` (rust api-server), `website`
  (vite), `postgres`, `redis`, `chatwoot-web`, `chatwoot-worker`, `cloudflared`
  (dev tunnel, remote-managed).
- Secrets loaded by `cf/deploy.py load_secrets_from_sops` →
  `scripts/dc-secrets export dev` (merges `common.yaml` + `dev.yaml`) → exported
  as env vars consumed by compose.
- Dev tunnel `decent-cloud-dev` is **remote-managed** via `cf/tunnel.py dev`
  (the only remaining use of `tunnel.py`; prod is local-managed). Routes
  `dev-support.decent-cloud.org` / `dev-api.decent-cloud.org` etc. → compose
  service targets (`website:59000`, `api-serve:59001`, `chatwoot-web:59002`).

## Plan

### Phase 1 — Refactor prod into a shared base + overlay (max reuse)

The goal is `dc-stage` reusing as much of `dc-prod`'s config as possible. Use a
**kustomize base + overlays** layout so the two envs share one base and diverge
only in a small patch (namespace, image tags, hostPath, secret/config names,
replicas). The nuc-k8s repo has no overlay tooling yet — this introduces it.

1. Restructure the existing prod manifests:
   ```
   cluster/apps/decent-cloud/
   |- base/                      # shared (moved from current flat 7 files)
   |   |- kustomization.yaml     # resources: dc-config, dc-api, dc-website, ...
   |   `- dc-{config,api,website,chatwoot,redis,cloudflared,cloudflared-config}.yaml
   |- prod/                      # prod overlay (reproduces current live state EXACTLY)
   |   `- kustomization.yaml     # namespace: dc-prod, namePrefix/suffix none,
   |                              # patches: image tags, hostPath, secret names, replicas
   `- stage/                     # NEW stage overlay (the dc-stage env)
       `- kustomization.yaml     # namespace: dc-stage, stage image tags, stage hostPath,
                                 # dc-stage-secret/dc-stage-config names, replicas
   ```
   - Move the current 7 flat `dc-*.yaml` into `base/` with the env-agnostic
     parts (Deployment/Service/ConfigMap structure, ports, probes, labels).
     Strip env-specifics (hardcoded `namespace: dc-prod`, image tags, hostPath
     paths, secret/config names) into overlay patches.
   - `prod/kustomization.yaml` must render **byte-for-byte-equivalent** to the
     current live prod state (namespace `dc-prod`, current image tags
     `dc-api/dc-api-sync=445a17d4`, `dc-website=v0.5.5-hotfix.445a17d4`,
     hostPath `/home/sat/apps/decent-cloud/...`, `dc-secret`/`dc-config`).
     Verify with `kustomize build prod/` diffed against the current live specs
     before committing — prod is live, this refactor is zero-behavior-change.
   - Repoint the prod ArgoCD App CR `source.path`: `cluster/apps/decent-cloud`
     → `cluster/apps/decent-cloud/prod`. ArgoCD builds kustomize natively.
2. New namespace `cluster/core/dc-stage.yaml` (mirror `dc-prod.yaml`).
3. Author `stage/kustomization.yaml` overlay: `namespace: dc-stage`, stage image
   tags, stage-owned hostPath (NO shared volumes with prod — separate api-data
   + redis AOF), `dc-stage-secret`/`dc-stage-config` names. This is small — most
   config is inherited from `base/`.
4. Provision stage data stores (see Open Decisions: separate Postgres instance
   vs a `decent_cloud_stage` DB in the shared `pgsql` app; separate stage Redis;
   stage Chatwoot + its DB).
5. Create `cluster/secrets/dc-stage-secret.yaml` (PGP-SOPS, ns `dc-stage`) with
   the stage values migrated from `secrets/shared/dev.yaml` (+ the readable
   subset of `common.yaml`). Create `dc-stage-config` ConfigMap (or let the
   overlay generate it from a stage patch).
6. Second ArgoCD App CR `cluster/argocd/application-decent-cloud-stage.yaml`
   (source path `cluster/apps/decent-cloud/stage`, ns `dc-stage`), added to the
   `root` app-of-apps path.
7. Reconfigure the stage tunnel. Rename tunnel `decent-cloud-dev` →
   `decent-cloud-stage` (PATCH name via CF API), then either convert it to
   local-managed (a `dc-stage-cloudflared-config` ConfigMap, same pattern as
   prod) OR keep remote-managed and re-point its ingress at the
   `*.dc-stage.svc.cluster.local` FQDNs. Update DNS hostnames `dev-*` → `stage-*`
   (`stage-support.decent-cloud.org`, `stage-api.decent-cloud.org`, …) — new CF
   records + tunnel ingress entries.

### Phase 2 — Rewire stage tooling to k8s

7. Update `cf/deploy.py`: stage config reads from the cluster (kubectl `dc-stage`
   configmap/secret) instead of `dc-secrets export dev`. The `config` subcommand
   already introspects via kubectl — extend the same pattern and rename the env
   from `dev`→`stage`.
8. Update `cf/CONFIG.md` + `cf/DEPLOYMENT_CONFIG.md`: stage now sources from
   `dc-stage-config` / `dc-stage-secret` in nuc-k3s (one store per env, same
   repo). Dev = local slim compose.

### Phase 3 — Delete obsolete code + the age store

9. **Delete the full dev-deploy stack** (replaced by dc-stage on k8s):
   - `cf/docker-compose.dev.yml` (full staging stack)
   - `cf/deploy.py` compose lifecycle subcommands (`deploy`/`stop`/`logs`/
     `status`/`restart` for the full stack) — keep only the `config` subcommand
     (+ any slim-compose helper that remains; see step 11)
   - `cf/tunnel.py` + `cf/test_tunnel.py` entirely, once the stage tunnel is
     reconfigured (both prod and stage tunnels are then k8s-managed; no
     imperative CF API client left). NOTE: the `TUNNELS` dict in `tunnel.py` is
     the current single-source of tunnel routing — migrate that knowledge into
     the two ConfigMaps before deleting the file.
   - `cf/chatwoot.env.example`, `cf/.env.dev.example`, `cf/.env.example` if they
     only served the compose stack (verify they're not referenced elsewhere).
10. **Delete the age secret store + its tooling** (nothing of value is lost —
    see the play/dev merge above):
    - `repo/secrets/` (`shared/{common,dev,play}.yaml`, `.sops.yaml`, `.locks/`,
      `.age-identity`)
    - `scripts/dc-secrets` + its tests `scripts/test_dc_secrets.py`,
      `scripts/test-dc-secrets.sh`
    - First migrate: `dev.yaml` real creds + `common.yaml` infra keys →
      `dc-stage-secret`; `play.yaml` local values → the new plaintext local dev
      config.
11. **Keep the slim local compose for the AI agent inner loop** (the merged
    dev/play env): the existing `scripts/dev-server.sh` warm stack
    (api + website + postgres, used by e2e) is the fast-iteration path and stays.
    Move every variable it needs (DB URL/pass, ports, test
    `CREDENTIAL_ENCRYPTION_KEY`, `CANISTER_ID`, `RATE_LIMIT_ENABLED=false`,
    dummy `STRIPE_WEBHOOK_SECRET`) into the slim docker-compose service `env:`
    block as **plaintext** — committed, no SOPS, no `.env.local`, nothing
    gitignored (per the "local dev needs no secrets" finding above). Remove the
    `dc-secrets export play` call at `dev-server.sh:113-119`. If `cf/deploy.py`
    shrinks to just `config`, move the slim-compose orchestration (if any) into
    `scripts/dev-server.sh` to keep one entry point.
12. **Sweep the 25 referencing files**: repoint every `dc-secrets` /
    `secrets/shared` mention to the new k8s path or remove. Update `AGENTS.md`
    "Credentials" section (it currently documents the dc-secrets layer model
    verbatim). Update `api/.env.example`, `cf/.env.example`, and e2e setup docs
    to reflect stage=k8s / dev=local-slim.

## Open decisions (resolve before/while executing)

- **DB strategy.** Separate Postgres `StatefulSet` for dc-stage vs a
  `decent_cloud_stage` database inside the existing shared `pgsql` app
  (`cluster/apps/pgsql.yaml`). Prod uses its own Postgres at
  `192.168.0.2:5432/decent_cloud_prod`. Recommendation: a separate DB in the
  shared pgsql instance (cheapest, isolates data, matches how Chatwoot already
  shares the pgsql host).
- **Image strategy.** Stage image tags: `:stage-<sha>` per push, or floating
  `:stage`/`:main` that ArgoCD auto-syncs. Recommendation: `:stage` floating tag
  pushed by CI on every merge to `main` (or a `stage` branch), so dc-stage tracks
  latest without manual bumps.
- **Hostname rename.** `dev-*` → `stage-*` (`stage-support.decent-cloud.org`
  etc.). Cleanest for clarity but needs new CF DNS records + tunnel ingress
  entries + any hardcoded `dev-*` refs in code/docs. Alternative: keep `dev-*`
  hostnames pointing at dc-stage (no DNS change, less clear). Recommend the
  rename since the whole point is clarity.
- **Manifest reuse — RESOLVED: kustomize base + overlays.** `cluster/apps/
  decent-cloud/base/` is shared; `prod/` and `stage/` are thin overlays
  patching namespace/image-tags/hostPath/secret-names. Maximises reuse (one base
  to edit), avoids the sync burden of duplicating 7 manifests. Prod refactor
  must render byte-equivalent to current live state (verify before commit).

## Reference facts (carry forward)

- Prod manifests (the template to mirror): `third_party/k8s/cluster/apps/decent-cloud/`
  (7 files: `dc-config.yaml`, `dc-api.yaml`, `dc-website.yaml`, `dc-chatwoot.yaml`,
  `dc-redis.yaml`, `dc-cloudflared.yaml`, `dc-cloudflared-config.yaml`). App CR
  `cluster/argocd/application-decent-cloud.yaml` sources
  `git@github.com:sasa-tomic/nuc-k3s.git` path `cluster/apps/decent-cloud`, ns `dc-prod`.
- Prod secret edit flow: `cd third_party/k8s && sops cluster/secrets/dc-secret.yaml`
  → `python3 scripts/manage-secrets.py` → `kubectl -n dc-prod rollout restart
  deploy/<affected>`. PGP key `FA5814CF1935EE80C454C9F1660DCCF069EC9176`,
  `encrypted_regex: ^(data|stringData)$`.
- `manage-secrets.py` → `SecretsManager` (`src/nuc_k3s/secrets_manager.py`) does
  `rglob("*.yaml")` on `cluster/secrets/` — filename-agnostic, so a new
  `dc-stage-secret.yaml` is picked up automatically.
- Stage values to migrate live in `repo/secrets/shared/dev.yaml` (decryptable) +
  `common.yaml` (currently NOT decryptable on this host — see age-key note; the
  agent may need to recover or rotate values that overlap prod, since the 5
  shared keys are almost certainly identical to prod's).
- `cf/deploy.py config <env>` already introspects both envs via kubectl (prod)
  and dc-secrets (dev) — extend it rather than reinventing.
- Prod tunnel pattern (the model for a local-managed stage tunnel): ConfigMap
  `dc-cloudflared-config` is the single routing source; tunnel `decent-cloud`
  id `2b53a68f-…`, `config_src=local`. Stage tunnel `decent-cloud-dev` is
  currently remote-managed (rename → `decent-cloud-stage`).
- Slim local dev stack that STAYS: `scripts/dev-server.sh` (api+website+pg) +
  outer `agent/docker-compose.yml` (postgres sidecar). Self-contained, no
  `secrets/shared` dependency.

## Out of scope

- The 5 genuinely-redundant prod/stage keys become moot once stage is in nuc-k3s
  (they collapse into one value per env in the same repo). No separate rotation
  pass needed.
- Issues #451 (chatwoot service-account token), #452 (dead CHATWOOT_INBOX_ID),
  #453 (.sqlx CI build bug) are independent of this consolidation.
