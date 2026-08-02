# Migration cutover runbook — staging → k8s (`dc-stage`)

**Created:** 2026-08-03
**Status:** Tracks 1+2+3 done autonomously; **Track 2 PoC VERIFIED LIVE** (dc-stage
serves HTTP 200, DB migrated, prod untouched); **operator cutover pending** (the steps below).
**Plan:** `docs/plans/2026-08-03-staging-to-k8s-dc-stage-consolidation.md`
**Goal:** Move the shared staging env (today called "dev") off the local
docker-compose stack + the `repo/secrets/shared/` age-SOPS store onto the k8s
cluster as namespace `dc-stage` (ArgoCD-synced from the nuc-k3s repo), then retire
the old dev-deploy stack + the age secret store.

This is the **single source of truth** for the operator cutover. Steps are
copy-pasteable and ordered. **Read the whole runbook once before starting.**

---

## Pre-cutover status (VERIFIED 2026-08-03)

**Headline: `dc-stage` is LIVE and healthy.** This is no longer a forward-looking
claim — the PoC was brought up on the cluster and verified end-to-end. Step 0 below
is now a quick re-confirmation, not a green-field check.

- **Health:** `kubectl -n dc-stage port-forward svc/dc-api 59011:59011` →
  `curl http://localhost:59011/api/v1/health` returns **HTTP 200**, body
  `{"success":true,"message":"Decent Cloud API is running","environment":"stage"}`.
- **Stage DB provisioned + migrated.** Dedicated role `decent_cloud_stage` (LOGIN)
  + DB `decent_cloud_stage` OWNER `decent_cloud_stage` with a fresh 32-char password
  (matches prod's isolation pattern; did NOT reuse play/dev pw). The api-server
  **auto-migrates on startup** against `API_DATABASE_URL` — **52 migrations applied,
  86 tables** in `public` (latest: `52 | drop account subscription feature`). Stage
  `API_DATABASE_URL` → `pgsql.apps.svc.cluster.local:5432/decent_cloud_stage`.
- **Shared Postgres discovered:** pod `pgsql-857cbb44d8-lbzw4` in namespace `apps`;
  Service `pgsql` (ClusterIP `10.43.159.212:5432`); in-cluster DNS
  `pgsql.apps.svc.cluster.local:5432`; image `pgvector/pgvector:pg18`; superuser
  `postgres`. It already hosts `decent_cloud_{dev,play,prod}` +
  `chatwoot_{dev,prod}` (one role per DB).
- **dc-stage namespace state:** `dc-api` 1/1 Ready, `dc-api-sync` **0/0 (scaled to 0
  for PoC safety — no outbound provider polling)**, `dc-redis` 1/1, `dc-website` 1/1.
  All Services are ClusterIP-only (**NO public exposure — the dev tunnel was NOT
  touched**). Stores: `dc-stage-config` (16 keys, from overlay), `dc-stage-secret`
  (17 keys, from `env.yaml`), `forgejo-registry-secret` (copied from `dc-prod`).
  **`dc-prod` is completely untouched.**
- **Stripe is in TEST mode:** `STRIPE_SECRET_KEY = sk_test_…` — safe regardless (api-sync
  at 0, no inbound public traffic).
- **Bugs found + root-cause fixed during the PoC:**
  1. **Stage overlay apply bug** — `dc-api-patch.yaml` added `SMTP_ADDRESS`/`SMTP_USERNAME`
     `configMapKeyRef`s without a `key` → `apply` failed
     (`configMapKeyRef.key: Required value`). Root cause: api-server doesn't read
     `SMTP_*` (those are Chatwoot-only; stage reuses prod Chatwoot). Fixed by removing
     the two lines (nuc-k3s commit `deb4018`, alongside Track 1). Stage dc-api env now
     mirrors prod's set.
  2. **hostPath permissions** — `stage-api-data`/`stage-redis` hostPaths were created
     root-owned (`DirectoryOrCreate`) but pods run
     `runAsUser/runAsGroup/fsGroup=1000` → `PermissionDenied` on `/data/ledger`. Fixed
     by `chown 1000:1000` on the stage-only dirs via a privileged one-off pod on node
     `nuc`. Prod dirs (`api-data`, `redis`) untouched.
- **Non-fatal warnings (expected for an isolated PoC; api still serves 200):**
  - `CHATWOOT_PLATFORM_API_TOKEN` (from `env.yaml`) returns **401** against prod
    Chatwoot (`Invalid access_token`) — the env.yaml token is stale vs prod's actual.
    Chatwoot agent-bot integration disabled until reconciled.
  - `RATE LIMITING DISABLED` — expected (`ENVIRONMENT=stage ≠ production`).
  - `CF_API_TOKEN/CF_ZONE_ID not configured` in `dc-api-sync` — the sync Deployment
    only wires `DATABASE_URL`+`CREDENTIAL_ENCRYPTION_KEY`; gateway-DNS sync would need
    `CF_*` wired if ever enabled.

---

## TL;DR (operator does these, in order)

1. **Verify** the pre-cutover state (Step 0) — confirm Tracks 1+2+3 landed
   (Track 2 PoC is **VERIFIED LIVE** — see Pre-cutover status above).
2. **Push nuc-k3s** (Step A) — ArgoCD syncs `dc-stage` from git. **Push BOTH
   commits `7013258` + `deb4018` together.**
3. **Encrypt + persist** the stage secret to git (Step B) — ⚠️ **reconcile the
   stage DB password FIRST or the first ArgoCD sync breaks DB auth** (see the
   CRITICAL note in Step B).
4. **Ship `:stage`** image tag (Step C) — optional until CI builds it.
5. **Public cutover** (Step D) — repoint the tunnel + DNS. This is the user-visible switch.
6. **Enable api-sync** in stage (Step E) — hostPath perms already fixed.
7. **Tear down the old dev host** (Step F).
8. **Delete the retired files** (Step G) — **only after F is confirmed** — a separate commit.
9. **Minor follow-ups** (TWILIO, stale CHATWOOT token, optional SMTP_PASSWORD drop) — see below.

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
- **Track 2 — `dc-stage` LIVE on the cluster (VERIFIED 2026-08-03, isolated, NOT publicly exposed).**
  See **Pre-cutover status (VERIFIED 2026-08-03)** above for the full verified detail.
  - Namespace `dc-stage` created; `forgejo-registry-secret` copied from `dc-prod`.
  - `dc-stage-secret` + `dc-stage-config` created in-cluster directly (bypassing
    SOPS — Step B persists the secret to git).
  - Stage DB provisioned: dedicated role `decent_cloud_stage` + DB
    `decent_cloud_stage` in the shared `pgsql` app (isolated from prod's
    `decent_cloud_prod`). **52 migrations auto-applied on api startup, 86 tables.**
  - Stage manifests applied (api, website, redis) **reusing the prod image tag**
    (`445a17d4`) — there is no `:stage` image yet, so stage tracks prod's code
    until Step C ships `:stage`. `dc-api-sync` is **scaled to 0** (Step E re-enables it).
  - Health **VERIFIED** via port-forward → `/api/v1/health` **HTTP 200**, body
    `{"success":true,"message":"Decent Cloud API is running","environment":"stage"}`.
    The dev tunnel was NOT touched → stage is invisible to the public internet.
- **Track 3 — product-repo prep (PUSHED to `main` as `andris-k85`).**
  - `cf/deploy.py` gained `deploy stage` (build + push `:stage` image + bump the
    nuc-k3s overlay) and `config stage` (read dc-stage cluster stores). The
    legacy `dev` docker-compose path is intact (Step G retires it).
  - This runbook.

### Step 0 — Verify the pre-cutover state

The PoC was **already brought up and verified** (see Pre-cutover status above). This
step is a quick re-confirmation that the cluster state is still as left. Run these on
a machine with cluster + repo access; stop and fix before proceeding if any check fails.

```bash
# Track 1: nuc-k3s has the stage manifests committed locally.
cd /project/decent-cloud/third_party/k8s
git log --oneline -5                                # expect stage/overlay commits
ls cluster/apps/decent-cloud/{base,prod,stage}/     # all three dirs present
ls cluster/core/dc-stage.yaml cluster/argocd/application-decent-cloud-stage.yaml
ls cluster/secrets/dc-stage-secret.yaml.template

# Track 1 (correctness): prod overlay renders byte-equivalent to the prior flat layout.
kubectl kustomize cluster/apps/decent-cloud/prod/ | head    # namespace dc-prod, current tags

# Track 2: dc-stage is live + healthy (re-confirm the VERIFIED PoC).
export KUBECONFIG=/project/decent-cloud/kubeconfig
kubectl get ns dc-stage
kubectl -n dc-stage get pods                         # api/website/redis Ready, api-sync 0/0
kubectl -n dc-stage port-forward svc/dc-api 59011:59011 &
curl -fsS http://localhost:59011/api/v1/health; echo   # expect HTTP 200 {"success":true,"message":"Decent Cloud API is running","environment":"stage"}
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

> ⚠️ **Push BOTH commits together** — `7013258` (base/prod/stage split) **and**
> `deb4018` (SMTP `configMapKeyRef` fix). The SMTP fix removed `SMTP_ADDRESS`/
> `SMTP_USERNAME` refs that had no `key:` and broke `apply`
> (`configMapKeyRef.key: Required value`) — the api-server doesn't read `SMTP_*`
> (Chatwoot-only; stage reuses prod Chatwoot). Pushing `7013258` alone lets ArgoCD
> re-apply the broken patch and fail the dc-stage sync. Confirm both are present:
> `git log --oneline origin/main..HEAD` must show `deb4018` on top of `7013258`.

```bash
cd /project/decent-cloud/third_party/k8s
git log --oneline origin/main..HEAD                  # review: expect BOTH 7013258 + deb4018
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

> 🚨 **CRITICAL — stage DB password reconciliation (do this FIRST, before the
> Step A push lands / ArgoCD's first sync).** The `decent_cloud_stage` role password
> currently lives **ONLY in the live in-cluster `dc-stage-secret`** (kubectl-created,
> NOT in git). The moment ArgoCD adopts the namespace (Step A) with a git-managed
> `dc-stage-secret`, the **first sync overwrites the live Secret with the SOPS value
> and breaks DB auth** — the api pod then crash-loops on `API_DATABASE_URL`. You MUST
> reconcile first, one of:
> - **(a) Extract + commit the live value (preserve the already-migrated DB):**
>   ```bash
>   kubectl -n dc-stage get secret dc-stage-secret -o jsonpath='{.data.API_DATABASE_URL}' \
>     | base64 -d   # → postgres://decent_cloud_stage:<pw>@pgsql.apps.svc.cluster.local:5432/decent_cloud_stage
>   ```
>   then put that connection string's password into the SOPS template below (Step B.1).
> - **(b) Set your OWN password in SOPS and make the live DB match BEFORE ArgoCD syncs:**
>   ```bash
>   # while dc-stage-secret is still the live (kubectl) one, before Step A:
>   kubectl -n apps exec -it pgsql-857cbb44d8-lbzw4 -- \
>     psql -U postgres -c "ALTER ROLE decent_cloud_stage PASSWORD '<your-sops-password>';"
>   ```
> Do NOT run Step A until one of (a)/(b) is done. (a) is safer — it keeps the
> already-migrated DB (52 migrations, 86 tables) intact.

```bash
cd /project/decent-cloud/third_party/k8s

# 1. Fill the real values in the plaintext template (Track 1 authored it).
#    Values migrate from the consolidated outer secrets/shared/env.yaml (read with
#    sops -d + .age-identity) — and the reconciled stage DB password from above.
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

> **SOPS PGP key:** `FA5814CF1935EE80C454C9F1660DCCF069EC9176` (same as prod's
> `dc-secret.yaml`). If you renamed the template away in step 1, the file MUST be
> named `dc-stage-secret.yaml` for `manage-secrets.py` (filename-agnostic `rglob`)
> to apply it.

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
then DNS. **Decide the hostname strategy first** (Open Decision, resolved:
overlay uses `stage-*`; operator MAY keep `dev-*` to avoid DNS churn — adjust the
tunnel ingress + any `dev-*` refs in code/docs accordingly).

**Hostname choice (pick one):**
- **Rename `dev-*` → `stage-*`** (cleanest; the overlay already uses `stage-*`):
  new CF DNS records + tunnel ingress entries — `stage-support.decent-cloud.org`,
  `stage-api.decent-cloud.org`, `stage-gw.decent-cloud.org`, …
- **Keep `dev-*`** (no DNS change): edit the stage overlay's hostnames/tunnel
  ingress back to `dev-*`. Less churn, less clear.

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

Create the `stage-*` (or keep `dev-*`) CNAME records → the tunnel's
`<tunnel-id>.cfargotunnel.com`. Verify propagation:

```bash
dig +short stage-api.decent-cloud.org        # resolves to the tunnel CNAME
curl -fsS https://stage-api.decent-cloud.org/api/v1/health && echo  # 200
curl -fsS https://stage.decent-cloud.org/ >/dev/null && echo "web OK"
```

> **Stripe / OAuth redirect URIs:** if you renamed to `stage-*`, add the new
> callback URLs in the Stripe dashboard + Google Cloud Console
> (`https://stage-api.decent-cloud.org/api/v1/oauth/google/callback`) and update
> `GOOGLE_OAUTH_REDIRECT_URL` / `FRONTEND_URL` in `dc-stage-config`. If you kept
> `dev-*`, no change.

---

## Step E — Enable api-sync in stage

Track 2 scaled the stage `dc-api-sync` Deployment to **0** for PoC safety (no
background provider sync / outbound polling running during the isolated bring-up).
Enable it now so the background provider/gateway sync runs in stage.

> **hostPath permissions are ALREADY fixed.** During the PoC the `stage-api-data`/
> `stage-redis` hostPaths were `chown 1000:1000`'d (pods run
> `runAsUser/runAsGroup/fsGroup=1000`) via a privileged one-off pod on node `nuc`.
> Prod dirs were untouched. So api-sync should start cleanly once scaled up —
> still verify it reaches Ready and logs no `PermissionDenied` on `/data/ledger`.

```bash
kubectl -n dc-stage scale deployment dc-api-sync --replicas=1
kubectl -n dc-stage rollout status deployment dc-api-sync   # reaches Ready
kubectl -n dc-stage logs deploy/dc-api-sync --tail=50 | grep -iE 'error|permission' || echo "no errors"
```

> If the overlay gated the api-sync manifest behind a comment/flag, instead enable
> it in git first:
> ```bash
> cd /project/decent-cloud/third_party/k8s
> kubectl kustomize cluster/apps/decent-cloud/stage/ | grep -A2 'name: dc-api-sync'  # confirm it renders
> git commit -am "stage: enable dc-api-sync" || echo "already enabled"
> git push origin main
> kubectl -n dc-stage get pods -l app=dc-api-sync     # becomes Ready
> ```
> Note: the dc-api-sync Deployment only wires `DATABASE_URL` +
> `CREDENTIAL_ENCRYPTION_KEY`; gateway-DNS sync additionally needs `CF_API_TOKEN`/
> `CF_ZONE_ID` (currently NOT configured — wire them in the overlay if you enable
> that path).

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

## Minor follow-ups (non-blocking, surfaced during the PoC)

These did NOT block the PoC (api served 200 throughout) but should be cleaned up
around the cutover:

- **`TWILIO_AUTH_TOKEN` is empty** in `env.yaml` → SMS escalation is disabled in
  stage. Populate it in `env.yaml` (and re-SOPS into `dc-stage-secret`) if stage
  should exercise SMS escalation.
- **`CHATWOOT_PLATFORM_API_TOKEN` is stale (401).** The token in `env.yaml`
  returns `Invalid access_token` against prod Chatwoot — it has drifted from
  prod's actual token. Reconcile (re-fetch the live token from prod Chatwoot and
  update `env.yaml` / `dc-stage-secret`) before relying on the Chatwoot agent-bot
  integration in stage. (Track 2 left that integration disabled as a result.)
- **`SMTP_PASSWORD` is unused in the api secret.** The api-server sends mail via
  MailChannels (`MAILCHANNELS_API_KEY` + `DKIM_*`), NOT via SMTP — `SMTP_*` is
  Chatwoot-only (and stage reuses prod Chatwoot). Optionally drop `SMTP_PASSWORD`
  from the api `dc-stage-secret` to avoid implying the api uses it.

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
  - **Hostname rename** → overlay uses `stage-*`; operator MAY keep `dev-*` (Step D).
