# Migration cutover runbook — staging → k8s (`dc-stage`)

**Created:** 2026-08-03
**Status:** Tracks 1+2+3 done autonomously; **operator cutover pending** (the steps below).
**Plan:** `docs/plans/2026-08-03-staging-to-k8s-dc-stage-consolidation.md`
**Goal:** Move the shared staging env (today called "dev") off the local
docker-compose stack + the `repo/secrets/shared/` age-SOPS store onto the k8s
cluster as namespace `dc-stage` (ArgoCD-synced from the nuc-k3s repo), then retire
the old dev-deploy stack + the age secret store.

This is the **single source of truth** for the operator cutover. Steps are
copy-pasteable and ordered. **Read the whole runbook once before starting.**

---

## TL;DR (operator does these, in order)

1. **Verify** the autonomous pre-cutover state (Step 0) — confirm Tracks 1+2+3 landed.
2. **Push nuc-k3s** (Step A) — ArgoCD syncs `dc-stage` from git.
3. **Encrypt + persist** the stage secret to git (Step B).
4. **Ship `:stage`** image tag (Step C) — optional until CI builds it.
5. **Public cutover** (Step D) — repoint the tunnel + DNS. This is the user-visible switch.
6. **Enable api-sync** in stage (Step E).
7. **Tear down the old dev host** (Step F).
8. **Delete the retired files** (Step G) — **only after F is confirmed** — a separate commit.

> **Non-destructive until Step G.** Steps A–F are additive/cutover; the live
> `dev` docker-compose host keeps running and can serve traffic until Step F.
> Step G (the `git rm`s) MUST be a separate commit AFTER Step F, or a premature
> `git pull` on the dev host breaks the running staging env.

---

## Prerequisites (operator must have)

| Capability | Detail |
|---|---|
| nuc-k3s push access | `git@github.com:sasa-tomic/nuc-k3s.git` (checked out at `/project/decent-cloud/third_party/k8s`). The SOPS PGP key `FA5814CF1935EE80C454C9F1660DCCF069EC9176` to edit `cluster/secrets/*.yaml`. |
| Registry push access | Forgejo `git.kalaj.org` (owner `decent-stuff`). `docker login git.kalaj.org` first; the token lives in `~/.docker/config.json` (never paste into chat / a commit / a manifest). |
| Cloudflare tunnel + DNS access | Cloudflare dashboard / API token with Tunnel + DNS edit on the `decent-cloud.org` zone. The `decent-cloud-dev` tunnel is currently **remote-managed** (`config_src=cloudflare`). |
| kubectl + kustomize | `export KUBECONFIG=/project/decent-cloud/kubeconfig` (or wherever the cluster kubeconfig lives). cluster-admin on `https://192.168.0.2:6443`. |
| sops + the operator PGP key | `sops` on PATH; the private half of `FA5814CF…` available for decrypt/encrypt. |

---

## Pre-cutover (already done autonomously — verify, do not redo)

These were done by Tracks 1/2/3 in the autonomous session. **Step 0 verifies them.**
They need operator push/adopt before they take effect.

- **Track 1 — nuc-k3s manifests (committed LOCALLY, NOT pushed).**
  - `cluster/apps/decent-cloud/{base,prod,stage}/` restructured into a kustomize
    base + thin overlays. `prod/` renders byte-equivalent to the prior live prod
    (zero behavior change). `stage/` overlays namespace `dc-stage`, stage image
    tag, stage hostPath, `dc-stage-secret`/`dc-stage-config` names.
  - `cluster/core/dc-stage.yaml` (the namespace).
  - `cluster/secrets/dc-stage-secret.yaml.template` (PLAINTEXT template — Step B
    encrypts it).
  - `cluster/argocd/application-decent-cloud-stage.yaml` (ArgoCD App CR, source
    path `cluster/apps/decent-cloud/stage`, ns `dc-stage`), added to the
    `root` app-of-apps path.
- **Track 2 — `dc-stage` LIVE on the cluster (isolated, NOT publicly exposed).**
  - Namespace `dc-stage` created; `forgejo-registry-secret` copied from `dc-prod`.
  - `dc-stage-secret` + `dc-stage-config` created in-cluster directly (bypassing
    SOPS — Step B persists the secret to git).
  - Stage DB provisioned: a `decent_cloud_stage` database in the shared `pgsql`
    app (separate DB, isolated from prod's `decent_cloud_prod`); migrations run.
  - Stage manifests applied (api, website, redis) **reusing the prod image tag**
    (`445a17d4`) — there is no `:stage` image yet, so stage tracks prod's code
    until Step C ships `:stage`. api-sync is intentionally **skipped** (Step E).
  - Health verified via port-forward (`/api/v1/health` 200). The dev tunnel was
    NOT touched → stage is invisible to the public internet.
- **Track 3 — product-repo prep (PUSHED to `main` as `andris-k85`).**
  - `cf/deploy.py` gained `deploy stage` (build + push `:stage` image + bump the
    nuc-k3s overlay) and `config stage` (read dc-stage cluster stores). The
    legacy `dev` docker-compose path is intact (Step G retires it).
  - This runbook.

### Step 0 — Verify the pre-cutover state

Run these on a machine with cluster + repo access. Stop and fix before proceeding
if any check fails.

```bash
# Track 1: nuc-k3s has the stage manifests committed locally.
cd /project/decent-cloud/third_party/k8s
git log --oneline -5                                # expect stage/overlay commits
ls cluster/apps/decent-cloud/{base,prod,stage}/     # all three dirs present
ls cluster/core/dc-stage.yaml cluster/argocd/application-decent-cloud-stage.yaml
ls cluster/secrets/dc-stage-secret.yaml.template

# Track 1 (correctness): prod overlay renders byte-equivalent to the prior flat layout.
kubectl kustomize cluster/apps/decent-cloud/prod/ | head    # namespace dc-prod, current tags

# Track 2: dc-stage is live + healthy (no public exposure yet).
export KUBECONFIG=/project/decent-cloud/kubeconfig
kubectl get ns dc-stage
kubectl -n dc-stage get pods                         # api/website/redis Ready
kubectl -n dc-stage port-forward deploy/dc-api 59011:59011 &
curl -fsS http://localhost:59011/api/v1/health; echo   # expect {"status":"ok"} 200
kill %1

# Track 3: product repo has the stage deploy target.
cd /project/decent-cloud/repo
python3 cf/deploy.py deploy --help | grep stage       # expect `stage` in choices
```

---

## Step A — Push nuc-k3s (ArgoCD adopts the live dc-stage)

Track 1 left the manifests committed **locally** (the autonomous session could not
push the private nuc-k3s repo). Pushing makes ArgoCD sync `dc-stage` from git and
**adopt the live resources by name** (Track 2 created them via kubectl; once the
App exists, ArgoCD manages them).

```bash
cd /project/decent-cloud/third_party/k8s
git log --oneline origin/main..HEAD                  # review what's about to push
git push origin main
```

ArgoCD auto-syncs (the stage App is `automated: {selfHeal: true, prune: true}`).
Verify:

```bash
kubectl -n argocd get application decent-cloud-stage    # Synced + Healthy
kubectl -n argocd patch application decent-cloud-stage --type=merge \
  -p '{"metadata":{"annotations":{"argocd.argoproj.io/refresh":"normal"}}}'  # force refresh if needed
kubectl -n dc-stage get pods                            # still Ready (adopted, not recreated)
```

> **Prod safety:** `dc-prod` is also ArgoCD-managed with `prune: true`. Track 1's
> prod overlay MUST render byte-equivalent to the prior flat layout — that was
> verified autonomously (`kubectl kustomize prod/` diff). Re-confirm before push
> if anything looks off: `kubectl kustomize cluster/apps/decent-cloud/prod/ | kubectl diff -f -`.

---

## Step B — Encrypt + persist the stage secret to git

Track 2 created `dc-stage-secret` in-cluster directly (bypassing SOPS, since the
autonomous session lacks the operator PGP key). Persist it to git so ArgoCD owns
it going forward.

```bash
cd /project/decent-cloud/third_party/k8s

# 1. Fill the real values in the plaintext template (Track 1 authored it).
#    Values migrate from repo/secrets/shared/dev.yaml (+ readable common.yaml) —
#    or the consolidated outer /project/decent-cloud/secrets/shared/env.yaml.
$EDITOR cluster/secrets/dc-stage-secret.yaml.template
mv cluster/secrets/dc-stage-secret.yaml.template cluster/secrets/dc-stage-secret.yaml

# 2. Encrypt with the operator PGP key (same encrypted_regex as prod dc-secret.yaml).
sops --encrypt --encrypted-regex '^(data|stringData)$' --in-place cluster/secrets/dc-stage-secret.yaml

# 3. Verify it decrypts + matches the live in-cluster secret, then commit + push.
sops -d cluster/secrets/dc-stage-secret.yaml | head
git add cluster/secrets/dc-stage-secret.yaml
git commit -m "secrets(stage): persist dc-stage-secret (PGP-SOPS)"
git push origin main

# 4. Reconcile the live secret from git (manage-secrets.py is filename-agnostic).
python3 scripts/manage-secrets.py
kubectl -n dc-stage rollout restart deploy/dc-api
```

---

## Step C — Ship the `:stage` image tag (optional until CI builds it)

Until this step, the stage overlay pins prod's image tag (`445a17d4`), so stage
runs prod's code. Once a `:stage` floating tag exists (built by CI on every merge
to `main`, or shipped manually below), point the overlay at it.

**Manual ship** (use `cf/deploy.py deploy stage` — added by Track 3):

```bash
cd /project/decent-cloud/repo
docker login git.kalaj.org                                   # operator token
python3 cf/deploy.py deploy stage                            # default tag = floating :stage
# → builds api image, pushes git.kalaj.org/decent-stuff/decent-cloud-api:stage,
#   bumps cluster/apps/decent-cloud/stage/kustomization.yaml, commits nuc-k3s LOCALLY.
```

The command prints the exact `git push` to run (it commits nuc-k3s locally and
cannot push from the autonomous session). Push it and let ArgoCD sync:

```bash
cd /project/decent-cloud/third_party/k8s
git push origin main
kubectl -n argocd patch application decent-cloud-stage --type=merge \
  -p '{"metadata":{"annotations":{"argocd.argoproj.io/refresh":"normal"}}}'
kubectl -n dc-stage rollout restart deploy/dc-api
```

> To pin a specific build instead of the floating tag:
> `python3 cf/deploy.py deploy stage --tag <short-sha>`. SHA-tagging is the repo's
> established image convention (see `repo/AGENTS.md` § PACKAGE REGISTRY).

---

## Step D — Public cutover (the user-visible switch)

This is the only step the public sees. Two parts: repoint the tunnel at dc-stage,
then DNS.

> **Decision (locked by operator 2026-08-03): rename `dev-*` → `stage-*`.** Use
> the `stage-*` hostnames the stage overlay already defines — `stage.decent-cloud.org`,
> `stage-api.decent-cloud.org`, `stage-support.decent-cloud.org`, `stage-gw.decent-cloud.org`,
> … (new CF DNS records + tunnel ingress entries). The "keep `dev-*`" alternative
> is **not** chosen; any remaining `dev-*` refs in code/docs are adjusted to `stage-*`.

**Hostname choice (DECIDED — `stage-*`):**
- ✅ **Rename `dev-*` → `stage-*`** (cleanest; the overlay already uses `stage-*`):
  new CF DNS records + tunnel ingress entries — `stage-support.decent-cloud.org`,
  `stage-api.decent-cloud.org`, `stage-gw.decent-cloud.org`, …
- ⬜ ~~**Keep `dev-*`** (no DNS change)~~ — **rejected** by the operator 2026-08-03.

### D.1 — Repoint the tunnel

The `decent-cloud-dev` tunnel is currently **remote-managed**. Convert it to
local-managed (a `dc-stage-cloudflared-config` ConfigMap, same pattern as prod)
OR keep it remote-managed and re-point its ingress at the
`*.dc-stage.svc.cluster.local` FQDNs. Recommended: convert to local-managed so
both tunnels are git-managed (this also retires the last use of `cf/tunnel.py`).

```bash
# Option A (recommended): rename the tunnel, convert to local-managed.
# Rename via CF API:
python3 cf/tunnel.py dev-rename decent-cloud-stage   # if that helper exists;
# otherwise use the CF dashboard/API: PATCH the tunnel name decent-cloud-dev → decent-cloud-stage

# Edit the stage overlay's cloudflared config (ConfigMap dc-cloudflared-config in
# the stage overlay) to route:
#   stage-support.decent-cloud.org → dc-chatwoot-web.dc-stage.svc.cluster.local:80
#   stage-api.decent-cloud.org     → dc-api.dc-stage.svc.cluster.local:59001
#   *.stage-gw.decent-cloud.org    → (gateway routing, same as prod's gw)
$EDITOR /project/decent-cloud/third_party/k8s/cluster/apps/decent-cloud/stage/kustomization.yaml
cd /project/decent-cloud/third_party/k8s && git commit -am "tunnel(stage): route decent-cloud-stage at dc-stage" && git push origin main
```

### D.2 — DNS

Create the `stage-*` CNAME records → the tunnel's
`<tunnel-id>.cfargotunnel.com`. Verify propagation:

```bash
dig +short stage-api.decent-cloud.org        # resolves to the tunnel CNAME
curl -fsS https://stage-api.decent-cloud.org/api/v1/health && echo  # 200
curl -fsS https://stage.decent-cloud.org/ >/dev/null && echo "web OK"
```

> **Stripe / OAuth redirect URIs (rename is locked):** add the new `stage-*`
> callback URLs in the Stripe dashboard + Google Cloud Console
> (`https://stage-api.decent-cloud.org/api/v1/oauth/google/callback`) and update
> `GOOGLE_OAUTH_REDIRECT_URL` / `FRONTEND_URL` in `dc-stage-config`.

---

## Step E — Enable api-sync in stage

Track 2 skipped the stage `dc-api-sync` Deployment for PoC safety (no background
provider sync running during the isolated bring-up). Enable it now so the
background provider/gateway sync runs in stage.

```bash
# The stage overlay should include a dc-api-sync manifest (mirrors prod's dc-api.yaml
# second Deployment). If Track 1 gated it behind a comment/flag, uncomment/enable it.
cd /project/decent-cloud/third_party/k8s
# verify the overlay renders the api-sync Deployment:
kubectl kustomize cluster/apps/decent-cloud/stage/ | grep -A2 'name: dc-api-sync'
git commit -am "stage: enable dc-api-sync" || echo "already enabled"
git push origin main
kubectl -n dc-stage get pods -l app=dc-api-sync     # becomes Ready
```

---

## Step F — Tear down the old dev host

**Only after Step D is confirmed** (dc-stage serves public traffic, health 200,
OAuth/Stripe/webhook flows exercised). The dev host still runs the OLD
docker-compose stack; stop it, then it is decommissioned.

```bash
# On the dev host (ssh in):
cd <product-repo-on-dev-host>
python3 cf/deploy.py stop dev                      # docker compose down for the dev stack
docker compose -p decent-cloud-dev -f cf/docker-compose.dev.yml down -v   # confirm fully down
# The Cloudflare tunnel connector (cloudflared container) stops here too — the
# tunnel is now served by the stage overlay's cloudflared pod (Step D.1).

git pull origin main                               # pull Track 3 + later commits
```

At this point staging is fully on `dc-stage` (k8s). The old dev-deploy stack is
stopped but its files still exist in the repo — Step G removes them.

---

## Step G — Delete the retired files (ONLY after F confirmed working)

**Separate commit, AFTER Step F.** Deleting these before the dev host is
decommissioned breaks its next `git pull` + redeploy. They are now dead code:
staging is `dc-stage` on k8s; local dev is the slim `scripts/dev-server.sh` stack
(which needs no SOPS — see `docs/plans/2026-08-03-staging-to-k8s-dc-stage-consolidation.md`
§ "Local dev needs no secrets").

```bash
cd /project/decent-cloud/repo

# 1. The retired dev-deploy stack + age secret store + its tooling.
git rm cf/docker-compose.dev.yml
git rm -r scripts/dc-secrets scripts/test_dc_secrets.py scripts/test-dc-secrets.sh
git rm -r secrets/shared                              # common.yaml dev.yaml play.yaml + .sops.yaml + .locks/

# 2. Remove the legacy `dev` path from cf/deploy.py (compose lifecycle: deploy/stop/
#    logs/status/restart for the full stack) — keep ONLY `config` (+ slim-compose
#    helpers if any remain; the slim stack is driven by scripts/dev-server.sh).
#    Also drop the now-dead get_env_config/load_secrets_from_sops/compose helpers
#    and the dev-only stop/logs/status/restart subcommands. `config dev` either
#    becomes a thin alias or is removed (local dev has no SOPS to audit).
$EDITOR cf/deploy.py

# 3. Sweep the ~25 referencing files: cf/{CONFIG,DEPLOYMENT_CONFIG}.md,
#    cf/.env*.example, api/.env.example, AGENTS.md, e2e setup docs, dev-server.sh's
#    optional `dc-secrets export play` call (lines ~113-119) — repoint or remove.
rg -l 'dc-secrets|secrets/shared|docker-compose\.dev'   # find what remains

python3 -m py_compile cf/deploy.py                      # syntax check before commit
git add -A
git commit -m "chore: remove retired dev-deploy stack (post k8s cutover)"
git push origin main
```

> The slim local dev stack (`scripts/dev-server.sh` + the outer
> `agent/docker-compose.yml` postgres sidecar) STAYS — it's the AI-agent inner
> loop and needs no SOPS.

---

## Rollback

How to revert each step if something goes wrong (in reverse order):

- **After Step G (files deleted):** `git revert <step-G-commit>` and `git pull` on
  the dev host, then `python3 cf/deploy.py deploy dev` to revive the old stack.
  Requires the age key (`repo/secrets/.age-identity`) to still be present.
- **After Step D (public cutover):** point the tunnel back at the dev-host
  compose targets (`website:59000`, `api-serve:59001`, `chatwoot-web:59002`) and
  DNS `stage-*` → `dev-*` (or restore the `dev-*` records). The old dev host must
  still be running (don't run Step F until Step D is stable for a watch period).
- **ArgoCD:** `kubectl -n argocd app history decent-cloud-stage` →
  `kubectl -n argocd app rollback decent-cloud-stage <revision-id>`. Or revert the
  nuc-k3s commit and push.
- **Stage DB:** `decent_cloud_stage` is a separate DB — rolling back the app does
  NOT touch prod's `decent_cloud_prod`. Drop the stage DB only if you want a
  clean slate: `kubectl -n pgsql exec -it <pgsql-pod> -- psql -c 'DROP DATABASE decent_cloud_stage;'`.

> **Prod is never touched by this migration.** `dc-prod`, its manifests, and its
> secret are unchanged throughout. The only prod-adjacent risk is Track 1's prod
> overlay refactor — verified byte-equivalent and re-confirmable with
> `kubectl kustomize .../prod/ | kubectl diff -f -` before the Step A push.

---

## Reference

- **Plan:** `docs/plans/2026-08-03-staging-to-k8s-dc-stage-consolidation.md`
  (Appendix A = verified environment; Appendix B = the 3-track split).
- **Config map:** `cf/CONFIG.md` (per-var source for each env) + `cf/deploy.py config <env>`.
- **Image/registry policy:** `repo/AGENTS.md` § PACKAGE REGISTRY (SHA-tag api images;
  hotfix website tags suffixed `-hotfix.<sha>`).
- **Open decisions (resolved here):**
  - **DB strategy** → separate `decent_cloud_stage` DB in the shared `pgsql` app.
  - **Image strategy** → floating `:stage` tag (shipped by `cf/deploy.py deploy stage`
    or CI); until then stage pins prod's tag.
  - **Hostname rename** → **DECIDED (operator 2026-08-03): rename `dev-*` → `stage-*`**;
    use the `stage-*` hostnames the stage overlay already defines (Step D).
