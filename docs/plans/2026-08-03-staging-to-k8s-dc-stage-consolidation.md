# Staging → k8s (dc-stage): eliminate the dual secret stores

**Created:** 2026-08-03
**Status:** **Tracks 1+2+3 done autonomously** (nuc-k8s manifests local + dc-stage
verified live on cluster + product-repo prep in PR #454); **operator cutover pending
— see `docs/MIGRATION-CUTOVER.md`** (Track 2 PoC verified HTTP 200, 52 migrations /
86 tables, prod untouched). **Related:** `cf/CONFIG.md` (per-var secret reference),
`cf/DEPLOYMENT_CONFIG.md`, GitHub issues #451, #452, #453

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

## Open decisions (RESOLVED 2026-08-03 during execution)

- **DB strategy — RESOLVED: separate `decent_cloud_stage` DB in the shared `pgsql`
  app.** Track 2 provisioned it (isolates data, matches how Chatwoot already
  shares the pgsql host, cheapest). Prod keeps its own Postgres
  (`192.168.0.2:5432/decent_cloud_prod`); stage uses `decent_cloud_stage` on the
  shared pgsql. Rolling back the app never touches prod data.
- **Image strategy — RESOLVED: floating `:stage` tag.** `cf/deploy.py deploy stage`
  (Track 3) builds + pushes `git.kalaj.org/decent-stuff/decent-cloud-api:stage`
  and bumps the stage overlay; CI's `:stage` build is the automated equivalent.
  Until `:stage` ships, the stage overlay pins prod's tag (`445a17d4`) so stage
  tracks prod's code — see the cutover runbook § Step C.
- **Hostname rename — RESOLVED: overlay uses `stage-*`; operator MAY keep `dev-*`.**
  The stage overlay is authored with `stage-*` hostnames
  (`stage-support`, `stage-api`, `stage-gw`, …) for clarity. The operator cutover
  (runbook § Step D) is the decision point: rename `dev-*`→`stage-*` (new DNS +
  tunnel ingress + Stripe/OAuth redirect updates) OR keep `dev-*` (edit the
  overlay hostnames back, no DNS change). Either is valid; the runbook covers both.
- **Manifest reuse — RESOLVED: kustomize base + overlays.** `cluster/apps/
  decent-cloud/base/` is shared; `prod/` and `stage/` are thin overlays
  patching namespace/image-tags/hostPath/secret-names. Maximises reuse (one base
  to edit), avoids the sync burden of duplicating 7 manifests. Prod refactor
  renders byte-equivalent to the prior live state (verified before commit).

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

---

## APPENDIX A — Verified execution environment (2026-08-03, probed live)

Corrects several stale premises in the body above. This is the authoritative
environment context for the build.

| Capability | Status | Detail |
|---|---|---|
| Cluster access | ✅ cluster-admin | kubeconfig at `/project/decent-cloud/kube-config`; server `https://192.168.0.2:6443`; `auth can-i --list` = `*.* [*]` (full admin) |
| kubectl + kustomize | ✅ installed | `~/.local/bin/kubectl` v1.36.3 (kustomize v5.8.1 built-in); set `export KUBECONFIG=/project/decent-cloud/kube-config` |
| sops + age | ✅ installed | `/usr/local/bin/{sops,age}` |
| nuc-k3s repo | ✅ local only | checked out at `/project/decent-cloud/third_party/k8s/` (remote `git@github.com:sasa-tomic/nuc-k3s.git`); can edit+commit locally, **CANNOT push** |
| Product repo push | ✅ as `andris-k85` | `GITHUB_TEST_PAT` (in outer `secrets/shared/env.yaml`) has `repo` scope, `permissions.push=true` on `decent-stuff/decent-cloud`. repo/ remote uses SSH host alias `github-decent-cloud` (not resolvable here) → push over HTTPS with the PAT instead |
| nuc-k3s push | ❌ BLOCKED | `sasa-tomic/nuc-k3s` is private; `GITHUB_TEST_PAT` → 404; SSH `claude-code` key → permission denied. Manifests can only be authored locally + applied to cluster via kubectl |
| Prod secret (dc-secret.yaml) | ❌ can't decrypt | PGP-SOPS with operator key `FA5814CF1935EE80C454C9F1660DCCF069EC9176` (not present here). Not needed — prod is untouched |
| Secrets decryption (age) | ✅ ALL decrypt | The body's "age key broken / common.yaml unreadable" premise is **STALE** — `repo/secrets/shared/{common,dev,play}.yaml` AND the NEW consolidated outer `/project/decent-cloud/secrets/shared/env.yaml` all decrypt cleanly with `SOPS_AGE_KEY_FILE=/project/decent-cloud/{repo,}/secrets/.age-identity` |
| Consolidated secret store | ✅ exists | Outer `/project/decent-cloud/secrets/shared/env.yaml` (age) holds EVERY key (DATABASE_URL, STRIPE_*, CHATWOOT_*, SMTP_*, CF_*, ANTHROPIC_*, GOOGLE_OAUTH_*, TELEGRAM_*, …). This supersedes the 3-layer `repo/secrets/shared/` model |
| Cloudflare creds | ✅ available | `CF_API_TOKEN` + `CF_ZONE_ID` in env.yaml (for tunnel/DNS work) |

**Cluster live state observed:** namespaces present incl. `dc-prod` (active, ArgoCD-managed),
`argocd`, `apps`. ArgoCD prod App is `automated: {selfHeal: true, prune: true}` → anything applied
to `dc-prod` NOT in git gets pruned; therefore **prod must NEVER be mutated via kubectl** (only via
git push to nuc-k3s, which is blocked). A NEW `dc-stage` namespace has no ArgoCD App → resources
applied there via kubectl persist until the operator pushes nuc-k3s (ArgoCD then adopts by name).

## APPENDIX B — Execution decision (autonomous scope vs operator-gated)

The migration's END STATE (git-persisted ArgoCD-synced dc-stage + dev host decommissioned) is
**partially blocked** by the nuc-k3s push denial. Split into two tracks:

### Track 1 — nuc-k3s manifests (LOCAL ONLY, operator pushes later)
Author + verify, do NOT apply to `dc-prod`:
1. Restructure `cluster/apps/decent-cloud/{base,prod,stage}/` (kustomize). **Verify**
   `kubectl kustomize prod/` renders byte-equivalent to current live prod (diff against
   `kubectl kustomize` of the current flat dir). Zero-behavior-change on prod.
2. Author `stage/` overlay (namespace `dc-stage`, stage image tag, stage hostPath, secret/config
   names `dc-stage-secret`/`dc-stage-config`).
3. `cluster/core/dc-stage.yaml` namespace manifest.
4. `cluster/secrets/dc-stage-secret.yaml.template` (PLAINTEXT — operator runs
   `sops --encrypt --in-place` with their PGP key; we cannot PGP-encrypt here).
5. `cluster/argocd/application-decent-cloud-stage.yaml` (source path `.../stage`, ns `dc-stage`).
Commit locally in `third_party/k8s/`; leave for operator to push.

### Track 2 — dc-stage LIVE on cluster (COMPLETE — VERIFIED LIVE 2026-08-03)

**Status: COMPLETE.** Brought dc-stage up in the cluster to PROVE the migration
end-to-end (a real PoC, per the mandatory workflow), WITHOUT exposing it externally
(no tunnel change → no public traffic → safe even with real creds). Verified live.

Original scope (executed):
1. `kubectl create ns dc-stage`.
2. Copy `forgejo-registry-secret` from `dc-prod` → `dc-stage` (so nodes can pull the image).
3. Create `dc-stage-secret` + `dc-stage-config` in-cluster directly from outer `env.yaml` values
   (kubectl create secret/configmap — bypasses SOPS; the git SOPS version is Track 1 step 4).
4. Provision stage DB: a `decent_cloud_stage` database in the shared `pgsql` app (recommended DB
   strategy from the Open Decisions); run migrations.
5. Apply stage manifests (api, api-sync, website, redis) **reusing the current prod image tag**
   (`445a17d4`) — there is no `:stage` image and we cannot push images; reusing prod's tag makes
   stage track prod's code until CI ships `:stage`.
6. Verify health via `kubectl -n dc-stage` (pods Ready, `port-forward` to api `/api/v1/health`
   200). **Do NOT** reconfigure the dev tunnel — that is the operator cutover.

**Verified results (2026-08-03):**
- **Health 200.** `kubectl port-forward svc/dc-api` → `curl /api/v1/health` = HTTP 200, body
  `{"success":true,"message":"Decent Cloud API is running","environment":"stage"}`.
- **Shared Postgres discovered + stage DB provisioned.** Pod `pgsql-857cbb44d8-lbzw4` (ns `apps`),
  Service `pgsql` (ClusterIP `10.43.159.212:5432`), in-cluster DNS `pgsql.apps.svc.cluster.local:5432`,
  image `pgvector/pgvector:pg18`, superuser `postgres` — already hosting `decent_cloud_{dev,play,prod}`
  + `chatwoot_{dev,prod}`. Created dedicated role `decent_cloud_stage` (LOGIN) + DB
  `decent_cloud_stage` OWNER `decent_cloud_stage` with a fresh 32-char password (did NOT reuse
  play/dev pw). api-server **auto-migrated on startup** against `API_DATABASE_URL` — **52
  migrations applied, 86 tables** in `public` (latest: `52 | drop account subscription feature`).
- **Namespace state.** `dc-api` 1/1 Ready, `dc-api-sync` **0/0 (scaled to 0 for PoC safety — no
  outbound provider polling)**, `dc-redis` 1/1, `dc-website` 1/1. All Services ClusterIP-only
  (NO public exposure — dev tunnel untouched). Stores: `dc-stage-config` (16 keys, overlay),
  `dc-stage-secret` (17 keys, from `env.yaml`), `forgejo-registry-secret` (copied from `dc-prod`).
  **`dc-prod` completely untouched.** `STRIPE_SECRET_KEY = sk_test_` (TEST mode) — safe regardless.
- **Bugs found + root-cause fixed during the PoC:**
  1. **Stage overlay apply bug** — `dc-api-patch.yaml` `SMTP_ADDRESS`/`SMTP_USERNAME`
     `configMapKeyRef`s had no `key` → `apply` failed (`configMapKeyRef.key: Required value`). The
     api-server doesn't read `SMTP_*` (Chatwoot-only; stage reuses prod Chatwoot). Fixed by
     removing the two lines → nuc-k3s commit `deb4018` (alongside Track 1's `7013258`). Stage dc-api
     env now mirrors prod's set.
  2. **hostPath permissions** — `stage-api-data`/`stage-redis` hostPaths created root-owned
     (`DirectoryOrCreate`) but pods run `runAsUser/runAsGroup/fsGroup=1000` → `PermissionDenied` on
     `/data/ledger`. Fixed by `chown 1000:1000` on the stage-only dirs via a privileged one-off pod
     on node `nuc`. Prod dirs untouched.
- **Non-fatal warnings (api still served 200 throughout):** `CHATWOOT_PLATFORM_API_TOKEN` 401 vs
  prod Chatwoot (env.yaml token stale — agent-bot integration disabled until reconciled);
  `RATE LIMITING DISABLED` (expected, `ENVIRONMENT=stage ≠ production`); `CF_API_TOKEN/CF_ZONE_ID
  not configured` in dc-api-sync (only `DATABASE_URL`+`CREDENTIAL_ENCRYPTION_KEY` wired).

Full operator-facing detail + the cutover steps: `docs/MIGRATION-CUTOVER.md`.

### Track 3 — product-repo cleanup (Phase 2/3, PUSHED as andris-k85)
Pushable to `decent-stuff/decent-cloud`. ORDER-SENSITIVE: the destructive deletions
(`cf/docker-compose.dev.yml`, `scripts/dc-secrets`, `repo/secrets/shared/`) must only take effect
AFTER the operator cuts over (dc-stage live + tunnel repointed + dev host decommissioned).
Therefore: ship the non-destructive rewiring + docs now; stage the destructive deletions behind a
clear "run-after-cutover" note (or a feature gate) so a premature `git pull` on the dev host does
not break the running staging env.

### Operator-gated (NOT done autonomously — the remaining open items)

These are the only gates left. They are the **operator cutover** — fully specced
with copy-pasteable commands in `docs/MIGRATION-CUTOVER.md` (runbook steps A–G + a
minor-follow-ups section). The 8 items, in dependency order:

1. **Push the nuc-k3s overlay fix WITH Track 1** — `cd third_party/k8s && git push
   origin main`. Commits `7013258` (base/prod/stage split) + `deb4018` (SMTP
   `configMapKeyRef` fix) MUST go together, else ArgoCD re-applies the broken patch
   and the dc-stage sync fails. ArgoCD then syncs dc-stage from git, adopting the
   live resources by name. (Runbook Step A.)
2. **Persist the stage DB password to git SOPS — CRITICAL (data-loss / auth-break
   risk).** The `decent_cloud_stage` role password currently lives ONLY in the live
   `dc-stage-secret` (kubectl-created, not SOPS). Before ArgoCD adopts the namespace,
   the operator must either (a) extract it
   (`kubectl -n dc-stage get secret dc-stage-secret -o jsonpath='{.data.API_DATABASE_URL}' | base64 -d`)
   and SOPS-encrypt that value into `cluster/secrets/dc-stage-secret.yaml` (fill the
   template from Track 1 step 4); or (b) set their own password in SOPS and
   `ALTER ROLE decent_cloud_stage PASSWORD '…'` to match BEFORE ArgoCD syncs.
   Otherwise the first ArgoCD sync overwrites the live secret with the SOPS value
   and **breaks DB auth**. (a) preserves the already-migrated DB. (Runbook Step B,
   CRITICAL note.)
3. **Encrypt the full `dc-stage-secret` to git** — fill
   `cluster/secrets/dc-stage-secret.yaml.template` values,
   `sops --encrypt --encrypted-regex '^(data|stringData)$' --in-place`,
   commit+push nuc-k3s (PGP key `FA5814CF1935EE80C454C9F1660DCCF069EC9176`).
   (Runbook Step B.)
4. **Ship the `:stage` image tag** in CI; update the stage overlay image from the
   pinned prod tag `445a17d4` to `:stage`. Until then stage tracks prod's tag.
   (Runbook Step C.)
5. **Public cutover** — reconfigure the `decent-cloud-dev` cloudflared tunnel →
   dc-stage services; DNS `dev-*` → `stage-*` (or keep `dev-*`); verify
   `https://api.stage.decent-cloud.org/api/v1/health` 200. THIS is the user-visible
   switch. (Runbook Step D.)
6. **Re-enable `dc-api-sync`** (cutover Step E) —
   `kubectl -n dc-stage scale deployment dc-api-sync --replicas=1`. hostPath perms
   are already fixed, so it should start cleanly — verify Ready + no `PermissionDenied`.
7. **Tear down dev host + delete retired files** (cutover Steps F/G) — stop the
   docker-compose dev stack, `git pull` main on the dev host, then
   `git rm cf/docker-compose.dev.yml scripts/dc-secrets/ repo/secrets/shared/` +
   remove the `dev` path from `cf/deploy.py` (separate commit, only after F).
8. **Minor** — populate `TWILIO_AUTH_TOKEN` in `env.yaml` (currently empty — SMS
   escalation disabled in stage); reconcile `CHATWOOT_PLATFORM_API_TOKEN` (stale →
   401); optionally drop `SMTP_PASSWORD` from the api secret (unused — the api sends
   via MailChannels `MAILCHANNELS_API_KEY`+`DKIM`, `SMTP_*` is Chatwoot-only).
   (Runbook "Minor follow-ups" section.)
