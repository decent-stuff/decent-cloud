# Decent Agents: GitHub repo webhook integration spec

- **Issue:** [#414](https://github.com/decent-stuff/decent-cloud/issues/414)
- **Status:** Plan only. No code in this commit. Implementation deferred.
- **Date:** 2026-04-25
- **Owner:** founder (solo)
- **Companion spec:** [`docs/specs/2026-04-25-decent-agents-identity-provisioning-spec.md`](2026-04-25-decent-agents-identity-provisioning-spec.md)
  (#413, may not yet be committed). #413 owns `agent_identities`, `agent_repos`,
  and `agent_subscriptions`. This spec owns everything between GitHub and that table set.
- **Adjacent prior art:** Stripe webhook implementation at
  [`api/src/openapi/webhooks.rs:130-231`](../../api/src/openapi/webhooks.rs)
  (signature verification + replay handling) and the dispute DB primitives at
  [`api/src/database/contracts/dispute.rs:56-105`](../../api/src/database/contracts/dispute.rs)
  (idempotent UPSERT pattern). Both are the templates this spec extends.

---

## TL;DR

Use a **GitHub App**. Confidence **9/10**. Reasoning lives in section A.

The webhook handler reuses the Stripe pattern at
[`api/src/openapi/webhooks.rs:130-231`](../../api/src/openapi/webhooks.rs):
HMAC verification of the raw body, idempotent dedupe on a delivery-id UNIQUE
column, fail-fast on parse errors, return 2xx after persisting so GitHub stops
retrying. New tables: `github_app_installations` and `github_webhook_deliveries`.
Repo-to-identity resolution is implicit-by-subscription for v1: one
subscription = one `agent_identities.id` = the agent for every repo in that
customer's installation. Outgoing calls use short-lived installation access
tokens generated server-side and passed to the agent container; the App private
key never leaves the API server.

---

## A. Decision: GitHub App vs PAT

### Option 1: GitHub App (RECOMMENDED)

**Customer onboarding (one-click):**

1. Customer clicks **"Connect GitHub"** in the Decent Agents dashboard.
2. We redirect to `https://github.com/apps/decent-agents/installations/new` with
   `state=<csrf_nonce>` and `redirect_uri=https://decent-cloud.org/dashboard/agents/connected`.
3. Customer picks repos to install on (all repos OR a subset).
4. GitHub redirects back with `installation_id` and `code`. We exchange `code`
   for a user-to-server token (one-time, used to confirm the installer's
   GitHub login matches the dashboard user). We store `installation_id` against
   the user.
5. From then on, GitHub sends webhook events for those repos to
   `https://api.decent-cloud.org/api/v1/webhooks/github`.

**Permissions requested at App registration time:**

| Scope | Access | Why |
|-------|--------|-----|
| Issues | Read & write | Read titles/bodies/comments; reply, label, close |
| Pull requests | Read & write | Read PR diffs; open PRs, comment, mark draft, request review |
| Contents | Read & write | Clone the repo, push commits to a branch |
| Checks | Read & write | Surface agent run status as a check on the PR |
| Metadata | Read | Mandatory; exposes repo names / default branch |

**Permissions explicitly NOT requested (least-privilege):**

- Actions, Deployments, Packages, Pages, Secrets, Workflows: agent must NOT
  modify CI/CD or read deployment secrets.
- Administration, Members: agent must NOT manage repo collaborators.
- Email addresses, Followers, Profile: zero need for personal data.

**Webhook events to subscribe:**

- `installation` (created, deleted, suspend, unsuspend, new_permissions_accepted)
- `installation_repositories` (added, removed)
- `issues` (opened, edited, closed, reopened, assigned, labeled)
- `issue_comment` (created, edited, deleted)
- `pull_request` (opened, edited, closed, reopened, synchronize, review_requested)
- `pull_request_review` (submitted, dismissed)
- `pull_request_review_comment` (created)
- `ping` (sanity check on App registration)

**Auth model:**

- App private key (PEM) lives in env var `GITHUB_APP_PRIVATE_KEY` (or in
  `secrets/shared/env.yaml` under SOPS, exported by the `dc-secrets` flow).
- Server creates a 10-minute JWT signed `RS256` with `iss=<app_id>` to call
  `POST /app/installations/:installation_id/access_tokens` and gets back a
  15-minute installation access token scoped to that one installation.
- Tokens are cached in-memory keyed by `installation_id` until 60 seconds before
  expiry, then refreshed.

**Customer DX:** one click, GitHub-managed token rotation, App permissions are
listed in the customer's GitHub settings and are revocable from there. Auditable.

**Operational cost (one-time):**

- ~1 hour: register the App in GitHub, generate private key, upload icon, write
  `https://github.com/apps/decent-agents` description page.
- Marketplace listing is **optional**. The App can stay private/unlisted
  indefinitely; we can flip it public later without re-registering.

**Cost to customer:** $0 (GitHub Apps are free regardless of repo count).

**Risks:**

- App per-org installation rate limit: GitHub allows generous numbers per org;
  not a real risk for our scale. Conf 9/10.
- The App's identity is "decent-agents[bot]". PRs and comments will be authored
  by that bot, NOT by the customer. UX implication: the customer cannot make
  the bot file an issue *as them* -- the bot files as itself. We document this
  upfront. Co-authored-by trailers in commits soften it.

### Option 2: PAT + manual webhook

**Customer onboarding (5-step):**

1. Customer creates a fine-grained PAT (`github.com/settings/tokens?type=beta`).
2. Customer picks the right scopes (Contents read+write, Issues read+write,
   PRs read+write, Metadata read). Easy to misconfigure.
3. Customer copies the PAT into our dashboard.
4. Our backend calls `POST /repos/:owner/:repo/hooks` per repo to install a
   webhook pointing at our endpoint with a generated secret per customer.
5. Customer verifies a delivery succeeded.

**Auth model:**

- We hold the customer's PAT (encrypted at rest with `CREDENTIAL_ENCRYPTION_KEY`).
- Outgoing API calls use the PAT directly.
- PAT expires per the customer's setting (max 1 year for fine-grained PATs).
  We cron a check 7 days before expiry and email the customer to rotate.
- Per-repo webhook needs to be re-installed if the customer revokes our PAT.

**DX:** Bad. PAT-creation flow is well-known to be error-prone; ~30% of users
mis-scope on first attempt (anecdotal, but known industry pain).

**Operational cost (per customer, ongoing):**

- PAT expiry chasing.
- Scope drift detection (customer downgrades a scope; webhook keeps working
  but PR comments stop).
- Webhook re-setup if the customer (or a teammate) deletes the webhook in
  repo settings.

**Cost to customer:** $0.

**Customer trust posture:** we hold a long-lived secret with broad repo access.
Revocation is in the customer's hands but is awkward (delete PAT in GitHub
settings, then come back to our dashboard to confirm).

### Recommendation

**GitHub App.** Confidence **9/10**.

Reasoning, ranked:

1. **DX:** one click vs five steps. Makes the difference between "I tried it on
   a Friday afternoon" and "I'll come back to this on Monday".
2. **Security:** GitHub-rotated 15-minute tokens scoped per-installation
   strictly beat a 1-year PAT we have to encrypt and watch.
3. **Ops:** zero PAT-expiry chasing. Solo founder, so this matters
   disproportionately.
4. **Audit:** customers see the App's permissions in their own GitHub settings;
   nothing hidden in our dashboard.
5. **Net upfront cost:** ~1 hour to register the App, ~1 day to implement JWT
   minting and token caching. PAT path saves nothing -- it just defers the cost
   into per-customer support.

The 1/10 hedge is for the bot-as-author UX caveat (section H, risk 3) and the
"first-time GitHub App work in this codebase" learning curve. Neither blocks.

---

## B. Architecture

```
GitHub repo (issue, PR, comment, review)
            |
            v
GitHub App webhook delivery
            |  POST + X-Hub-Signature-256 + X-GitHub-Delivery + X-GitHub-Event
            v
+-------------------------------------------------------------+
|  api.decent-cloud.org/api/v1/webhooks/github                |
|  (Poem handler, rate-limit-skipped via rate_limit.rs:175)   |
+-------------------------------------------------------------+
            |
            | 1. Verify HMAC-SHA256 of raw body using GITHUB_APP_WEBHOOK_SECRET
            | 2. INSERT INTO github_webhook_deliveries           (UNIQUE on
            |    github_delivery_id; duplicate -> 200 OK, no work)
            | 3. Parse event JSON by X-GitHub-Event header
            v
+-------------------------------------------------------------+
|  Event dispatcher (match on event_type)                     |
+-------------------------------------------------------------+
            |
            +--> installation* events --> mutate github_app_installations
            |                              and agent_repos rows
            |
            +--> issues / issue_comment / pull_request / pull_request_review:
            |        a. Resolve installation_id -> agent_identities.id
            |           (via agent_repos -> agent_subscriptions -> identity)
            |        b. Apply trigger semantics (section G).
            |           If not triggered: log + return 200.
            |        c. INSERT INTO agent_runs (status='queued',
            |           identity_id, repo_full_name, event_ref, ...)
            |        d. Mint installation access token (15 min TTL)
            |        e. enqueue dispatch job: docker exec -e
            |           AGENT_NAME=<slug> -e GITHUB_TOKEN=<inst_token>
            |           dc-agent-<slug> agent-runner ...
            v
+-------------------------------------------------------------+
|  Identity container (shell-mode, per agent/docker-compose.yml) |
|  reuses existing per-identity HOME + agent runner            |
+-------------------------------------------------------------+
            |
            v
Agent does work, writes back via the GitHub API using the installation token
(comments, PRs, check runs). Updates agent_runs (status='succeeded'/'failed',
token_usage, exit_code).
```

Three things to notice:

1. **The webhook endpoint is the only ingress.** GitHub never talks to the
   container.
2. **The container never holds the App private key.** It receives a 15-minute
   installation access token; if it needs a new one mid-run it asks the API
   server back via an internal IPC (e.g. localhost socket inside the container,
   or a `dc-agent api-cli` call to a new endpoint `/agents/:run_id/refresh-token`).
3. **The dispatch path between API and container is the existing
   `docker exec` shell-mode pattern**
   ([`agent/docker-compose.yml:11-67`](../../../agent/docker-compose.yml)
   in the outer workspace). We extend it: the per-identity container is started
   long-running, and the API queues exec invocations. No new container runtime.

---

## C. Data model

### Tables owned by #413 (referenced, not redefined here)

- `agent_identities` -- one row per agent persona; columns `id`,
  `slug` (e.g. `andris-k85`), `display_name`, `github_username` (the bot
  account paired with the identity), `home_dir`, etc.
- `agent_subscriptions` -- one row per paying customer; columns `id`, `user_id`,
  `agent_identity_id`, `status`, `current_period_end_ns`, ...
- `agent_repos` -- link table; columns `id`, `subscription_id`,
  `github_installation_id`, `github_repo_id`, `github_repo_full_name`,
  `enabled`, `created_at_ns`, `removed_at_ns` (nullable, soft-delete).

This spec assumes #413 lands first. If #413 is reordered after #414, this spec's
DDL block should be merged into the same migration to keep the schema atomic.

### NEW tables owned by #414

#### `github_app_installations`

```sql
CREATE TABLE github_app_installations (
    id BIGSERIAL PRIMARY KEY,
    github_installation_id BIGINT NOT NULL UNIQUE,
    github_account_login TEXT NOT NULL,
    github_account_type TEXT NOT NULL,
        -- 'User' | 'Organization' (free-text, no CHECK; the GitHub-side enum
        -- is the source of truth and we forward the raw value)
    user_id BYTEA REFERENCES accounts(pubkey) ON DELETE SET NULL,
        -- Decent Cloud user who installed the App; may be NULL if the
        -- installation predates the OAuth-confirm step (defensive).
    suspended_at_ns BIGINT,
        -- non-NULL while GitHub has the installation suspended; we MUST stop
        -- dispatching until it's NULL again.
    removed_at_ns BIGINT,
        -- non-NULL after installation.deleted webhook; soft-delete, never DROP.
    created_at_ns BIGINT NOT NULL,
    updated_at_ns BIGINT NOT NULL
);

CREATE INDEX idx_github_app_installations_user
    ON github_app_installations(user_id)
    WHERE user_id IS NOT NULL AND removed_at_ns IS NULL;
```

Pattern note: `BIGINT NOT NULL UNIQUE` on `github_installation_id` mirrors
`stripe_dispute_id TEXT NOT NULL UNIQUE` at
[`api/migrations_pg/043_dispute_pause_state.sql:17`](../../api/migrations_pg/043_dispute_pause_state.sql).
Same idempotency story.

#### `github_webhook_deliveries`

```sql
CREATE TABLE github_webhook_deliveries (
    id BIGSERIAL PRIMARY KEY,
    github_delivery_id TEXT NOT NULL UNIQUE,
        -- X-GitHub-Delivery header (UUID). Replay protection.
    event_type TEXT NOT NULL,
        -- X-GitHub-Event header (e.g. 'issues', 'pull_request').
    action TEXT,
        -- payload['action'] when present (e.g. 'opened', 'created').
    github_installation_id BIGINT,
        -- payload['installation']['id'] when present; NULL for ping events
        -- and (rare) installation-less events.
    raw_payload JSONB NOT NULL,
        -- Full body for audit, replay, and debugging. Not joined on.
    signature_verified BOOLEAN NOT NULL,
        -- true when X-Hub-Signature-256 matched. We still persist failures
        -- (with verified=false) to detect attempted forgeries, but never
        -- dispatch them.
    dispatched_to_identity_id BYTEA REFERENCES agent_identities(id),
        -- NULL when no dispatch happened (control event or trigger not met).
    dispatch_status TEXT NOT NULL DEFAULT 'pending',
        -- 'pending' | 'dispatched' | 'skipped' | 'failed'
    error_message TEXT,
        -- non-NULL when dispatch_status='failed'
    processed_at_ns BIGINT NOT NULL,
        -- when our handler finished (used for ops dashboards)
    created_at_ns BIGINT NOT NULL
);

CREATE INDEX idx_github_webhook_deliveries_processed_at
    ON github_webhook_deliveries(processed_at_ns DESC);
CREATE INDEX idx_github_webhook_deliveries_installation
    ON github_webhook_deliveries(github_installation_id)
    WHERE github_installation_id IS NOT NULL;
```

#### `agent_runs` (new -- minimal; full schema in #413 if it claims this table; otherwise here)

```sql
CREATE TABLE agent_runs (
    id BIGSERIAL PRIMARY KEY,
    agent_identity_id BYTEA NOT NULL REFERENCES agent_identities(id),
    github_delivery_id TEXT NOT NULL REFERENCES github_webhook_deliveries(github_delivery_id),
    github_repo_full_name TEXT NOT NULL,
    github_event_kind TEXT NOT NULL,
        -- 'issue' | 'pr' | 'review' | 'comment'
    github_event_ref TEXT NOT NULL,
        -- e.g. 'issue#123' or 'pr#45' or 'review#789'
    status TEXT NOT NULL,
        -- 'queued' | 'running' | 'succeeded' | 'failed' | 'cancelled'
    started_at_ns BIGINT,
    finished_at_ns BIGINT,
    exit_code INT,
    cost_usd_e9s BIGINT,
        -- nanodollars (matches decent-cloud's e9s currency convention)
    error_message TEXT,
    created_at_ns BIGINT NOT NULL
);

CREATE INDEX idx_agent_runs_identity ON agent_runs(agent_identity_id);
CREATE INDEX idx_agent_runs_status ON agent_runs(status)
    WHERE status IN ('queued', 'running');
```

If #413 already defines `agent_runs`, drop this block and add only the
`github_delivery_id` FK column there.

### Migration file

`api/migrations_pg/0NN_decent_agents_github_integration.sql` where `0NN` is the
next available number at implementation time. Top of `main` at spec authoring
is `043_dispute_pause_state.sql`; `044_refund_audit.sql` is in flight on
another branch. Pick whichever number is free when this lands; the migration
runner is order-by-filename, so number conflicts are surfaced loudly.

---

## D. Webhook handler

New module: `api/src/openapi/webhooks/github.rs` (note: `webhooks.rs` is
2675 lines; the existing webhook code is one flat file. We split the new code
into a sibling file under a `webhooks/` directory and re-export from
`api/src/openapi/mod.rs`. This is the right time to start the split because
GitHub adds ~600 lines and the existing file is past the comfort threshold).

Restructure:

- `api/src/openapi/webhooks.rs` -> `api/src/openapi/webhooks/mod.rs` (re-exports)
  + `api/src/openapi/webhooks/stripe.rs` (existing code)
  + `api/src/openapi/webhooks/chatwoot.rs` (existing code)
  + `api/src/openapi/webhooks/icpay.rs` (existing code)
  + `api/src/openapi/webhooks/telegram.rs` (existing code)
  + `api/src/openapi/webhooks/github.rs` (NEW)
  + `api/src/openapi/webhooks/util.rs` (NEW -- shared HMAC utility)

Pull `verify_signature` (Stripe) at `webhooks.rs:130-172` and the equivalent
ICPay verifier at `webhooks.rs:1315-1367` into `webhooks/util.rs::verify_hmac_sha256_hex`,
parameterised on the header layout (Stripe uses `t=...,v1=...`; GitHub uses
plain `sha256=...`). DRY -- one HMAC implementation, three callers. The split
itself can be a separate small PR landed before the GitHub spec implementation
to keep diffs reviewable.

### Route registration

In `api/src/main.rs` (currently registers webhooks at `main.rs:1289-1304`):

```rust
.at(
    "/api/v1/webhooks/github",
    post(openapi::webhooks::github::github_webhook),
)
```

Rate-limit skip already handled by the prefix match at
[`api/src/rate_limit.rs:175`](../../api/src/rate_limit.rs).

### Handler structure (pseudocode)

```text
pub async fn github_webhook(db, body, req) -> Result<Response, PoemError>:
    1. Read raw body as bytes (we MUST verify signature on bytes, not on a
       re-serialised JSON value).
    2. delivery_id = req.header("X-GitHub-Delivery")  # required, UUID
       event_type = req.header("X-GitHub-Event")      # required
       signature  = req.header("X-Hub-Signature-256") # required, "sha256=..."
       Missing any -> 400.

    3. Compute HMAC-SHA256(body, GITHUB_APP_WEBHOOK_SECRET) and constant-time
       compare against signature. Use `subtle::ConstantTimeEq` (already a
       dep transitively via cryptography stack).
       Mismatch -> insert delivery row with signature_verified=false,
       return 401.

       Failure mode caveat: a NON-2xx tells GitHub to retry. Returning 401 on
       a forged delivery is fine because GitHub didn't sign it; the legitimate
       App will only ever produce 200 here. We log forgery attempts at WARN.

    4. INSERT INTO github_webhook_deliveries (delivery_id, event_type, action,
       installation_id, raw_payload, signature_verified=true,
       dispatch_status='pending', processed_at_ns=now, created_at_ns=now)
       ON CONFLICT (github_delivery_id) DO NOTHING RETURNING id.

       If RETURNING is empty (UNIQUE collision), this is a replay -> return
       200 OK with body '{"replay":true}'. Stripe-pattern dedupe.

    5. Parse body as serde_json::Value (cheap; we already deserialised for
       signature anyway).

    6. match event_type:
         "ping"                       -> return 200 (App registration probe)
         "installation"               -> handle_installation(db, payload)
         "installation_repositories"  -> handle_installation_repos(db, payload)
         "issues"                     -> handle_issues(db, payload, delivery_id)
         "issue_comment"              -> handle_issue_comment(db, payload,
                                                              delivery_id)
         "pull_request"               -> handle_pull_request(db, payload,
                                                             delivery_id)
         "pull_request_review"        -> handle_pull_request_review(...)
         "pull_request_review_comment"-> handle_pr_review_comment(...)
         _                            -> log INFO + return 200 (unknown event;
                                          we keep the row, dispatch=skipped)

    7. Each handler returns DispatchOutcome { status, identity_id?, error? }.
       UPDATE github_webhook_deliveries SET dispatch_status=$1,
       dispatched_to_identity_id=$2, error_message=$3 WHERE id=$row_id.

    8. Return 200 always (post signature check + dedupe).
```

### Handler-by-handler behaviour

**`installation.created`:**

```text
let inst = payload["installation"];
INSERT INTO github_app_installations (
    github_installation_id = inst["id"],
    github_account_login   = inst["account"]["login"],
    github_account_type    = inst["account"]["type"],
    user_id                = NULL,  -- linked later via dashboard OAuth flow
    created_at_ns = now, updated_at_ns = now
)
ON CONFLICT (github_installation_id) DO UPDATE SET
    github_account_login = EXCLUDED.github_account_login,
    github_account_type  = EXCLUDED.github_account_type,
    suspended_at_ns      = NULL,
    removed_at_ns        = NULL,    -- reinstall after delete
    updated_at_ns        = EXCLUDED.updated_at_ns;

For each repo in payload["repositories"]:
    INSERT INTO agent_repos (subscription_id=NULL, github_installation_id,
                             github_repo_id, github_repo_full_name)
    -- subscription_id stays NULL until the dashboard OAuth step links the
    -- installation to a user and finds their active subscription. Webhook
    -- events for unlinked installations are persisted but not dispatched
    -- (DispatchOutcome::skipped, error="installation not linked to user").
```

**`installation.deleted`:**

```text
UPDATE github_app_installations SET removed_at_ns=now, updated_at_ns=now
    WHERE github_installation_id=payload["installation"]["id"];
UPDATE agent_repos SET removed_at_ns=now
    WHERE github_installation_id=payload["installation"]["id"]
      AND removed_at_ns IS NULL;
```

**`installation.suspend` / `installation.unsuspend`:**

```text
UPDATE github_app_installations SET
    suspended_at_ns = (CASE WHEN action='suspend' THEN now ELSE NULL END),
    updated_at_ns = now
WHERE github_installation_id=payload["installation"]["id"];
```

While `suspended_at_ns IS NOT NULL`, ALL other event types skip dispatch.

**`installation_repositories.added`:**

```text
For each repo in payload["repositories_added"]:
    INSERT INTO agent_repos (subscription_id, github_installation_id,
                             github_repo_id, github_repo_full_name)
    -- subscription_id resolved by joining github_app_installations.user_id
    -- to agent_subscriptions.user_id.
```

**`installation_repositories.removed`:**

```text
UPDATE agent_repos SET removed_at_ns=now
WHERE github_installation_id=payload["installation"]["id"]
  AND github_repo_id = ANY(payload["repositories_removed"][*]["id"]);
```

**`issues.opened`, `issues.edited`, `issue_comment.created`,
`pull_request.opened`, `pull_request.synchronize`,
`pull_request_review.submitted`, `pull_request_review_comment.created`:**

```text
1. Resolve installation_id -> agent_identities.id (section E).
   If unresolved: DispatchOutcome::skipped(reason="no active subscription").
2. Apply trigger filter (section G).
   If not triggered: DispatchOutcome::skipped(reason="trigger not matched").
3. Mint installation access token (section F).
4. INSERT INTO agent_runs (status='queued', identity_id, repo_full_name,
   event_ref=...).
5. Spawn the dispatch (described in section F).
6. DispatchOutcome::dispatched(identity_id).
```

The spawn itself is ASYNC: the webhook handler returns 200 the moment the row
is in `agent_runs` and the docker exec is invoked. Long agent runs do NOT block
GitHub's 10-second delivery timeout.

---

## E. Repo-to-identity resolution

### v1 rule: implicit-by-subscription

Each customer has at most one active subscription, which carries one
`agent_identity_id`. All repos in their installation map to that one identity.

### Algorithm

```text
fn resolve_identity(installation_id: i64, db: &Database)
    -> Result<Option<AgentIdentityId>>:

    let inst = SELECT user_id, suspended_at_ns, removed_at_ns
               FROM github_app_installations
               WHERE github_installation_id = $1;
    if inst is None:
        // installation event raced ahead of installation.created;
        // skip, log, do NOT fail.
        return Ok(None);
    if inst.removed_at_ns IS NOT NULL: return Ok(None);
    if inst.suspended_at_ns IS NOT NULL: return Ok(None);
    if inst.user_id IS NULL:
        // Installation not yet linked to a Decent Cloud user. Ignore until
        // the user finishes the dashboard OAuth-confirm step.
        return Ok(None);

    let sub = SELECT id, agent_identity_id, status
              FROM agent_subscriptions
              WHERE user_id = $1
                AND status IN ('active', 'trialing')
              ORDER BY created_at_ns DESC
              LIMIT 1;
    if sub is None: return Ok(None);

    Ok(Some(sub.agent_identity_id))
```

### Why implicit, not explicit-per-repo

- v1 product rule: 1 subscription -> 1 identity. Per-repo selection is
  meaningless.
- Solo founder, ship-fastest principle: the dashboard does NOT need a "pick
  identities per repo" UI in v1.
- Implicit resolution is one query. Three joins max.

### Multi-identity future (deferred)

When the product offers multiple identities per subscription (post-launch,
filed under label `deferred-post-launch`), switch to **explicit-per-repo**:
add `agent_identity_id` column directly on `agent_repos`, write a small
dashboard UI to assign each repo to one identity. The above algorithm becomes:

```text
SELECT agent_identity_id FROM agent_repos
WHERE github_installation_id=$1 AND github_repo_id=$2 AND removed_at_ns IS NULL;
```

That's the only line that changes. Document this clearly so future-us knows
the shape.

---

## F. Authentication for outgoing API calls

### Goal

Agent containers must NEVER hold the App private key. They get short-lived
installation access tokens scoped to one installation, valid 15 minutes.

### Flow

```text
[API server, request time]
1. Load GITHUB_APP_PRIVATE_KEY (PEM) at startup. Refuse to start if missing or
   malformed. (Pattern from CLAUDE.md "DEPLOY-TIME VALIDATION".)
2. mint_jwt(app_id, private_key) -> JWT (RS256, 10-min TTL, iss=app_id).
3. POST https://api.github.com/app/installations/:installation_id/access_tokens
   with `Authorization: Bearer <JWT>` -> { token, expires_at }.
4. Cache (installation_id -> (token, expires_at)) in tokio::sync::RwLock<HashMap>.
   Refresh when expires_at - now < 60s.

[Dispatch to container]
5. docker exec -e AGENT_NAME=<slug>
                -e GITHUB_TOKEN=<installation_token>
                -e GITHUB_TOKEN_EXPIRES_AT=<unix_seconds>
                -e DC_AGENT_RUN_ID=<run_id>
                <container_name> /home/agent/agent-runner
   The container reuses the existing per-identity HOME (
   tools/homes/dc-<slug>/) and the gh CLI auth model: `gh` reads
   GITHUB_TOKEN from env, no `gh auth login` needed.

[Mid-run refresh]
6. If a run lasts past expiry, agent calls back into the API:
   POST /api/v1/agent-runs/:run_id/refresh-token
   Headers: Authorization: Bearer <run_secret>  -- a per-run nonce we
                                                    generated at queue-time
                                                    and passed via env.
   Response: { token, expires_at }
   The handler uses the cached token (step 4) and only calls GitHub if needed.
```

### Why split key (server) from token (container)

1. **Blast radius.** A compromised container leaks one customer's 15-minute
   token, scoped to one installation. A compromised App private key would
   leak access to *all* customers' repos.
2. **Audit story.** GitHub logs every access-token mint by JWT claim. The
   API server's audit trail (`github_webhook_deliveries.dispatched_to_identity_id`,
   `agent_runs.id`) cross-references each mint to a delivery and a run.
3. **Operational sanity.** Rotating the App private key is one env-var change
   on one box; rotating it across N customer containers is N opportunities to
   miss one.

### Storing the private key

`GITHUB_APP_PRIVATE_KEY` (PEM, multi-line). Stored under SOPS at
`secrets/shared/env.yaml`. Validated at startup by `serve_command` and
`api-server doctor` (per CLAUDE.md DEPLOY-TIME VALIDATION rule). Doctor check:
attempt to mint a JWT, then call `GET /app` with it; assert the App ID matches
`GITHUB_APP_ID`.

Required env vars (added to `api/.env.example` and `cf/.env.example`):

- `GITHUB_APP_ID` -- numeric ID from GitHub App settings.
- `GITHUB_APP_PRIVATE_KEY` -- full PEM, line-broken with `\n` literal.
- `GITHUB_APP_WEBHOOK_SECRET` -- strong random string (>=32 bytes), set at App
  registration.
- `GITHUB_APP_CLIENT_ID` / `GITHUB_APP_CLIENT_SECRET` -- for the OAuth
  user-to-server flow that links installation to user.

---

## G. Trigger semantics

### Default trigger phrase

`@decent-agent`

Configurable per-installation in v1 via a new column
`github_app_installations.trigger_phrase TEXT NOT NULL DEFAULT '@decent-agent'`.
The dashboard surfaces it as a single text input on the connected-repos page.
v1 ships with the default; configurability is a 5-minute UI element.

### Rules

| Event | Trigger? |
|-------|----------|
| `issues.opened` | Trigger if title or body contains the trigger phrase OR has label `decent-agent`. |
| `issues.edited` | Trigger if the edit ADDED the phrase or label (compare changes from `payload.changes`). |
| `issues.labeled` | Trigger if the added label is `decent-agent`. |
| `issue_comment.created` | Trigger if comment body contains the trigger phrase. |
| `pull_request.opened` | Trigger if PR description contains the trigger phrase. |
| `pull_request.synchronize` | Trigger if the PR was previously dispatched (the agent is iterating on its own PR) OR explicitly mentioned. |
| `pull_request_review.submitted` (`changes_requested`) | Trigger when the review is on a PR previously authored by the agent's bot account. |
| `pull_request_review.submitted` (`approved` or `commented`) | Log only. |
| `pull_request_review_comment.created` | Trigger if comment body contains trigger phrase. |
| Anything else | Log only. |

### "Previously dispatched" tracking

`agent_runs.github_event_ref` already records the issue/PR identity. Lookup:

```sql
SELECT 1 FROM agent_runs
WHERE github_repo_full_name=$1
  AND github_event_ref=$2
  AND status IN ('succeeded','running','queued')
LIMIT 1;
```

If hit, the event is part of an existing thread the agent owns and we trigger
without requiring an explicit re-mention.

### Boundary: never cross subscription boundaries

The resolver in section E is keyed on `installation_id`. Two customers with two
installations can never receive each other's events because the resolution
strictly follows the chain
`installation_id -> user_id -> active subscription -> identity_id`. This is
the meaningful security invariant; callable out in tests.

---

## H. Risk and uncertainty

| # | Claim | Confidence | Mitigation |
|---|-------|-----------:|------------|
| 1 | GitHub App marketplace approval not required for private/unlisted use | **9/10** | Verified against GitHub docs as of 2026-04. App is functional from minute one without listing. |
| 2 | Installation-token quota generous enough for our scale | **8/10** | GitHub publishes 5000 req/h per installation. Each agent run does <50 calls. We can support 100 concurrent runs per installation per hour without throttling. Add a `tracing::warn!` at >80% utilisation per the LOUD-misconfig rule. |
| 3 | Bot-as-author UX is acceptable for customers | **8/10** | Comments and PRs will say "decent-agents[bot]". Mitigation: every commit the agent makes includes `Co-authored-by: <customer-github-name> <email>` trailer, and the PR body opens with "Created on behalf of @customer". This is industry-standard (Renovate, Dependabot). |
| 4 | "Contents: read+write" includes git push | **7/10** | Documented in GitHub's permissions page. Verify by smoke test before launch. If wrong, we need to add "Workflows" too -- unlikely. |
| 5 | Webhook delivery to public endpoint works for installations on private repos | **9/10** | GitHub does not require the endpoint to have access to the repo; it just signs and posts. We've implemented this pattern for Stripe; same shape. |
| 6 | Process group / docker exec teardown works on long agent runs | **7/10** | Reuse existing `os.setsid` + `os.killpg` from dispatcher (per MEMORY.md). Add an integration test that kills a 30-min agent at 5 min via the API. |
| 7 | No race between `installation.created` and the customer's dashboard OAuth-confirm | **8/10** | Webhook events for unlinked installations are persisted but skipped. The dashboard step links retroactively (`UPDATE github_app_installations SET user_id=...`). Worst case: customer's first event gets dropped -- log it loudly and surface in dashboard so they retry. |
| 8 | One App private key in env-var (vs HSM/KMS) is acceptable for v1 | **7/10** | Private key is PEM in SOPS, encrypted at rest, decrypted only on the API host. KMS deferred until customer count or compliance demands it. Rotation procedure: register a new key in App settings (GitHub allows two simultaneously), update env, retire old. Document in runbook before launch. |

---

## I. Acceptance criteria

Tick-box list. Each item maps to a file:line where the change lands.

- [ ] **App registered with required permissions** -- runbook entry in
      `docs/operations/decent-agents-runbook.md` (NEW; section "Initial GitHub
      App registration").
- [ ] **Migration `0NN_decent_agents_github_integration.sql`** creates
      `github_app_installations` and `github_webhook_deliveries` (and
      `agent_runs` if not in #413). File: `api/migrations_pg/0NN_*.sql` --
      pick the next free number at implementation time.
- [ ] **Module split** -- `api/src/openapi/webhooks.rs` becomes
      `api/src/openapi/webhooks/mod.rs` plus per-provider files. The Stripe and
      ICPay HMAC verifiers collapse to a single
      `api/src/openapi/webhooks/util.rs::verify_hmac_sha256_hex`.
- [ ] **`POST /api/v1/webhooks/github`** registered at
      `api/src/main.rs:1289` (next to other webhooks). Rate-limit skip
      already covered by prefix check at `api/src/rate_limit.rs:175`.
- [ ] **Signature verification** -- constant-time HMAC-SHA256 against raw body
      with `GITHUB_APP_WEBHOOK_SECRET`. Forgeries logged at WARN, persisted with
      `signature_verified=false`, return 401.
- [ ] **Dedupe on `github_delivery_id`** -- UNIQUE collision returns 200 OK
      without re-dispatching. Tested with a positive replay test.
- [ ] **Event handlers** for: `ping`, `installation.{created,deleted,suspend,
      unsuspend,new_permissions_accepted}`, `installation_repositories.{added,
      removed}`, `issues.{opened,edited,labeled}`, `issue_comment.created`,
      `pull_request.{opened,synchronize}`, `pull_request_review.submitted`,
      `pull_request_review_comment.created`. Unknown event types log INFO and
      return 200.
- [ ] **Repo-to-identity resolver** -- section E algorithm implemented as
      `Database::resolve_identity_for_installation(installation_id) -> Option<id>`.
      Unit-tested for: happy path, suspended, removed, unlinked user, no active
      subscription.
- [ ] **JWT minting** -- `api/src/github_app/auth.rs` (new module) with
      `mint_app_jwt(app_id, private_key)` and
      `get_installation_token(installation_id) -> CachedToken` using
      `tokio::sync::RwLock<HashMap<i64, CachedToken>>`.
- [ ] **Dispatch to container** -- `api/src/agent_dispatch.rs` (new module)
      runs `docker exec` against the per-identity container with `AGENT_NAME`,
      `GITHUB_TOKEN`, `GITHUB_TOKEN_EXPIRES_AT`, `DC_AGENT_RUN_ID`,
      `DC_AGENT_RUN_SECRET` env. Integration-tested with a stub container.
- [ ] **Mid-run token refresh endpoint** --
      `POST /api/v1/agent-runs/:run_id/refresh-token` with run-secret bearer auth.
- [ ] **Trigger filter** --
      `api/src/openapi/webhooks/github/trigger.rs::is_triggered(event, payload, trigger_phrase) -> bool`.
      Unit-tested per row of section G's table.
- [ ] **Doctor check** -- `api/src/main.rs::doctor_command` mints a JWT and
      calls `GET /app`; on failure refuses to mark green.
- [ ] **Startup validation** -- `serve_command` rejects boot when any of
      `GITHUB_APP_ID`, `GITHUB_APP_PRIVATE_KEY`, `GITHUB_APP_WEBHOOK_SECRET`
      is missing/malformed. Loud warn if `GITHUB_APP_CLIENT_ID` is missing
      (OAuth-link flow disabled but webhooks still work -- degraded mode is
      explicit, not silent).
- [ ] **Env templates** -- `api/.env.example` and `cf/.env.example` get the
      five new env vars with comments.
- [ ] **`scripts/dc-secrets`** -- runbook entry showing `dc-secrets set
      shared/env GITHUB_APP_*=...`.
- [ ] **Unit tests**:
      - `webhooks::github::tests::ping_returns_200`
      - `webhooks::github::tests::forged_signature_returns_401_persists_row`
      - `webhooks::github::tests::replay_returns_200_without_redispatch`
      - `webhooks::github::tests::installation_created_inserts_row`
      - `webhooks::github::tests::installation_deleted_soft_deletes_repos`
      - `webhooks::github::tests::issue_with_trigger_dispatches`
      - `webhooks::github::tests::issue_without_trigger_skips`
      - `webhooks::github::tests::cross_subscription_isolation`
        (assert installation A's events never resolve to subscription B's identity)
      - `github_app::auth::tests::jwt_round_trip`
      - `github_app::auth::tests::token_cache_refreshes_before_expiry`
- [ ] **E2E test** -- Playwright suite under `website/tests/e2e/decent-agents/`:
      open a fixture issue against a test repo (using a sandbox install of the
      App), verify the webhook fires, the row lands in
      `github_webhook_deliveries`, and the agent posts a reply. Tagged
      `@decent-agents-e2e` and gated on `DC_E2E_GITHUB_APP=1`.
- [ ] **Dashboard surface** -- `website/src/routes/dashboard/agents/connected/+page.svelte`
      (NEW): list connected installations, show last 20 deliveries, show
      trigger phrase config. Read-only in v1 for everything except the
      trigger phrase.
- [ ] **Documentation** -- `docs/decent-agents/customer-onboarding.md` (NEW)
      walks through the GitHub App install flow with screenshots.

---

## J. Effort estimate

Honest. Solo founder. Includes test-writing, doctor-check, dashboard scaffold,
runbook. Excludes Marketplace listing (deferred).

| Section | Effort |
|---------|-------:|
| App registration + key handling + env validation (section F infrastructure) | 0.5d |
| Module split of `webhooks.rs` (prereq, separate small PR) | 0.5d |
| Migration + DB primitives (section C) | 0.5d |
| Webhook handler dispatch core + signature + dedupe (section D items 1-5) | 0.5d |
| All event-type handlers (section D items 6, plus section G triggers) | 1d |
| JWT minting + token cache (section F server-side) | 0.5d |
| Container dispatch glue + run-secret + refresh endpoint (section F container-side) | 1d |
| Resolver + cross-subscription isolation tests (section E) | 0.5d |
| E2E test against sandbox repo | 1d |
| Dashboard scaffold (read-only list, trigger config) | 0.5d |
| Customer onboarding docs + runbook | 0.5d |
| Buffer (always-needed) | 1d |
| **Total** | **~7.5 days** |

This assumes #413 lands first (or at least the table shape is settled). If
#413 slips, add 1-2 days of merge-conflict and joint-migration work.

---

## K. Out of scope (filed as separate tickets after this lands)

- GitHub Marketplace listing (paid or free tier). File under
  `decent-agents,deferred-post-launch`.
- Multi-identity per subscription. File under `decent-agents,deferred-post-launch`.
- Bitbucket / GitLab support. Different webhook shape, different auth model;
  separate spec.
- Agent self-mentioning loop protection (the bot must not trigger itself by
  mentioning its own handle). Implementation: filter out `payload.sender.type
  == 'Bot' && sender.login == '<our-app>[bot]'`. Ticket: file at implementation
  time so the test catches it.
- Per-repo cost caps and customer-visible spend dashboard. Subscription-level
  spend already exists in `agent_subscriptions`; per-repo split is a UX item.

---

## Appendix: cited files

- [`api/src/openapi/webhooks.rs:130-172`](../../api/src/openapi/webhooks.rs) -- Stripe HMAC verifier (template).
- [`api/src/openapi/webhooks.rs:174-231`](../../api/src/openapi/webhooks.rs) -- Stripe handler header, signature check, dedupe pattern.
- [`api/src/openapi/webhooks.rs:1315-1367`](../../api/src/openapi/webhooks.rs) -- ICPay HMAC verifier (also subsumes into shared util).
- [`api/src/database/contracts/dispute.rs:56-105`](../../api/src/database/contracts/dispute.rs) -- idempotent UPSERT pattern (`ON CONFLICT DO UPDATE` with COALESCE).
- [`api/migrations_pg/043_dispute_pause_state.sql:14-46`](../../api/migrations_pg/043_dispute_pause_state.sql) -- most recent migration; numbering and conventions.
- [`api/src/main.rs:1289-1304`](../../api/src/main.rs) -- webhook route registration block.
- [`api/src/rate_limit.rs:168-185`](../../api/src/rate_limit.rs) -- webhook prefix is already rate-limit-skipped.
- [`agent/docker-compose.yml:11-67`](../../../agent/docker-compose.yml) (outer workspace) -- shell-mode container model that the dispatch step reuses.
- [`tools/src/dc_team/identity.py`](../../../tools/src/dc_team/identity.py) (outer workspace) -- existing per-identity GH-token + HOME flow.
- [`tools/src/dc_team/dispatcher.py`](../../../tools/src/dc_team/dispatcher.py) (outer workspace) -- existing dispatch pipeline, kept for the founder's internal pipeline; the productized backend webhook is a parallel, simpler ingress that targets the same per-identity container.
