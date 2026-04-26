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
column, fail-fast on parse errors, return 2xx after persisting regardless of
downstream dispatch outcome. **Critical: GitHub does NOT automatically retry
failed webhook deliveries** — once we return non-2xx, the event is lost. We
MUST return 2xx after persisting the delivery row even if token minting or
container dispatch fails (the persisted row enables manual replay later). New tables: `github_app_installations` and `github_webhook_deliveries`.
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
2. The API server generates a per-attempt `state` (cryptographic random
   nonce) and `code_verifier` (32-byte base64url-encoded), inserts a row into
   `github_oauth_pending` (DDL in section C) keyed on `state`, and redirects
   to
   `https://github.com/apps/decent-agents/installations/new?state=<state>&code_challenge=<base64url(SHA256(code_verifier))>&code_challenge_method=S256`.
   Note: the installation URL does **NOT** accept `redirect_uri` — that parameter
   belongs to the OAuth authorize endpoint. The App must be configured with
   **"Request user authorization (OAuth) during installation"** enabled, which
   causes GitHub to redirect to the pre-registered **User authorization callback
   URL** (e.g. `https://decent-cloud.org/dashboard/agents/connected/callback`)
   with `?code=<code>&state=<state>` after installation completes.
   `state` and `code_verifier` MUST live in the server-side
   `github_oauth_pending` table and not in a session cookie: cookies travel
   with cross-origin top-level redirects in browsers configured for strict
   SameSite, and are unavailable to the API server during the GitHub-initiated
   callback unless explicitly relaxed. A keyed table avoids that whole class
   of bug and keeps the protocol stateless on the browser side.
3. Customer picks repos to install on (all repos OR a subset).
4. GitHub fires `installation.created` webhook (our handler persists the
   installation and repos with `account_id=NULL`) and then redirects to our
   callback URL with `code` and `state`.
   The callback handler SELECTs from `github_oauth_pending` WHERE
   `state = $1 AND created_at_ns > now - 10 min`. If not found or expired, 400.
   It then exchanges `code` for a user-to-server token via
   `POST https://github.com/login/oauth/access_token` with `client_id`,
   `client_secret`, `code`, and `code_verifier` (read from the row). After
   the token exchange completes, the row is `DELETE`d (single-use) so that
   the same `state` cannot be replayed.
   We then call `GET /user`; the response includes the user's numeric GitHub
   ID, which we match against
   `oauth_accounts(provider='github_oauth', external_id=<github_id_as_text>)`.
   If no linked GitHub OAuth account exists, the callback refuses to link the
   installation and redirects to the dashboard with an explicit linking error.
   We then UPDATE `github_app_installations SET account_id=<matched_account_id>`
   for the installation we received via webhook (correlation rule below).
   Security note: the `installation_id` in the redirect is spoofable — we do
   NOT trust it for identity linkage. We correlate the OAuth-confirmed user
   with the `installation.created` webhook (which is HMAC-verified) via the
   user's stable numeric GitHub ID, never via mutable login.
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
  **1-hour** installation access token scoped to that one installation.
  (GitHub docs: "Installation tokens expire one hour from the time you create them.")
- Tokens are cached in-memory keyed by `installation_id` until 5 minutes before
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
2. **Security:** GitHub-rotated 1-hour tokens scoped per-installation
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
            |        d. Mint installation access token (1 hour TTL)
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
2. **The container never holds the App private key.** It receives a 1-hour
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

- `agent_identities` -- one row per agent persona; columns `id` (BIGSERIAL),
  `subscription_id`, `slug` (e.g. `andris-k85`), `github_actor_login`,
  `home_dir_path`, lifecycle timestamps, etc.
- `agent_subscriptions` -- one row per paying customer subscription; columns
  `id`, `account_id`, `status`, `current_period_end_ns`, ...
- `agent_repos` -- link table; columns `id`, `subscription_id`,
  `github_installation_id`, `github_repo_id`, `github_repo_full_name`,
  `enabled`, `created_at_ns`, `removed_at_ns` (nullable, soft-delete).

Migration order is fixed: #413 may land first and creates its agent tables
without GitHub-table foreign keys. #414 creates the GitHub tables and then adds
the cross-spec foreign keys listed below. If implementation deliberately merges
#413 and #414 into one PR, keep the same ownership boundaries inside one
migration file.

### NEW tables owned by #414

#### OAuth provider migration

This implementation also extends the existing `oauth_accounts.provider` CHECK in
`api/migrations_pg/001_schema.sql` from `('google_oauth')` to
`('google_oauth','github_oauth')` in a forward migration. The GitHub OAuth
callback links installations by looking up
`oauth_accounts(provider='github_oauth', external_id=<numeric_github_user_id>)`.
Do NOT add a parallel GitHub identity column to `accounts`; `oauth_accounts`
already owns external OAuth identity linkage.

Cross-spec migration order is fixed: #413 creates `agent_repos` and `agent_runs`
without references to #414 tables. #414's migration creates
`github_app_installations` and `github_webhook_deliveries`, then adds:

```sql
ALTER TABLE agent_repos
    ADD CONSTRAINT fk_agent_repos_github_installation
    FOREIGN KEY (github_installation_id)
    REFERENCES github_app_installations(github_installation_id);

ALTER TABLE agent_runs
    ADD CONSTRAINT fk_agent_runs_github_delivery
    FOREIGN KEY (github_delivery_id)
    REFERENCES github_webhook_deliveries(github_delivery_id);
```

This lets #413 land independently and makes #414 the only migration that knows
about GitHub-owned tables.

#### `github_app_installations`

```sql
CREATE TABLE github_app_installations (
    id BIGSERIAL PRIMARY KEY,
    github_installation_id BIGINT NOT NULL UNIQUE,
    github_account_login TEXT NOT NULL,
    github_account_id BIGINT NOT NULL,
        -- Numeric GitHub account ID (stable across login renames).
        -- `github_account_login` can change; this is the canonical identity key.
    github_account_type TEXT NOT NULL,
        -- 'User' | 'Organization' (free-text, no CHECK; the GitHub-side enum
        -- is the source of truth and we forward the raw value)
    sender_user_id BIGINT NOT NULL,
        -- Numeric GitHub user id of the human who clicked "Install"
        -- (`payload.sender.id` on the `installation.created` webhook).
        -- Used to correlate this installation with the OAuth user-to-server
        -- token returned to the dashboard callback. The redirect's
        -- `installation_id` URL parameter is unsigned and spoofable; this
        -- column is the canonical correlation key. See section D OAuth
        -- callback step 6.
    account_id BYTEA REFERENCES accounts(id) ON DELETE SET NULL,
        -- Decent Cloud account that installed the App; may be NULL if the
        -- installation predates the OAuth-confirm step (defensive).
    trigger_phrase TEXT NOT NULL DEFAULT '@decent-agent',
        -- Per-installation trigger phrase. The dashboard surfaces it as a
        -- single text input on the connected-repos page. Match is
        -- case-insensitive via `to_lowercase()`.
    suspended_at_ns BIGINT,
        -- non-NULL while GitHub has the installation suspended; we MUST stop
        -- dispatching until it's NULL again.
    removed_at_ns BIGINT,
        -- non-NULL after installation.deleted webhook; soft-delete, never DROP.
    created_at_ns BIGINT NOT NULL,
    updated_at_ns BIGINT NOT NULL
);

CREATE INDEX idx_github_app_installations_account
    ON github_app_installations(account_id)
    WHERE account_id IS NOT NULL AND removed_at_ns IS NULL;
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
    raw_payload BYTEA NOT NULL,
        -- Full body bytes for audit, replay, and debugging. Stored as BYTEA
        -- (not JSONB) because malformed-but-signature-valid payloads may not
        -- be valid JSON. We parse JSON for event routing after persistence;
        -- the raw bytes are the audit trail. Forged deliveries (failed HMAC)
        -- are NOT persisted -- they are warn-logged + metric-counted only
        -- (see section D step 3).
    dispatched_to_identity_id BIGINT REFERENCES agent_identities(id),
        -- NULL when no dispatch happened (control event or trigger not met).
    dispatch_status TEXT NOT NULL DEFAULT 'pending'
        CHECK (dispatch_status IN ('pending','dispatched','skipped','failed')),
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

### Retention policy

Raw webhook payloads contain repo content and may include private repo data.
Retention schedule:
- Non-dispatched deliveries (ping, skipped events): purge after 90 days.
- Dispatched deliveries: retain for 1 year (cross-references `agent_runs` for audit).
- A periodic cleanup job (shared with #410's framework) handles the purge.
- Forged deliveries (failed HMAC) are NEVER persisted (section D step 3); they
  appear only in warn logs and the
  `decent_agents_github_webhook_forgery_count_total` metric counter.

#### `github_oauth_pending`

```sql
CREATE TABLE github_oauth_pending (
    state            TEXT     PRIMARY KEY,
    code_verifier    TEXT     NOT NULL,
        -- Plaintext PKCE verifier (base64url, 32 random bytes). Single-use.
        -- DELETED on successful callback exchange or by the periodic cleanup
        -- job (see below). Storing plaintext is acceptable because the
        -- companion `code_challenge` is sent to GitHub at redirect time and
        -- the row is unreachable to anyone but the API server.
    account_id       BYTEA    NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
        -- The Decent Cloud account that initiated the install flow. Pre-bound
        -- here (the user is logged into the dashboard before clicking
        -- "Connect GitHub"). The OAuth callback uses this to scope the
        -- account_id update on github_app_installations to the same user.
    created_at_ns    BIGINT   NOT NULL
);

CREATE INDEX idx_github_oauth_pending_age ON github_oauth_pending (created_at_ns);
```

The 10-minute callback window is enforced in SQL
(`created_at_ns > now - 10 * 60 * 1_000_000_000`). Rows older than 1 hour are
hard-deleted by a periodic cleanup pass to bound the table size and cap the
plaintext-verifier exposure window. Cleanup follows the
`api/src/cleanup_service.rs` pattern (a `cleanup_once()` step iterates the
periodic loop). Single-use is enforced by the callback handler `DELETE`ing the
row immediately after a successful token exchange (a successful exchange
without DELETE would leave a replayable verifier).

#### `agent_runs` relationship

#413 owns the full `agent_runs` schema. #414 uses its `identity_id BIGINT`,
`github_delivery_id TEXT UNIQUE`, repo metadata, event-ref fields, and
`queued/running/succeeded/failed/cancelled` statuses. If
`github_webhook_deliveries` lands first, add the
`agent_runs.github_delivery_id -> github_webhook_deliveries.github_delivery_id`
foreign key in the #413 migration; otherwise #414 adds the FK.

### Migration file

`api/migrations_pg/0NN_decent_agents_github_integration.sql` where `0NN` is the
next available number at implementation time. Top of `main` at spec authoring
is `045_contract_timeout_states.sql`. Pick whichever number is free when this
lands; the migration runner is order-by-filename, so number conflicts are
surfaced loudly.

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

In `api/src/main.rs` (currently registers webhooks at `main.rs:1318-1333`):

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
       Mismatch -> verify-first-then-DB: do NOT touch the database. Emit
       `tracing::warn!` with the source IP, the `event_type` header (if
       present), and the rejected delivery id; bump the
       `decent_agents_github_webhook_forgery_count_total` metric counter;
       return 401 immediately.

       Rationale: the only legitimate sender is GitHub, and a forged event has
       no audit value beyond the warn-log + metric. Persisting forgeries would
       give an attacker free unbounded INSERTs into a TEXT/BYTEA-heavy table
       (raw_payload is `BYTEA NOT NULL`) and a DoS vector. The Stripe webhook
       handler at `api/src/openapi/webhooks.rs:218` follows the same shape:
       `verify_signature(...)?` runs before any DB call.

       Failure mode caveat: GitHub does **NOT** auto-retry failed deliveries.
       Returning 401 on a forged delivery is acceptable because the sender is
       not GitHub and no legitimate event is lost. For all other errors (token
       mint failure, dispatch failure), we MUST return 2xx after persisting the
       delivery row -- the event is safely stored and can be replayed manually
       or by an internal reprocess job.

    4. INSERT INTO github_webhook_deliveries (delivery_id, event_type, action,
       installation_id, raw_payload, dispatch_status='pending',
       processed_at_ns=now, created_at_ns=now)
       ON CONFLICT (github_delivery_id) DO NOTHING RETURNING id.

       Persistence runs ONLY after signature verification passes; rows in this
       table therefore always represent verified GitHub deliveries.

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

### OAuth callback route

New route: `GET /api/v1/decent-agents/github-oauth/callback?code=<code>&state=<state>`

The callback uses the server-side `github_oauth_pending` table (DDL in
section C) to validate `state`, retrieve the PKCE `code_verifier`, and
correlate the install attempt with the originating Decent Cloud account.
Session cookies are NOT used for this flow.

```text
pub async fn github_oauth_callback(db, query) -> Result<Response, PoemError>:
    1. SELECT code_verifier, account_id, created_at_ns
       FROM github_oauth_pending
       WHERE state = $query.state
         AND created_at_ns > (now_ns() - 10 * 60 * 1_000_000_000);
       Not found / expired -> 400.
    2. POST https://github.com/login/oauth/access_token with:
       client_id=GITHUB_APP_CLIENT_ID,
       client_secret=GITHUB_APP_CLIENT_SECRET,
       code=query.code,
       code_verifier=<retrieved>
       Accept: application/json
    3. Response: { access_token, token_type, scope, expires_in (28800 = 8h) }.
       DELETE FROM github_oauth_pending WHERE state = $query.state. Single-use:
       even if the rest of the handler fails the verifier cannot be reused.
    4. Use the user access token to call GET https://api.github.com/user and
       obtain the installer's GitHub login and numeric `id` (immutable).
    5. Match numeric ID against oauth_accounts where provider='github_oauth'
       and external_id=<numeric_id_as_text>. If no match -> redirect to
       dashboard with error "GitHub account not linked to this Decent Cloud
       account. Link GitHub first, then install the App." The matched
       Decent Cloud account_id MUST equal the account_id pre-bound on the
       pending row (step 1) -- mismatch is a hard 403.
    6. Resolve the installation by GitHub user id (NOT by the redirect's
       installation_id, which is unsigned and spoofable; see correlation
       rule below):
       UPDATE github_app_installations
          SET account_id = <matched_account_id>, updated_at_ns = now_ns()
        WHERE sender_user_id = <github_user_id>
          AND account_id IS NULL;
    7. Find the active agent_subscriptions row for this account and UPDATE
       agent_repos SET subscription_id=<subscription_id> WHERE
       github_installation_id=<installation_id> AND subscription_id IS NULL.
    8. Discard the user access token — it is used once for identity confirmation
       and NOT stored. All ongoing API operations use installation access tokens.
    9. Redirect to /dashboard/agents/connected.
```

Unlinked installation cleanup: a periodic job marks installations older than 24h
with `account_id IS NULL AND removed_at_ns IS NULL` as stale by setting
`removed_at_ns=now` and soft-deleting their `agent_repos`. The dashboard shows a
clear "install again" error for stale rows. This prevents abandoned install flows
from accumulating forever.

### Handler-by-handler behaviour

**`installation.created`:**

```text
let inst = payload["installation"];
INSERT INTO github_app_installations (
    github_installation_id = inst["id"],
    github_account_login   = inst["account"]["login"],
    github_account_id      = inst["account"]["id"],
    github_account_type    = inst["account"]["type"],
    sender_user_id         = payload["sender"]["id"],  -- numeric, immutable
    account_id             = NULL,  -- linked later via dashboard OAuth flow
    created_at_ns = now, updated_at_ns = now
)
ON CONFLICT (github_installation_id) DO UPDATE SET
    github_account_login = EXCLUDED.github_account_login,
    github_account_type  = EXCLUDED.github_account_type,
    sender_user_id       = EXCLUDED.sender_user_id,
    suspended_at_ns      = NULL,
    removed_at_ns        = NULL,    -- reinstall after delete
    updated_at_ns        = EXCLUDED.updated_at_ns;

For each repo in payload["repositories"]:
    INSERT INTO agent_repos (subscription_id=NULL, github_installation_id,
                             github_repo_id, github_repo_full_name)
    -- subscription_id is NULL on purpose. The Decent Agents identity spec
    -- (#413) declares `agent_repos.subscription_id` as nullable specifically
    -- to allow this insert path: the GitHub App webhook `installation.created`
    -- event arrives BEFORE the dashboard OAuth callback completes, so the
    -- subscription is unknown at this moment. The OAuth callback flow
    -- (section D.OAuth callback step 7) sets it once the account+subscription
    -- are linked. Webhook events for unlinked installations are persisted but
    -- not dispatched (DispatchOutcome::skipped, error="installation not
    -- linked to user").
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
    -- subscription_id resolved by joining github_app_installations.account_id
    -- to agent_subscriptions.account_id.
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
1. **Bot self-trigger guard** — if `payload.sender.type == 'Bot'` AND
   `payload.sender.login` matches the GitHub App bot identity
   (e.g. `decent-agents[bot]`), skip: DispatchOutcome::skipped(
   reason="self-trigger guard: sender is our bot"). This prevents infinite
   loops from day one.
2. Resolve installation_id -> agent_identities.id (section E).
   If unresolved: DispatchOutcome::skipped(reason="no active subscription").
3. Apply trigger filter (section G).
   If not triggered: DispatchOutcome::skipped(reason="trigger not matched").
4. Mint installation access token (section F).
5. INSERT INTO agent_runs (status='queued', identity_id,
   github_delivery_id, repo_full_name, github_event_ref=typed_ref,
   run_secret=<32-byte cryptographic random>).
   Store `github_event_ref` as a typed ref (`issue/<number>`, `pull/<number>`,
   `review/<id>`, or `review-comment/<id>`). Guard: reject if an active run
   already exists for the same `(repo_full_name, github_event_ref)` with status
   IN ('queued','running').
   This prevents double-dispatch from two webhook events arriving
   within milliseconds for the same issue/PR.
6. Spawn the dispatch (described in section F).
7. DispatchOutcome::dispatched(identity_id).
```

The spawn itself is ASYNC: the webhook handler returns 200 the moment the row
is in `agent_runs` and the docker exec is invoked. Long agent runs do NOT block
GitHub's 10-second delivery timeout — but since GitHub does not auto-retry,
returning 2xx promptly is essential to avoid the delivery being marked as failed
in GitHub's delivery logs (which are visible to the customer).

### Orphaned run reconciliation

If the API server crashes after INSERT INTO `agent_runs` but before the
container exec completes, or if the container dies mid-run, the `agent_runs`
row is stuck in `status='queued'` or `status='running'` forever. A periodic
reconciliation job (shared with #410's framework) scans:

```sql
SELECT id FROM agent_runs
WHERE status IN ('queued','running')
  AND created_at_ns < now_ns() - (AGENT_RUN_TIMEOUT_MINUTES * 60_000_000_000)
```

For each orphaned row: mark `status='failed'`, set
`failure_reason='reconciler: run timed out or server crashed'`, clear
`run_secret`. Default timeout is 60 minutes; staging may use a shorter value in
tests. Alert ops. This job MUST exist before launch.

---

## E. Repo-to-identity resolution

### v1 rule: implicit-by-subscription

Each customer has at most one active subscription, and #413 enforces
one identity per active subscription. All repos in their installation
map to that identity.

### Algorithm

```text
fn resolve_identity(installation_id: i64, repo_id: i64, db: &Database)
    -> Result<Option<AgentIdentityId>>:

    let inst = SELECT account_id, suspended_at_ns, removed_at_ns
               FROM github_app_installations
               WHERE github_installation_id = $1;
    if inst is None:
        // installation event raced ahead of installation.created;
        // skip, log, do NOT fail.
        return Ok(None);
    if inst.removed_at_ns IS NOT NULL: return Ok(None);
    if inst.suspended_at_ns IS NOT NULL: return Ok(None);
    if inst.account_id IS NULL:
        // Installation not yet linked to a Decent Cloud user. Ignore until
        // the user finishes the dashboard OAuth-confirm step.
        return Ok(None);

    // Check that the specific repo is enabled and not removed.
    let repo = SELECT ar.subscription_id
               FROM agent_repos ar
               WHERE ar.github_installation_id = $1
                 AND ar.github_repo_id = $2
                 AND ar.enabled = TRUE
                 AND ar.removed_at_ns IS NULL;
    if repo is None: return Ok(None);
    if repo.subscription_id IS NULL:
        // Repo not yet linked to a subscription (OAuth step incomplete).
        return Ok(None);

    let row = SELECT ai.id AS identity_id, ai.state
              FROM agent_identities ai
              WHERE ai.subscription_id = $1
                AND ai.state = 'ready'
              LIMIT 1;
    if row is None: return Ok(None);

    Ok(Some(row.identity_id))
```

Note: this resolver takes `repo_id` as a parameter (extracted from the webhook
payload). This prevents dispatch when a repo has been removed from the
installation but the installation itself is still active.

### Why implicit, not explicit-per-repo

- v1 product rule: 1 subscription -> 1 identity. Per-repo selection is
  meaningless.
- Solo founder, ship-fastest principle: the dashboard does NOT need a "pick
  identities per repo" UI in v1.
- Implicit resolution is one query. Three joins max.

### Multi-identity future (deferred)

When the product offers multiple identities per subscription (post-launch,
filed under label `deferred-post-launch`), switch to **explicit-per-repo**:
add `identity_id BIGINT REFERENCES agent_identities(id)` directly on
`agent_repos`, write a small
dashboard UI to assign each repo to one identity. The above algorithm becomes:

```text
SELECT identity_id FROM agent_repos
WHERE github_installation_id=$1 AND github_repo_id=$2 AND removed_at_ns IS NULL;
```

That's the only line that changes. Document this clearly so future-us knows
the shape.

---

## F. Authentication for outgoing API calls

### Goal

Agent containers must NEVER hold the App private key. They get short-lived
installation access tokens scoped to one installation, valid **1 hour**
(GitHub docs: "Installation tokens expire one hour from the time you create them.").

### Flow

```text
[API server, request time]
1. Load GITHUB_APP_PRIVATE_KEY (PEM) at startup. Refuse to start if missing or
   malformed. (Pattern from CLAUDE.md "DEPLOY-TIME VALIDATION".)
2. mint_jwt(app_id, private_key) -> JWT (RS256, 10-min TTL, iss=app_id).
3. POST https://api.github.com/app/installations/:installation_id/access_tokens
   with `Authorization: Bearer <JWT>` -> { token, expires_at }.
4. Cache (installation_id -> (token, expires_at)) in tokio::sync::RwLock<HashMap>.
   Refresh when expires_at - now < 300s (5 minutes before expiry).

[Dispatch to container]
5. docker exec -e AGENT_NAME=<slug>
                 -e GITHUB_TOKEN=<installation_token>
                 -e GITHUB_TOKEN_EXPIRES_AT=<unix_seconds>
                 -e DC_AGENT_RUN_ID=<run_id>
                 -e DC_AGENT_RUN_SECRET=<run_secret_hex>
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

1. **Blast radius.** A compromised container leaks one customer's 1-hour
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

Case-insensitive match via `to_lowercase()`.

Configurable per-installation via the `github_app_installations.trigger_phrase`
column (see DDL in section C). The dashboard surfaces it as a single text input
on the connected-repos page. v1 ships with the default; configurability is a
5-minute UI element.

### Rules

| Event | Trigger? |
|-------|----------|
| `issues.opened` | Trigger if title or body contains the trigger phrase OR has label `decent-agent`. |
| `issues.edited` | Trigger if the edit ADDED the phrase or label (compare changes from `payload.changes`). If `changes` is absent, evaluate the current title/body/labels and log that precise add-vs-existing detection was unavailable. |
| `issues.labeled` | Trigger if the added label is `decent-agent`. |
| `issue_comment.created` | Trigger if comment body contains the trigger phrase. |
| `pull_request.opened` | Trigger if PR description contains the trigger phrase. |
| `pull_request.synchronize` | Trigger iff `sender_is_not_our_bot && (previously_dispatched || contains_trigger_phrase)`. Bot-authored pushes are excluded to prevent self-trigger loops. |
| `pull_request_review.submitted` (`changes_requested`) | Trigger when the review is on a PR previously authored by the agent's bot account. |
| `pull_request_review.submitted` (`approved` or `commented`) | Log only. |
| `pull_request_review_comment.created` | Trigger if comment body contains trigger phrase. |
| Anything else | Log only. |

### "Previously dispatched" tracking

`agent_runs.github_event_ref` records typed event identity (`issue/123`,
`pull/123`, `review/987`, `review-comment/654`) so issue #123 and PR #123 in
the same repo cannot collide. Lookup:

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
`installation_id -> account_id -> active subscription -> identity_id`. This is
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
| 7 | No race between `installation.created` and the customer's dashboard OAuth-confirm | **8/10** | Webhook events for unlinked installations are persisted but skipped. The dashboard step links retroactively (`UPDATE github_app_installations SET account_id=...`). Worst case: customer's first event gets dropped -- log it loudly and surface in dashboard so they retry. |
| 8 | One App private key in env-var (vs HSM/KMS) is acceptable for v1 | **7/10** | Private key is PEM in SOPS, encrypted at rest, decrypted only on the API host. KMS deferred until customer count or compliance demands it. Rotation procedure: register a new key in App settings (GitHub allows two simultaneously), update env, retire old. Document in runbook before launch. |

### Escalations before public launch

- **Trigger abuse:** v1 allows any actor whose event reaches the installed repo to
  trigger work by mentioning the phrase. This can burn a customer's cap. Founder
  decision required before public launch: keep beta behavior, require
  collaborator/write permission, or add an allowlist.
- **Webhook raw-payload retention:** dispatched deliveries retain raw GitHub
  payload bytes for 1 year. Founder/legal decision required before public launch:
  confirm this retention window or shorten it to the minimum operational window.
- **Anthropic key exposure:** see the companion #413 spec. Prompt injection can
  exfiltrate the shared platform key from a customer container; beta acceptance
  must be explicit, and public launch requires the proxy/sidecar follow-up.

---

## I. Acceptance criteria

Tick-box list. Each item maps to a file:line where the change lands.

- [ ] **App registered with required permissions** -- runbook entry in
      `docs/operations/decent-agents-runbook.md` (NEW; section "Initial GitHub
      App registration").
- [ ] **Migration `0NN_decent_agents_github_integration.sql`** creates
      `github_app_installations` (with `sender_user_id BIGINT NOT NULL`),
      `github_webhook_deliveries`, and `github_oauth_pending`, extends
      `oauth_accounts.provider` to allow `github_oauth`, and adds the #414-owned
      FKs from `agent_repos` / `agent_runs` to GitHub tables. File:
      `api/migrations_pg/0NN_*.sql` -- pick the next free number at
      implementation time.
- [ ] **Module split** -- `api/src/openapi/webhooks.rs` becomes
      `api/src/openapi/webhooks/mod.rs` plus per-provider files. The Stripe and
      ICPay HMAC verifiers collapse to a single
      `api/src/openapi/webhooks/util.rs::verify_hmac_sha256_hex`.
- [ ] **`POST /api/v1/webhooks/github`** registered at
      `api/src/main.rs:1318` (next to other webhooks). Rate-limit skip
      already covered by prefix check at `api/src/rate_limit.rs:175`.
- [ ] **Signature verification** -- constant-time HMAC-SHA256 against raw body
      with `GITHUB_APP_WEBHOOK_SECRET`. The shared HMAC utility also migrates
      Stripe and ICPay to constant-time byte comparison. Forgeries are
      verify-first-then-DB: NO row is inserted, source IP and rejected delivery
      id are warn-logged, the `decent_agents_github_webhook_forgery_count_total`
      counter is bumped, and the response is 401. Mirrors the Stripe pattern
      at `api/src/openapi/webhooks.rs:218`.
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
      `tokio::sync::RwLock<HashMap<i64, CachedToken>>`. Startup uses `GET /app`
      to validate `GITHUB_APP_ID` and derive the bot login as `<app_slug>[bot]`.
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
      `GITHUB_APP_ID`, `GITHUB_APP_PRIVATE_KEY`, `GITHUB_APP_WEBHOOK_SECRET`,
      `GITHUB_APP_CLIENT_ID`, or `GITHUB_APP_CLIENT_SECRET` is missing/malformed.
- [ ] **Env templates** -- `api/.env.example` and `cf/.env.example` get the
      five new env vars with comments.
- [ ] **`scripts/dc-secrets`** -- runbook entry showing `dc-secrets set
      shared/env GITHUB_APP_*=...`.
- [ ] **Unit tests**:
      - `webhooks::github::tests::ping_returns_200`
      - `webhooks::github::tests::forged_signature_returns_401_no_db_write`
        (assert NO row in `github_webhook_deliveries`, assert metric counter
        `decent_agents_github_webhook_forgery_count_total` incremented)
      - `webhooks::github::tests::replay_returns_200_without_redispatch`
      - `webhooks::github::tests::installation_created_inserts_row`
      - `webhooks::github::tests::installation_deleted_soft_deletes_repos`
      - `webhooks::github::tests::issue_with_trigger_dispatches`
      - `webhooks::github::tests::issue_without_trigger_skips`
      - `webhooks::github::tests::cross_subscription_isolation`
        (assert installation A's events never resolve to subscription B's identity)
      - `webhooks::github::tests::bot_self_mention_skips_dispatch`
        (assert events from the App's own bot account are skipped)
      - `webhooks::github::tests::duplicate_active_run_prevented`
        (assert second event for same issue/PR while first is queued/running
        returns skipped, not a second dispatch)
      - `webhooks::github::tests::issue_and_pr_numbers_do_not_collide`
        (assert typed `github_event_ref` keeps `issue/123` separate from
        `pull/123`)
      - `webhooks::github::tests::removed_repo_skips_dispatch`
        (assert events for a removed repo are skipped even if installation is active)
      - `webhooks::github::tests::github_oauth_callback_links_by_numeric_id`
        (assert correlation uses `sender_user_id` from the webhook payload,
        NOT the redirect's `installation_id` URL parameter)
      - `webhooks::github::tests::oauth_pending_state_mismatch_returns_400`
        (assert callback rejects unknown / expired `state` without contacting
        GitHub)
      - `webhooks::github::tests::oauth_pending_row_is_single_use`
        (assert callback DELETEs the `github_oauth_pending` row after
        successful exchange; second callback with the same state -> 400)
      - `webhooks::github::tests::stale_unlinked_installation_is_soft_deleted`
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
- Per-repo cost caps and customer-visible spend dashboard. Subscription-level
  spend already exists in `agent_subscriptions`; per-repo split is a UX item.
- Trigger abuse protection (e.g. any public commenter can trigger paid agent
  work via `@decent-agent`). v1 trusts that installation repo access =
  authorization to trigger. Post-launch: require collaborator permission or
  an allowlist. File as `decent-agents,deferred-post-launch`.
- Metrics/alerting (dispatch failure rate, token mint latency, queue depth).
  The `dispatch_status` and `processed_at_ns` columns support this; concrete
  Prometheus counters and alert thresholds are post-launch.

---

## Appendix: cited files

- [`api/src/openapi/webhooks.rs:130-172`](../../api/src/openapi/webhooks.rs) -- Stripe HMAC verifier (template).
- [`api/src/openapi/webhooks.rs:174-231`](../../api/src/openapi/webhooks.rs) -- Stripe handler header, signature check, dedupe pattern.
- [`api/src/openapi/webhooks.rs:1315-1367`](../../api/src/openapi/webhooks.rs) -- ICPay HMAC verifier (also subsumes into shared util).
- [`api/src/database/contracts/dispute.rs:56-105`](../../api/src/database/contracts/dispute.rs) -- idempotent UPSERT pattern (`ON CONFLICT DO UPDATE` with COALESCE).
- [`api/migrations_pg/043_dispute_pause_state.sql:14-46`](../../api/migrations_pg/043_dispute_pause_state.sql) -- most recent migration pattern for `_ns` timestamps, UNIQUE on external ID, partial indexes.
- [`api/src/main.rs:1318-1333`](../../api/src/main.rs) -- webhook route registration block.
- [`api/src/rate_limit.rs:168-185`](../../api/src/rate_limit.rs) -- webhook prefix is already rate-limit-skipped.
- [`agent/docker-compose.yml:11-67`](../../../agent/docker-compose.yml) (outer workspace) -- shell-mode container model that the dispatch step reuses.
- [`tools/src/dc_team/identity.py`](../../../tools/src/dc_team/identity.py) (outer workspace) -- existing per-identity GH-token + HOME flow.
- [`tools/src/dc_team/dispatcher.py`](../../../tools/src/dc_team/dispatcher.py) (outer workspace) -- existing dispatch pipeline, kept for the founder's internal pipeline; the productized backend webhook is a parallel, simpler ingress that targets the same per-identity container.
