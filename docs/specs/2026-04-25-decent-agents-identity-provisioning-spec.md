# Decent Agents -- Per-Subscription Identity Provisioning Spec

- Issue: decent-stuff/decent-cloud#413
- Date: 2026-04-25 (Europe/Zurich)
- Author: Backend API agent (planning only, no code)
- Status: SPEC ONLY -- implementation deferred. Closes the planning phase
  of #413; future PRs will reference back to this document.
- Related: #414 (GitHub App integration), #415 (subscription billing /
  cap enforcement), #410 (stale-pending cleanup -- shared periodic-job
  primitive), #408 + #421 + #424 (Phase 1/2 Stripe disputes -- pause
  primitives reused here), #423 (`agents_waitlist` -- pattern reused).

## 0. Goal and non-goals

Goal: define the data model, lifecycle, provisioning pipeline, and
secret-management strategy for the per-subscription "agent identity"
that ships with every Decent Agents subscription (single tier, CHF
49/month, EU-hosted on Hetzner, 7-day grace on cancel). The output is
detailed enough that an implementing agent can stop guessing about
schema, state transitions, or ownership boundaries.

Non-goals (each filed or noted as `deferred-post-launch`):
- Multi-identity per subscription (#413-followup; v1 is 1:1).
- BYOK Anthropic key per customer (section H, deferred).
- Cross-host failover for the Hetzner box (section I.5, single-host
  for v1; downtime is acceptable for solo-founder posture).
- Admin UI for identity management (separate ticket; CLI/SQL only at
  launch).
- Per-customer Anthropic billing exports (section G; #415 owns it).

## 1. Source citations (read before changing anything)

- `repo/api/migrations_pg/041_agents_waitlist.sql:1-13` -- newest table
  pattern (BIGSERIAL, `created_at TIMESTAMPTZ`, descending index).
- `repo/api/migrations_pg/043_dispute_pause_state.sql:14-46` -- pattern
  for nanosecond timestamps, partial indexes, and ALTER TABLE additions
  alongside a new sibling table.
- `repo/api/migrations_pg/045_contract_timeout_states.sql` -- current
  latest migration (next free = 046).
- `repo/api/src/database/agents_waitlist.rs:12-54` -- newest DB-layer
  module pattern (struct + `Database` impl + `Result` errors with
  context; tests assert idempotency and ordering).
- `repo/api/src/database/contracts/dispute.rs:56-269` -- pause/resume
  primitives we will reuse for cap-driven and dispute-driven identity
  pauses; in particular the idempotent re-pause check (`:123-137`) and
  the credited-pause-interval bookkeeping (`:220-235`).
- `repo/api/migrations_pg/001_schema.sql:255-304` -- `accounts` table
  (the `users` referent the spec body calls `users`); already has
  `stripe_customer_id`, `subscription_*` columns we will piggyback on.
- `repo/api/migrations_pg/001_schema.sql:736-767` --
  `subscription_plans` and `subscription_events`; the new
  `agent_subscriptions` table sits ALONGSIDE these and references the
  same `accounts.id` BYTEA primary key (16-byte random).
- `repo/api/src/openapi/webhooks.rs:465-692` -- existing
  `customer.subscription.created`, `customer.subscription.updated`,
  `customer.subscription.deleted` handlers. Today they update
  `accounts.subscription_*`; new code dispatches off the same events
  to provision/destroy identities.
- `repo/api/src/openapi/webhooks.rs:723-739` -- the `charge.dispute.*`
  cluster shows the pattern of "wrap a Phase-1 DB primitive in a
  webhook arm with idempotent-on-replay semantics + 2xx-on-soft-fail".
  Identity provisioning follows the same shape.
- `repo/dc-agent/src/provisioner/mod.rs:98-150` -- `Provisioner` trait
  (`provision`, `terminate`, `stop`, `health_check`, `get_instance`,
  `list_running_instances`, `verify_setup`, `collect_resources`).
  Decent Agents containers slot in via either an extension to the
  existing Docker provisioner (`docker.rs`) or a new
  `agent_container.rs` sibling -- decision in section D.
- `repo/dc-agent/src/provisioner/docker.rs:103-224` -- existing
  Docker-container provisioner. Already does image pulls, port
  bindings, CPU/memory limits, restart policy, dc-agent labels. The
  hard parts (name->container_id mapping, ssh key injection) are
  done; what is missing is "long-lived without contract_id".
- `agent/docker-compose.yml:11-79` (outer workspace) -- shell-mode
  reference: `command: sleep infinity`, `docker exec` for each prompt,
  per-agent target/cargo/rustup volumes, host docker socket bind-mount
  for nested-Docker uses. THIS is the closest analog to what each
  Decent Agents customer container looks like.
- `tools/src/dc_team/dispatcher.py:1-150` and
  `tools/src/dc_team/identity.py` -- founder's internal pipeline
  primitive. Each identity has a slug (`andris-kalns`,
  `roberts-kalejs`, ...), a HOME under `tools/homes/dc-<slug>/`,
  GitHub PAT and credentials sourced from
  `secrets/hires/<slug>/env.yaml` via SOPS. Decent Agents
  productizes the slug/HOME/container shape, but replaces persistent
  per-identity PATs with #414 GitHub App installation tokens minted
  per dispatch.

## 2. Where this fits in the codebase

```text
repo/
|- agent/                                             NEW (productized runtime
|                                                       image; ported from the
|                                                       outer workspace agent/)
|- api/
|  |- migrations_pg/046_agent_identities.sql        NEW (this spec; use the
|  |                                                  next free number if a
|  |                                                  peer migration lands
|  |                                                  first)
|  |- src/database/
|  |  |- agent_subscriptions.rs                     NEW
|  |  |- agent_identities.rs                        NEW
|  |  |- agent_runs.rs                              NEW (foundation for #415)
|  |  |- agent_repos.rs                             NEW (#414 join table)
|  |  `- mod.rs                                     edit (re-export new modules)
|  `- src/openapi/
|     `- webhooks.rs                                edit (dispatch identity
|                                                       provisioning off
|                                                       customer.subscription.*)
`- dc-agent/
   `- src/provisioner/
      |- agent_container.rs                         NEW (long-lived shape;
      |                                                  see section D for the
      |                                                  extend-vs-new decision)
      `- mod.rs                                     edit (register provisioner)
```

A. ARCHITECTURE OVERVIEW
========================

```text
                         Stripe (EU)
                             |
                             | customer.subscription.created
                             | customer.subscription.deleted
                             | invoice.payment_failed
                             v
                  +-----------------------+
                  |  api-server           |
                  |  /api/v1/webhooks/    |   (poem handler)
                  |    stripe             |
                  +-----------+-----------+
                              |
                +-------------+-------------+
                |                           |
                v                           v
   +--------------------+        +-----------------------+
   | Postgres           |        | dc-agent provisioner  |
   |  agent_subscript.. |        |  (Hetzner host A)     |
   |  agent_identities  |        |   - Docker daemon     |
   |  agent_runs        |        |   - HOMEs under       |
   |  agent_repos       |        |     /home/dc-agent-<slug>/
   +--------------------+        +-----------+-----------+
                                             |
                                             | docker run
                                             v
                                +------------------------+
                                | Customer container     |
                                |  (long-lived, sleep    |
                                |   infinity)            |
                                |   image:               |
                                |     dc-agent-runtime   |
                                |   network: bridge      |
                                |   volume: HOME:/home   |
                                +-----------+------------+
                                            |
                  docker exec opencode/Claude per event
                                            |
                                            v
                              +-------------------------+
                              | Anthropic API           |
                              |  (shared platform key)  |
                              +-------------------------+

                   ^                         ^
                   |                         |
                   |                         |
                   |                         |
    GitHub repo: #414 GitHub App webhook -> /api/v1/webhooks/github
           (resolves installation/repo -> identity_id, then dispatches)
                        |
                        v
           api-server -> host-control channel -> docker exec <slug> ...
                         (extends the dc-agent reconcile loop; see D.8)
```

Three concepts, three tables, three lifetimes:

1. **Customer** (`accounts` table, existing,
   `repo/api/migrations_pg/001_schema.sql:255`). One human (or org).
   Lifetime: forever-ish. Has 0..N agent subscriptions over time.
2. **Subscription** (`agent_subscriptions`, NEW). One Stripe-mediated
   recurring billing entity. Lifetime: from
   `customer.subscription.created` to `customer.subscription.deleted`
   (with 7-day grace before destruction). Each subscription has 0..1
   identity at a time during launch tier; multi-identity subscriptions
   are `deferred-post-launch`.
3. **Identity** (`agent_identities`, NEW). The agent's environment:
   slug, HOME dir on the Hetzner host, GitHub App actor, container,
   runtime state.
   Lifetime: from the async provisioning job that fires after subscription
   create through the 7-day-grace teardown after subscription delete.
   1:1 with subscription at any moment; new subscription -> new identity
   (slugs are not reused).

Confidence: 9/10 -- this matches the founder's internal execution model
(slug = `dc_team` slug, HOME dir = `tools/homes/dc-<slug>/`) while
using the #414 GitHub App model for product authentication.

B. DATA MODEL
=============

All four new tables are added by a single forward-only migration
`046_agent_identities.sql` in this checkout. If another peer branch lands
first, renumber to the next free migration before merge. Nanosecond
timestamps everywhere (matches contract / dispute schema; `accounts`
and `agents_waitlist` use
TIMESTAMPTZ -- we choose `_ns` to align with the dc-agent and dispute
ecosystem because most consumers of these rows are operational not
human-readable).

### B.1 `agent_subscriptions`

```sql
-- 1:1 with a Stripe Subscription. Independent of accounts.subscription_*
-- fields, which today track plan/period for non-agents (the website's
-- Pro tier). Decent Agents has its own subscription product code path
-- so the join column is account_id (NOT users -- the existing table is
-- named `accounts`).
CREATE TABLE agent_subscriptions (
    id                       BIGSERIAL    PRIMARY KEY,
    account_id               BYTEA        NOT NULL REFERENCES accounts(id)
                                                    ON DELETE RESTRICT,
    stripe_subscription_id   TEXT         NOT NULL UNIQUE,
    -- Stripe subscriptions are bound to a single customer for their lifetime.
    -- Duplicating this value is intentional: webhook handlers can match the
    -- payload directly without joining through accounts, and there is no sync
    -- path because Stripe does not move a subscription between customers.
    stripe_customer_id       TEXT         NOT NULL,
    -- Normalized billing status only. Compute pauses live in
    -- agent_identities.state; do NOT write cap pauses here.
    -- Existing webhook mapping (webhooks.rs:539-545) treats incomplete as
    -- past_due and incomplete_expired/unpaid/canceled as canceled.
    status                   TEXT         NOT NULL CHECK (status IN
                                            ('active','trialing','past_due','canceled')),
    -- v1 has one tier; the column exists so #415 can introduce 'pro_plus'
    -- without a schema migration.
    tier                     TEXT         NOT NULL DEFAULT 'pro',
    current_period_start_ns  BIGINT       NOT NULL,
    current_period_end_ns    BIGINT       NOT NULL,
    canceled_at_ns           BIGINT,
    -- Audit metadata.
    created_at_ns            BIGINT       NOT NULL,
    updated_at_ns            BIGINT       NOT NULL
);

CREATE INDEX idx_agent_subscriptions_account
    ON agent_subscriptions (account_id);
CREATE INDEX idx_agent_subscriptions_status
    ON agent_subscriptions (status);
-- Partial index over still-billable rows -- supports the cap-renewal
-- and grace-period periodic jobs cheaply.
CREATE INDEX idx_agent_subscriptions_active
    ON agent_subscriptions (current_period_end_ns)
    WHERE status IN ('active','trialing','past_due');
```

### B.2 `agent_identities`

```sql
-- 1:1 with agent_subscriptions in v1 (UNIQUE on subscription_id).
-- The constraint is dropped in the multi-identity follow-up.
CREATE TABLE agent_identities (
    id                       BIGSERIAL    PRIMARY KEY,
    subscription_id          BIGINT       NOT NULL UNIQUE
                                          REFERENCES agent_subscriptions(id)
                                          ON DELETE RESTRICT,
    -- Server-generated wordlist+suffix slug, e.g. 'decent-bot-h7k2p9'.
    -- See section I.4 for the collision-avoidance rationale.
    slug                     TEXT         NOT NULL UNIQUE
                                          CHECK (slug ~ '^[a-z][a-z0-9-]{2,62}$'),
    -- Absolute path on the Hetzner host: /home/dc-agent-<slug>.
    home_dir_path            TEXT         NOT NULL,
    -- GitHub actor used for customer-visible API writes. v1 uses the
    -- GitHub App bot login from #414; this is NOT a credential.
    github_actor_login       TEXT         NOT NULL,
    -- Docker container ID on the Hetzner host. NULL while provisioning,
    -- cleared on destroy.
    container_id             TEXT,
    -- Which dc-agent host runs this container. v1: a single string,
    -- e.g. 'hetzner-eu-helsinki-1'. Looking up the host's IP/SSH config
    -- lives in dc-agent host registry, NOT in this row.
    hetzner_host_id          TEXT         NOT NULL,
    -- Lifecycle state -- see section C.2 and section D.8.5.
    state                    TEXT         NOT NULL CHECK (state IN
                                            ('provisioning','provisioning_failed',
                                             'ready','paused',
                                             'tearing_down','destroyed')),
    -- Pause bookkeeping (mirrors contract_sign_requests; reuses the
    -- dispute pause primitive at contracts/dispute.rs:115).
    pause_reason             TEXT,
    paused_at_ns             BIGINT,
    total_paused_ns          BIGINT       NOT NULL DEFAULT 0,
    -- Time the provisioning task started; provisioned_at = container
    -- launched OK; ready_at = first successful health check.
    provisioned_at_ns        BIGINT,
    ready_at_ns              BIGINT,
    last_activity_ns         BIGINT,
    -- Set exactly once when state enters tearing_down. The 7-day grace
    -- cleanup job keys off this value; destroyed_at_ns is only populated
    -- after archive + delete have completed.
    teardown_started_at_ns   BIGINT,
    destroyed_at_ns          BIGINT,
    -- Soft-archive pointer (S3 URI or local path) populated at
    -- teardown. NULL until destroyed; see section I.8.
    archive_location         TEXT,
    -- Periodic teardown retry bookkeeping. Increment on archive/delete
    -- failure, leave state='tearing_down', and alert per section D.7.
    teardown_failure_count   INT          NOT NULL DEFAULT 0,
    last_teardown_error      TEXT,
    created_at_ns            BIGINT       NOT NULL,
    updated_at_ns            BIGINT       NOT NULL
);

-- UNIQUE constraints on subscription_id and slug already create backing
-- indexes; do not add duplicate unique indexes in the migration.
CREATE INDEX idx_agent_identities_state
    ON agent_identities (state);
CREATE INDEX idx_agent_identities_host
    ON agent_identities (hetzner_host_id, state);
-- Partial index for the 7-day grace cleanup job (only tearing_down
-- rows are interesting; ready/paused/destroyed are filtered out).
CREATE INDEX idx_agent_identities_tearing_down
    ON agent_identities (teardown_started_at_ns)
    WHERE state = 'tearing_down';
```

### B.3 `agent_runs`

Foundational for #415 cap enforcement. We spec it here so #415 inherits
a concrete schema. ONE row per "agent activation" (a GitHub event came
in and the agent worked on it). Tokens are denormalized counters; the
final receipt is whatever Anthropic billing reconciles.

```sql
CREATE TABLE agent_runs (
    id                       BIGSERIAL    PRIMARY KEY,
    identity_id              BIGINT       NOT NULL REFERENCES agent_identities(id)
                                                   ON DELETE CASCADE,
    -- Stripe-replay-style idempotency: GitHub Delivery-ID UUID.
    -- Identical event = no-op upsert (reject duplicate inserts).
    github_delivery_id       TEXT         NOT NULL UNIQUE,
    -- GitHub App installation that produced the token used for this run.
    github_installation_id   BIGINT       NOT NULL,
    -- Numeric GitHub repo ID (stable even if repo is renamed).
    repo_id                  BIGINT       NOT NULL,
    repo_full_name           TEXT         NOT NULL,
    github_event_kind        TEXT         NOT NULL,
    github_event_ref         TEXT         NOT NULL,
    -- Hashed per-run secret for the mid-run token refresh endpoint (#414).
    -- The plaintext secret (32-byte cryptographic random from `ring::rand`) is
    -- generated at queue time and passed to the container as DC_AGENT_RUN_SECRET
    -- env var; it is NEVER persisted. We persist only its SHA-256 digest.
    -- SHA-256 of the 32-byte secret. Compare with subtle::ConstantTimeEq against
    -- the supplied secret in the refresh-token endpoint.
    -- Cleared (set NULL) when the run reaches a terminal state.
    run_secret_hash          BYTEA,
    -- Denormalized from agent_subscriptions.current_period_start_ns at insert
    -- time. #415 cap checks are equality scans on (identity_id,
    -- billing_period_start_ns), not range joins against mutable subscription
    -- period columns.
    billing_period_start_ns  BIGINT       NOT NULL,
    queued_at_ns             BIGINT       NOT NULL,
    started_at_ns            BIGINT,
    ended_at_ns              BIGINT,
    duration_ms              BIGINT,
    -- Token counts; populated by the opencode bridge after each
    -- prompt round-trip. NULL while running.
    claude_input_tokens      BIGINT,
    claude_output_tokens     BIGINT,
    status                   TEXT         NOT NULL CHECK (status IN
                                            ('queued','running','succeeded',
                                             'failed','cancelled')),
    failure_reason           TEXT,
    result_summary           TEXT,
    created_at_ns            BIGINT       NOT NULL,
    updated_at_ns            BIGINT       NOT NULL
);

CREATE INDEX idx_agent_runs_identity_started
    ON agent_runs (identity_id, queued_at_ns DESC);
CREATE INDEX idx_agent_runs_billing_period
    ON agent_runs (identity_id, billing_period_start_ns);
CREATE INDEX idx_agent_runs_status
    ON agent_runs (status)
    WHERE status IN ('queued','running');
-- The github_delivery_id UNIQUE constraint already creates the replay index;
-- do not add a duplicate unique index.
```

Cross-spec FK ordering is fixed: #413 creates `agent_runs` without a foreign key
to `github_webhook_deliveries`. #414 owns that FK because #414 creates the
referenced GitHub delivery table.

### B.4 `agent_repos`

Join table mapping an installed GitHub repository to an agent
subscription. The active identity is resolved by joining
`agent_repos.subscription_id -> agent_subscriptions.id ->
agent_identities.subscription_id`. Soft-delete via `removed_at_ns` (a
customer might uninstall the App from one repo while keeping others; we
don't lose historical run rows that reference the repo via
`agent_runs.repo_id`).

```sql
CREATE TABLE agent_repos (
    id                       BIGSERIAL    PRIMARY KEY,
    -- Nullable: the GitHub App webhook flow in #414 inserts rows on
    -- `installation.created` BEFORE the OAuth callback links the installation
    -- to a Decent Cloud account+subscription. The link is established later by
    -- the OAuth callback flow (#414 section D, OAuth callback step 7), which
    -- UPDATEs subscription_id once the account is matched. Until then, webhook
    -- events for the installation are persisted but never dispatched (the
    -- resolver in #414 section E returns None when `subscription_id IS NULL`).
    subscription_id          BIGINT       REFERENCES agent_subscriptions(id)
                                                   ON DELETE RESTRICT,
    github_installation_id   BIGINT       NOT NULL,
    github_repo_id           BIGINT       NOT NULL,
    github_repo_full_name    TEXT         NOT NULL,
    enabled                  BOOLEAN      NOT NULL DEFAULT TRUE,
    installed_at_ns          BIGINT       NOT NULL,
    removed_at_ns            BIGINT,
    created_at_ns            BIGINT       NOT NULL,
    updated_at_ns            BIGINT       NOT NULL
);

-- Same repo can be added/removed/added again under the same installation.
-- UNIQUE on active rows only.
CREATE UNIQUE INDEX idx_agent_repos_active
    ON agent_repos (github_installation_id, github_repo_id)
    WHERE removed_at_ns IS NULL;
CREATE INDEX idx_agent_repos_subscription
    ON agent_repos (subscription_id)
    WHERE subscription_id IS NOT NULL AND removed_at_ns IS NULL;
```

#414 owns `github_app_installations`. If that table exists before this
migration, add `github_installation_id REFERENCES
github_app_installations(github_installation_id)`; otherwise #414 adds
the FK when it lands.

C. LIFECYCLE STATE MACHINES
===========================

### C.1 `agent_subscriptions.status`

```text
                         Stripe customer.subscription.created
                                       |
                                       v
                              +----------------+
                              | active/trialing|
                              +-------+--------+
                                      |
              invoice.payment_failed  | subscription.updated -> active
                  --------+           | (recovery)
                          v           ^
                       +----+----+    |
                       | past_due+----+
                       +----+----+
                            |
             subscription.deleted / unpaid / incomplete_expired
                            |
                            v
                      +----------+
                      | canceled |
                      +----------+
```

`agent_subscriptions.status` is billing status only. Disputes and cap
exhaustion pause compute by changing `agent_identities.state`; they do
not write `agent_subscriptions.status='paused'`.

Transitions and triggers (exhaustive):

| From             | To       | Trigger                                                                                  |
|------------------|----------|------------------------------------------------------------------------------------------|
| (insert)         | active   | Stripe `customer.subscription.created` with status=active                                 |
| (insert)         | trialing | Stripe `customer.subscription.created` with status=trialing                               |
| (insert)         | past_due | Stripe `customer.subscription.created` with status=past_due or incomplete                 |
| active, trialing | past_due | Stripe `customer.subscription.updated` with status=past_due, OR `invoice.payment_failed` |
| past_due         | active   | Stripe `customer.subscription.updated` with status=active (recovered)                    |
| any              | canceled | `customer.subscription.deleted`, `unpaid`, `incomplete_expired`, OR dispute event          |

Edge cases:
- `past_due` and identity `paused` may coexist. Billing recovery moves
  subscription status back to `active`; compute resumes only when the
  corresponding cap pause reason is cleared.
- `canceled` is terminal. Re-subscribing creates a NEW
  `agent_subscriptions` row (Stripe issues a new subscription_id);
  the old identity is NOT revived.

### C.2 `agent_identities.state`

```text
                           subscription.created -> async provision job
                                       |
                                       v
                                +-------------+
                                | provisioning|   (container creating, HOME
                                +------+------+    setup, App actor recorded)
                                       |
                          health check passes
                                       v
                                  +-------+
                                  | ready |  <----+
                                  +---+---+        \
                                      |             \  resume
                                      |              \  (cap window reset)
                                      | pause         \
                          (cap exhaustion)           \
                                      v                 |
                                +--------+              |
                                | paused |--------------+
                                +----+---+
                                     |
                  subscription.deleted | hard-fail | beta dispute event
                                     v
                              +-------------+
                              | tearing_down|  (stop container, archive HOME,
                              +------+------+   stop accepting GitHub events)
                                     |
                          7-day grace job fires
                                     v
                              +-----------+
                               | destroyed |  (container removed, HOME archived
                               +-----------+   to S3, retention per I.8/GDPR,
                                               row stays for audit)
```

Transitions and triggers:

| From          | To            | Trigger                                                              |
|---------------|---------------|----------------------------------------------------------------------|
| (insert)      | provisioning  | async provisioning task started after `subscription.created`         |
| provisioning  | ready         | dc-agent reports container running AND first health check passes     |
| provisioning  | tearing_down  | subscription deleted, beta dispute event, or provisioning failed permanently |
| ready         | paused        | cap exhaustion -- writes `pause_reason`                              |
| paused        | ready         | cap window rolled over                                                |
| ready, paused | tearing_down  | `subscription.deleted` OR beta dispute event                         |
| tearing_down  | destroyed     | grace timer (7 days from `teardown_started_at_ns`) fires periodic job |
| any           | destroyed     | operator-initiated kill (admin SQL only, audit-logged)               |

Every transition MUST be implemented as a compare-and-swap update:

```sql
UPDATE agent_identities
SET state = $new_state, updated_at_ns = $now_ns, ...
WHERE id = $identity_id AND state = $expected_old_state
RETURNING *;
```

If zero rows are returned, the handler MUST `SELECT` the current row before any
side effect. Treat it as an idempotent replay only when the row already matches
the requested target state and transition timestamp/effect marker. Otherwise log
warn/error with old state, requested transition, and event id, return the current
row, and do not issue the side effect. This is the same race-control shape as the
identity state. The pattern is the same race-control shape as the
marketplace contract pause primitive.

Edge cases:
- `provisioning -> tearing_down` skips `ready` because the container
  never came up. Section D defines retries before we give up.
- Beta dispute handling is intentionally simple: any DA subscription dispute
  event tears the identity down. No dispute pause/resume path exists for beta;
  disputes are not a beta objective.
- `paused` (cap exhaustion) MUST distinguish `pause_reason` from
  operator pause so the resume path knows which side to consult. Reasons used:
  `cap_exhausted:<cycle_end_ns>` and `operator:<memo>`. The
  `agent_identities.pause_reason` column is TEXT to keep this open-ended.
- Identity `paused` does NOT pause the Stripe subscription -- billing keeps
  running. The customer is still paying for the cycle; pause just
  stops compute.
- The 7-day grace can be cut short by a customer-initiated
  "destroy now" admin action (post-launch). v1: grace is fixed.

D. PROVISIONER INTEGRATION
==========================

### D.1 Extend Docker provisioner vs new provisioner type

Recommendation: **new `agent_container.rs` provisioner type**.
Confidence: 8/10.

Rationale:
- The existing `DockerProvisioner` (`dc-agent/src/provisioner/docker.rs:103-224`)
  is parameterized on `ProvisionRequest.contract_id` and calls
  `extract_contract_id("dc-<id>")`. Decent Agents does NOT have a
  contract_id; it has a slug. Forcing it into the contract concept
  pollutes the marketplace data model and makes `cancel_contract` /
  `terminate_contract_for_dispute_lost` accidentally applicable.
- The shape is different: a marketplace VM is created, runs for
  hours-to-weeks, and is destroyed at end-of-contract. An agent
  container is created once at subscription start, lives for as
  long as the customer keeps paying (months to years), and is
  paused/resumed weekly across cap cycles. Different lifetime, different
  reconcile semantics.
- Code reuse is still high: the new provisioner can share the
  bollard helpers (`pull_image_if_needed`, `build_container_config`,
  IP extraction, container_to_instance) by extracting them into a
  `dc-agent/src/provisioner/docker_common.rs` private module. Net
  duplication: zero, while the public surface stays clean.

Implementation boundary: do NOT pass a fake `contract_id` through
`ProvisionRequest`. The existing `Provisioner::provision(&ProvisionRequest)`
is contract-shaped (`contract_id`, `offering_id`, SSH pubkey) and should remain
the marketplace interface. `agent_container.rs` defines an explicit
`AgentProvisionRequest { slug, tier, hetzner_host_id, home_dir_path, image }`
and an `AgentRuntimeProvisioner` surface with `provision`, `stop`, `start`,
`terminate`, `health_check`, and `list_agent_containers`. If a later refactor
folds this into the common `Provisioner` trait, `start()` must be an explicit
required method for the agent path; missing start/resume support is a compile-
time error, not a runtime surprise.

### D.2 Pause/resume reconciliation actions

The existing `ReconcilePauseInstance` bucket is NOT reused as-is: the current
type in `common/src/api_types.rs` requires `contract_id`, and Decent Agents has
no contract. Reusing it would create a fake marketplace contract identifier and
cross the two domains.

Add explicit agent-shaped reconciliation actions in `common/src/api_types.rs`:

```rust
pub struct ReconcilePauseAgentIdentity {
    pub external_id: String,    // Docker container ID or name
    pub identity_slug: String,
    pub reason: String,         // cap_exhausted:<cycle_end_ns> | operator:<memo>
}

pub struct ReconcileResumeAgentIdentity {
    pub external_id: String,
    pub identity_slug: String,
    pub reason: String,
}
```

`ReconcileResponse` gains `pause_agents` and `resume_agents` buckets (both
`#[serde(default)]`). The marketplace `pause` bucket remains contract-only.
Agent pause routes to `AgentRuntimeProvisioner::stop()`; resume routes to
`AgentRuntimeProvisioner::start()`. Both operations must preserve the HOME
volume and container identity; destroy is only allowed through the teardown
state machine. This is the current recommendation, not a quiet compromise: if
the founder wants a generic reconcile action instead, decide that before editing
`common/` (see I.9).

### D.3 Container image bill of materials

Image: `decent-agents-runtime:<commit-sha>` (versioned with the api
server's commit SHA, NOT `latest`, so a customer's container can be
pinned across server upgrades). Built from `agent/Dockerfile` inside
the product repo. The implementation PR must port the current outer
workspace `../agent/` runtime into `repo/agent/`; product CI must not
depend on files outside the submodule.

Required dependencies (justified):
- **Rust toolchain** (`rustup` + stable + clippy + rustfmt) -- the
  agent works on Rust monorepos; needed to run `cargo check`,
  `cargo clippy`, `cargo nextest`. Cost: ~1.5 GB on disk per host
  (shared via `target-cache` / `cargo-cache` named volumes,
  matching `agent/docker-compose.yml:47-49`).
- **Node.js + Bun** -- for repos with TypeScript/JS, and because
  the opencode bridge (`agent/opencode_bridge.js`) is a Node
  script. Cost: ~200 MB.
- **Playwright** -- for any UI verification step the agent performs.
  Cost: ~500 MB (Chromium + headless deps).
- **age + sops** -- to read per-identity secrets sourced from
  the dc-agent host's secret bundle (NOT shared across customers).
  Cost: <10 MB.
- **opencode runtime** (`@opencode-ai/sdk` v2 + the opencode CLI at
  `/home/ubuntu/.opencode/bin/opencode`) -- the actual model client,
  per the founder's pipeline (memory: "Model IDs use dashes not
  dots").
- **PostgreSQL client** (`psql`) -- for repos with DB-touching tests.
  Server is NOT included; tests can run their own.
- **Standard Linux toolchain** -- git, ssh, curl, jq, gh CLI (for
  `gh pr create`, `gh pr review`).

Total expected image size: ~3 GB. Pulled once per Hetzner host; new
container instances reuse the layer cache.

### D.4 Per-identity HOME dir layout

```text
/home/dc-agent-<slug>/                       (mounted into container as $HOME)
|- .config/
|  |- git/config                             (safe.directory + author defaults)
|  |- gh/                                    (CLI config only; auth comes from
|  |                                           GITHUB_TOKEN env per dispatch)
|  `- opencode/                              (model config, OAuth state -- bind
|                                              mount from host opencode root)
|- .anthropic/                               (Anthropic API key for v1 = shared
|                                              platform key; v2 = BYOK)
|- workspaces/
|  `- <repo-slug>/                           (one git checkout per agent_repos
|                                              row; cloned lazily on first
|                                              event)
|- .cache/                                    (cargo + npm + bun caches; named
                                                volume per host so concurrent
                                                customers do not invalidate
                                                each other's caches)
`- audit.jsonl                                (per-identity append-only run log)
```

The HOME is owned by uid `1000:1000` inside the container (matching
`agent/docker-compose.yml:21`). Launch isolation is one customer per VM, so a
customer cannot read or mutate a sibling's workspace through the host
filesystem. Inside that VM, use one runtime user with `0700` HOME; per-identity
host users are unnecessary for beta because the VM is the isolation boundary.

### D.5 Container resource limits

Per-container limits (enforced via Docker `--cpus`, `--memory`,
`--pids-limit`, `--storage-opt`):

| Limit       | Value     | Justification                                                  |
|-------------|-----------|----------------------------------------------------------------|
| CPU         | 2 cores   | A single Rust check + opencode prompt fits in 2; bursts are    |
|             |           | cap-bounded. Hetzner CCX33 has 8 cores -> ~3 customers/host.   |
| Memory      | 4 GB      | Rust LSP + cargo + opencode + Playwright peaks observed at     |
|             |           | ~3 GB in the founder's pipeline; 4 leaves headroom.            |
| Disk        | 50 GB     | Full clone + target/ + caches sits at ~20 GB; 50 leaves room   |
|             |           | for multi-repo customers.                                      |
| pids-limit  | 4096      | Prevents fork-bomb classes; opencode never approaches this.    |
| Network     | bridge    | Per-container bridge; no host-network access. Egress to GitHub |
|             |           | + Anthropic only (egress firewall on the host -- ipset rule).  |

Launch capacity rule: 1 customer = 1 VM. These limits size the runtime inside a
customer VM; they are not a multi-customer density target. Multi-customer host
packing is deferred until after beta load data exists. Section I.1 expands.

### D.6 On-subscription-created flow

```text
Stripe                     api-server                       dc-agent host
  |                            |                                  |
  |--customer.subscription.----->|                                |
  |    created                  |                                  |
  |                            |--insert agent_subscriptions------->|
  |                            |  (status='active')                |
  |                            |                                  |
  |                            |--insert agent_identities--------->|
  |                            |  (state='provisioning',          |
  |                            |   slug=server-generated,         |
  |                            |   github_actor_login=<app bot>)  |
  |                            |                                  |
  |                            |--enqueue async task---------+    |
  |<----200 OK------------------|                            |    |
  |                                                          |    |
  |                                                          v    |
  |                            +-------- async provisioning task -+
  |                            |                                  |
  |                            |--host-control: mkdir HOME-------->|
  |                            |--host-control: write git/opencode|
  |                            |  config; no GitHub token stored  |
  |                            |--host-control: docker run -d ---->>
  |                            |              --name dc-<slug>    |
  |                            |              --restart unless-stopped
  |                            |              -v HOME:/home       |
  |                            |              decent-agents-runtime
  |                            |              sleep infinity      |
  |                            |                                  |
  |                            |<--container_id------------------|
  |                            |                                  |
  |                            |--UPDATE agent_identities         |
  |                            |  SET container_id=...,           |
  |                            |      provisioned_at_ns=now,      |
  |                            |      state='ready' (after        |
  |                            |      health probe passes)        |
```

Acceptance: identity exists in `ready` state within 60 seconds of
`subscription.created` (per #413 acceptance criterion). Realistic
budget: ~25s pulling cached image + ~5s container start + ~10s health
probe + ~10s host setup. Slack: 10s.

Failure handling:
- If GitHub App configuration is missing or malformed: api-server
  refuses to start and `api-server doctor` fails (#414 owns those
  checks). Identity provisioning never begins without valid App config.
- If HOME setup, image pull, container start, or health check fails: retry the
  async provisioning task 3 times with exponential backoff over a maximum of 10
  minutes while the row stays `state='provisioning'`. After the retry budget,
  compare-and-swap to `tearing_down`, stop accepting GitHub events, and alert
  ops with the captured command/error output. Hetzner capacity is the most
  likely cause; alert triggers manual host rotation.

### D.7 On-subscription-deleted flow

```text
Stripe                       api-server                     dc-agent host
  |                             |                                |
  |--customer.subscription.----->|                               |
  |    deleted                  |                                |
  |                             |--UPDATE agent_subscriptions--->|
  |                             |  SET status='canceled',        |
  |                             |      canceled_at_ns=now        |
  |                             |                                |
  |                             |--UPDATE agent_identities------>|
  |                             |  SET state='tearing_down',     |
  |                             |      teardown_started_at_ns=now|
  |                             |    (so new GitHub events       |
  |                             |    are dropped immediately)    |
  |                             |                                |
  |                             |--host-control: docker stop---->|
  |                             |  (graceful, 30s timeout)       |
  |<----200 OK------------------|                                |
  |                                                              |
  ... 7 days pass ...                                            |
  |                                                              |
  |                             +--periodic job: 7-day grace ----+
  |                             |                                |
  |                             |--SELECT * FROM agent_identities
  |                             |  WHERE state='tearing_down'    |
  |                             |    AND now - teardown_started_at_ns >= 7d
  |                             |                                |
  |                             |--host-control: tar HOME -> S3->|
  |                             |--host-control: docker rm------->|
  |                             |--host-control: rm -rf HOME---->|
  |                             |                                |
  |                             |--UPDATE agent_identities       |
  |                             |  SET state='destroyed',        |
  |                             |      destroyed_at_ns=now,      |
  |                             |      archive_location='s3://...'
```

The 7-day grace timer reuses the same periodic-job framework as #410
stale-pending cleanup. **Shared infrastructure dependency**: this spec
and #410 both add a periodic-job table or both consume the same one;
implementation must coordinate. Suggested path if #410 has not landed:
introduce `api/src/database/periodic_jobs.rs` once, used by both. If #410 lands
first, follow its file/module shape instead of forcing this path. Confidence
9/10 -- the founder confirmed in the brief that #410 is in flight.

Teardown failure handling: archive, `docker rm`, and HOME delete are separate
idempotent steps. If any step fails, leave `state='tearing_down'`, increment
`teardown_failure_count`, populate `last_teardown_error`, and retry from the
failed step on the next periodic-job pass. Alert after 3 consecutive failures
or after 14 days in `tearing_down`, whichever comes first. Do not mark
`destroyed` until archive + container removal + HOME deletion have all
succeeded, except for an operator-initiated kill that is audit-logged.

### D.8 Host-control channel: wire protocol

The architecture diagrams in section A and section D.6 reference a
"host-control channel" between the API server and dc-agent on the Hetzner host.
This section pins the wire protocol so the implementation PR cannot drift back
into ad-hoc SSH.

#### D.8.1 Why not SSH

Forbidden. The host-control channel must be auditable and deterministic, not
interactive. SSH carries operational baggage that disqualifies it for this path:

- Persistent operator credentials (host key pinning, authorized_keys rotation,
  agent-forwarding accidents) widen the blast radius beyond what a per-host
  registration token achieves.
- Free-form shell sessions cannot be replayed or audited as discrete actions.
- A flaky network mid-`docker run` leaves no idempotency handle.

dc-agent already speaks an authenticated, polled, idempotent protocol with the
API server (`api/src/openapi/providers.rs:3640-3796` reconcile endpoint;
`dc-agent/src/main.rs:2151-2237` reconcile loop). Reuse it.

#### D.8.2 Wire protocol: extend the bidirectional reconcile loop

Recommendation: extend the existing reconcile mechanism that
`e50ea8e5` (Phase-2 disputes) used to introduce `ReconcilePauseInstance`.
Instead of inventing a parallel command channel, add a new bucket on
`ReconcileResponse` carrying agent-identity provisioning intents:

```rust
// common/src/api_types.rs (sketch -- exact field set finalized at impl time)
pub struct ReconcileProvisionAgentIdentity {
    pub identity_slug: String,           // server-generated, e.g. eager-otter-h7k2p9
    pub home_dir_path: String,           // /home/dc-agent-<slug>
    pub image_tag: String,               // decent-agents-runtime:<commit-sha>
    pub env: Vec<(String, String)>,      // ANTHROPIC_API_KEY mount path, etc.
    pub resource_limits: AgentResourceLimits, // cpus, memory, pids, disk (D.5)
    pub restart_policy: String,          // "unless-stopped"
}

pub struct ReconcileTeardownAgentIdentity {
    pub identity_slug: String,
    pub external_id: String,             // container id once known
    pub archive_uri: Option<String>,     // s3://... target for HOME tarball
}
```

`ReconcileResponse` gains four new buckets, all `#[serde(default)]` so older
dc-agent builds keep parsing:

- `provision_agent_identity` -- carries the `ReconcileProvisionAgentIdentity`
  intents the host must enact (mkdir HOME, write tool config, `docker run -d`).
- `teardown_agent_identity` -- carries `ReconcileTeardownAgentIdentity` intents
  for the 7-day-grace cleanup pass (archive HOME, `docker rm`, `rm -rf` HOME).
- `pause_agents` / `resume_agents` -- already specified in section D.2;
  routed to `AgentRuntimeProvisioner::stop()` / `start()`.

The marketplace-shaped buckets (`keep`, `terminate`, `unknown`, `pause`) stay
unchanged; agent identities never appear there because they have no
`contract_id`.

#### D.8.3 Auth model

The reconcile endpoint already authenticates dc-agent via the
`AgentAuthenticatedUser` extractor (`api/src/auth.rs:175,395-502`):
`X-Agent-Pubkey` + `X-Signature` + `X-Timestamp` + `X-Nonce`, signed with the
agent's private key, validated against an active provider delegation row. This
is the per-host registration token model and is reused as-is. No new
credentials, no SSH keys, no per-identity secrets on the wire.

The API server authorizes agent-identity actions on top of the existing
delegation check by:

- Confirming the calling agent's `provider_pubkey` matches the
  `agent_identities.hetzner_host_id` -> agent registration mapping.
- Refusing intents whose `identity_slug` resolves to an identity owned by a
  different host. This is the same boundary as the marketplace
  "agent can only reconcile their delegated provider's contracts" check at
  `api/src/openapi/providers.rs:3662-3672`.

#### D.8.4 Ordering guarantees

Reconcile is poll-driven. dc-agent calls `POST /providers/:pubkey/reconcile`
every `config.polling.interval_seconds` (`dc-agent/src/main.rs:1378-1379`).
On a single host:

- Action ordering is sequential per poll cycle. dc-agent processes the response
  in a fixed order: `pause_agents` -> `resume_agents` ->
  `provision_agent_identity` -> `teardown_agent_identity` -> marketplace buckets
  (`pause` -> `terminate` -> `keep`). The order matches the reconcile loop's
  existing pause-before-terminate convention (`dc-agent/src/main.rs:2197-2237`).
- Multiple identities in the same bucket are processed serially. There is no
  intra-host parallelism for agent provisioning at v1; one VM per customer
  (section I.1) makes parallel provision intents on a single host the
  exception, not the norm.
- Across poll cycles: actions are idempotent on `identity_slug` + container
  state. A `provision` intent for an identity whose container is already
  running is a no-op; a `teardown` intent for an already-archived identity
  reports success without retrying the archive step.

#### D.8.5 Failure modes and reporting

dc-agent reports outcomes back via the next reconcile call. The reconcile
request body (`ReconcileRequest`) is extended with an
`agent_identity_results` field carrying per-action success/failure with the
captured stdout/stderr from the failing step.

Concrete failure transitions (mirror the contract pattern from `e62ac055`,
which introduced the `failed-provisioning` cleanup state for marketplace
contracts):

- Container start failure (image pull, `docker run`, health probe never passes):
  dc-agent reports `provision_failed` with the captured error in the next
  reconcile call. The API server compare-and-swaps
  `agent_identities.state` from `provisioning` to `provisioning_failed`,
  bumps a retry counter, and re-emits the `provision_agent_identity` intent
  on the following reconcile cycle.
- Periodic retry: up to 3 attempts with exponential backoff over a maximum of
  10 minutes (matching section D.6's failure budget). After exhaustion, the
  state machine compare-and-swaps `provisioning_failed` to `tearing_down`,
  refuses new GitHub events, and alerts ops via the LOUD-misconfig pattern.
  The captured command output is persisted on `agent_identities` (a future
  column `last_provision_error TEXT`) for runbook diagnosis.
- Teardown failure (archive, `docker rm`, or HOME delete): handled in section
  D.7 already; the intent stays on the next reconcile cycle until
  `teardown_failure_count` exceeds the alert threshold.

The state set is therefore extended:

```text
provisioning ---(dc-agent reports failure)---> provisioning_failed
provisioning_failed ---(retry budget exhausted)---> tearing_down
provisioning_failed ---(retry succeeds)---> ready
```

`provisioning_failed` is added to the `agent_identities.state` CHECK list in
section B.2. The cap-driven and dispute-driven pause primitives are unaffected.

#### D.8.6 What the host-control channel does NOT do

- Does NOT carry GitHub installation tokens. Per section F.1, those are minted
  per dispatch and passed via `docker exec` env (#414 owns the dispatch path).
- Does NOT replace the `docker exec` dispatch path for GitHub events. The
  reconcile loop handles container *lifecycle* (provision / pause / resume /
  teardown); per-event invocations use the existing `docker exec` shell-mode
  pattern documented in section A's diagram and section E's routing.
- Does NOT poll for agent runs. `agent_runs` rows are written by the API
  server when a GitHub webhook arrives (section E.1) and updated by the
  container's runner via the #414 refresh-token endpoint.

E. WEBHOOK INTEGRATION WITH #414 (GITHUB APP)
=============================================

### E.1 Routing

```text
GitHub App -> POST /api/v1/webhooks/github
                          |
                          v
         parse payload (delivery_id, installation_id, repo_id,
                        event_type, body)
                          |
                          v
           SELECT ai.id, ai.state, ai.slug
           FROM agent_repos ar
           JOIN agent_subscriptions sub ON sub.id = ar.subscription_id
           JOIN agent_identities ai ON ai.subscription_id = sub.id
           WHERE ar.github_installation_id = $1
             AND ar.github_repo_id = $2
             AND ar.enabled = TRUE
             AND ar.removed_at_ns IS NULL
             AND sub.status IN ('active','trialing')
             AND ai.state = 'ready'
                          |
            +-------------+-------------+
            |                           |
       not found / unlinked          found
            |                           |
            v                           v
   200 OK with          use row.state and row.slug from resolver
   "no identity
    installed"
                                        |
                            +-----------+-----------+
                            |                       |
                       state != ready          state = ready
                            |                       |
                            v                       v
                     200 OK +              mint GitHub App installation
                     reason in body         token (#414), INSERT agent_runs
                                             (status='queued',
                                              github_delivery_id=delivery_id)
                                             |
                                             v
                          api-server -> host channel -> docker exec dc-<slug> \
                                                       -e GITHUB_TOKEN=...
                                                       /usr/local/bin/agent-handle-event \
                                                       --event-json /tmp/<delivery_id>.json
                                             |
                                             v
                          on completion: UPDATE agent_runs SET
                            ended_at_ns, duration_ms,
                            claude_input_tokens, claude_output_tokens,
                            status, result_summary
```

Idempotency: `agent_runs.github_delivery_id UNIQUE`. Replays from
GitHub's manual redelivery (GitHub does NOT auto-retry failed deliveries)
hit the unique constraint and are
no-ops with a 200. The ON CONFLICT handler returns the existing row's
status so the caller knows the event has already been processed. This
matches the founder's pattern at `webhooks.rs:723-739`.

### E.2 `agent_repos` lifecycle

- Insert: `installation_repositories.added` GitHub App webhook event.
- Soft-delete: `installation_repositories.removed` event sets
  `removed_at_ns = NOW()`.
- Hard-delete: NEVER. Historical `agent_runs.repo_id` may reference
  it; analytics/audit need the join.

F. SECRET MANAGEMENT
====================

### F.1 GitHub App credentials

- v1 uses the #414 GitHub App path only. There is no per-identity PAT,
  no per-identity GitHub credential column, and no GitHub credential
  written into the customer's HOME.
- The API server holds `GITHUB_APP_PRIVATE_KEY` and `GITHUB_APP_ID` in
  SOPS-backed shared env. `serve_command()` and `api-server doctor`
  must validate that the key can mint a JWT and call `GET /app` before
  accepting traffic (#414 owns the exact helper).
- For each dispatch, the API server mints a short-lived installation
  token scoped to `github_installation_id`, then passes it to the
  container as `GITHUB_TOKEN`. The `gh` CLI and git HTTPS transport read
  the token from env; the token is not stored in Postgres or on disk.
- Long runs use the #414 refresh endpoint with a per-run nonce. That
  nonce belongs to `agent_runs`, not `agent_identities`, because it is
  event-scoped and must die with the run.

### F.2 Per-identity filesystem secrets

- v1 does not generate SSH keys for GitHub access. GitHub App tokens are
  HTTPS credentials; adding deploy-key or customer SSH-key support is a
  separate post-launch feature with a separate table.
- The HOME may contain tool config (`git`, `gh`, `opencode`) and cached
  repository state, but no long-lived GitHub credential.
- Loss of the HOME means loss of local checkout/cache state only. The
  authoritative source for GitHub access is the installation recorded by
  #414, not a filesystem secret.

### F.3 Anthropic API key

- v1: single `ANTHROPIC_API_KEY` env var on the Hetzner host, mounted
  read-only into every container at `/var/run/anthropic_api_key` (NOT
  in the customer's HOME, so the customer's container processes can
  read it but the customer cannot accidentally export it via a
  `tar HOME`).
- Beta accepts the exfiltration risk for trusted customers only. A follow-up
  GitHub issue tracks replacing the mount with an Anthropic proxy/sidecar that
  injects the key and meters per identity before broader launch.
- Per-customer Claude usage is billed against the platform key;
  margin protection is the cap layer (#415), not key-level isolation.
- v2 BYOK: `agent_identities.anthropic_api_key_sealed` column added
  later, encrypted by the BYOK spec's chosen row-level secret scheme.
  NOT spec'd here.

G. CAP ENFORCEMENT (HANDOFF TO #415)
====================================

This spec deliberately does NOT track caps. #415 owns:
- 20 active agent-hours/month (counted from `agent_runs.duration_ms`
  summed per `(identity_id, billing_period_start_ns)`).
- 3M Claude Sonnet tokens/month (counted from
  `agent_runs.claude_input_tokens + claude_output_tokens` summed
  per `(identity_id, billing_period_start_ns)`).

What this spec MUST guarantee for #415:
1. **Identity is the unit of cap enforcement.** One identity = one
   quota bucket per billing cycle. Implementation already aligns
   (foreign keys from `agent_runs.identity_id`).
2. **`agent_runs` rows feed the cap calculation.** They MUST be
    durable (no UPDATE-with-loss), populated by the time the agent
    exits, idempotent on `github_delivery_id`, and stamped with
    `billing_period_start_ns` at insert time. #415 queries the
    `(identity_id, billing_period_start_ns)` index; it does not perform
    range joins against mutable subscription period columns.
3. **When cap is hit, identity transitions to `paused` with
   `pause_reason='cap_exhausted:<cycle_end_ns>'`.** The pause
   primitive (`agent_identities` state machine, section C.2) already
   handles this; #415 just calls the (yet-to-be-written) Rust helper
   `db.pause_agent_identity(identity_id, reason)`.
4. **Auto-resume on cycle reset.** A periodic job (same framework as
   the 7-day-grace job) scans paused identities with
   `pause_reason LIKE 'cap_exhausted:%'` and resumes them when the
   embedded cycle_end timestamp passes.

H. CLAUDE API KEY STRATEGY
==========================

Decision: **single shared platform Anthropic key** (v1).

Rationale:
- Solo founder posture; per-customer keys add support burden and
  Stripe/Anthropic billing reconciliation work that we cannot pay
  off in v1.
- Cap enforcement (#415) protects margin: if a customer hits 3M
  tokens, their identity pauses, and they pay overage upfront for
  more (Stripe metered billing, separate ticket). Margin formula:
  `49 CHF/mo - 3M*sonnet_4_6_unit - 20h*infra_cost`. Confidence
  6/10 on the per-customer-per-month margin -- founder spreadsheet
  has the actual numbers; this spec only defends the architecture
  decision.
- BYOK is `deferred-post-launch`. Future column:
  `agent_identities.anthropic_api_key_sealed BYTEA NULL`. NULL means
  "use platform key"; non-NULL means "use this one". No code changes
  to the runtime; the bridge selects at startup.
- Security caveat: mounting the platform key into customer containers means a
  malicious or compromised run can read and exfiltrate it. This is not solved by
  #415 caps because a copied key can be used outside Decent Cloud. Beta accepts
  this risk for trusted customers only; public launch needs the proxy/sidecar
  ticket resolved.

I. OPEN DESIGN QUESTIONS / RISKS
================================

Each item: question, recommendation, confidence (1-10).

### I.1 Container per host density

Question: how many identities per Hetzner box?
Decision for beta: **1 customer = 1 VM**. Do not pack multiple customers onto a
single VM for launch. This makes isolation and incident response cleaner while
runtime behavior is still unknown.
Confidence: 9/10 for launch simplicity; cost is higher but accepted for beta.

Math:
- CPU: size each VM for one agent runtime at the D.5 limits plus OS headroom.
- Memory: size each VM for one runtime plus browser/LSP spikes.
- Disk: size each VM for one customer's repos, build cache, image layers, and OS.
- Network: 1 Gbps shared; per-customer ingress is GitHub webhooks
  (negligible) and egress is `git pull` + Anthropic API calls.

Cost: beta accepts one-VM-per-customer infra cost. Revisit multi-customer packing
only after real runtime telemetry and customer demand justify the complexity.

### I.2 Container image build pipeline

Question: separate CI job or piggyback on existing dc-agent build?
Recommendation: **separate CI job**, triggered on changes to
`agent/Dockerfile` and the lockfiles (Cargo.lock, package-lock.json,
bun.lockb). Tag images `decent-agents-runtime:<commit-sha>` and push to
ghcr.io/decent-stuff/decent-agents-runtime. The Hetzner hosts pull from
ghcr.
Confidence: 9/10. The founder's internal pipeline already builds an
agent image; productizing means adding a `.github/workflows/agent-runtime.yml`
that builds + pushes on the same triggers. Net new work: ~50 lines of YAML.

### I.3 GitHub App private key handling

Question: where does the credential that can mint installation tokens live?
Recommendation: **api-server env only, backed by SOPS**, exactly as #414
specifies.
Confidence: 9/10.

Trade-offs:
- SOPS shared env: accepted. One App private key, one server-side
  validator, one rotation path.
- Postgres storage: rejected. The App private key is not row-shaped
  identity data and should never be queryable from the database.
- Per-container storage: rejected. Containers receive installation
  tokens only, never the App private key.

### I.4 Identity slug collisions

Question: what if two customers pick the same slug?
Recommendation: **server-generated slugs only**. Format:
`<adjective>-<noun>-<6-char-hex>`, e.g. `eager-otter-h7k2p9`. Adjective
+ noun pulled from a curated wordlist (~500 each = 250k base
combinations); 6-char hex suffix (16M) gives ~4e12 unique slugs. Never
user-chosen.
Confidence: 10/10. The DB UNIQUE constraint on `slug` is the
final-line defence; the suffix makes practical collision impossible
(birthday bound at 50% collision: ~2M slugs).

### I.5 What happens if Hetzner host dies?

Question: DR strategy -- replicate, or accept downtime + re-provision?
Recommendation: **accept downtime + re-provision**. Solo-founder posture;
SLA at launch is best-effort, NOT 99.9%.
Confidence: 8/10.

DR plan:
- Hetzner snapshots daily (`/home/dc-agent-*` backed up via
  Hetzner's snapshot API). Restore RTO ~30 min.
- If a host is gone for >30 min, customers see "agent paused, restoring"
  on a status page (post-launch); for v1, ops paging is the signal.
- Multi-host replication with active-active is the v2 ask. NOT in
  this spec.
- Post-mortem: the 90-day archive policy (section I.8) means even a
  catastrophic loss is recoverable to within 24h of customer state.

### I.6 GitHub App vs PAT

Question: should v1 support PAT as an alternate path?
Recommendation: **no**. #414 chooses GitHub App with 1-hour
installation tokens. This spec is intentionally App-only so identity
provisioning does not need row-level GitHub secret storage, token
expiry polling, or per-customer PAT support.
Confidence: 9/10. A future PAT/BYOK-style option would add a
separate credential table and auth-mode enum; do not overload
`agent_identities`.

### I.7 Container restart on host reboot

Question: existing dc-agent restart behavior -- verify and document.
Recommendation: **rely on Docker's `--restart unless-stopped` flag**.
Already in use at `dc-agent/src/provisioner/docker.rs:208-211` for
marketplace VMs; same flag for agent containers. On Hetzner host
reboot, dockerd starts on boot, all containers come back up.
Confidence: 9/10.

Caveat: a paused container (`docker stop` from the cap or dispute
path) is NOT restarted by `unless-stopped` -- which is correct,
because a cap-paused container should stay down until the cycle
resets. The `agent_identities.state` field is the source of truth;
on host boot a reconcile pass compares Docker container state with
the DB state and either starts (state=ready) or leaves down
(state=paused, tearing_down, destroyed).

### I.8 Backups

Question: does `agent_identities.home_dir_path` need backup before destroy?
Decision: **GDPR compliance is the hard requirement; no separate retention
promise for beta**. Keep the shortest retention that still supports operational
recovery, document it in customer terms, and delete on verified customer erasure
requests. Do not promise 90 days unless legal/product explicitly chooses it.

### I.9 Decisions log (resolved)

| # | Decision | Choice | Rationale |
|---|----------|--------|-----------|
| 1 | Anthropic key isolation | Accept shared-key mount for beta; file ticket for proxy/sidecar before public launch | Beta customers are trusted; exfiltration risk accepted. Public launch requires per-identity key proxy. |
| 2 | Host-control channel | Extend the existing reconcile loop with `provision_agent_identity`, `teardown_agent_identity`, `pause_agents`, `resume_agents` buckets (section D.8). Not SSH. | Reuses dc-agent's authenticated, polled, idempotent reconcile path (`api/src/openapi/providers.rs:3640-3796`, `dc-agent/src/main.rs:2151-2237`). Avoids SSH key management, pinning, rotation; auditable per intent. |
| 3 | Async job durability | DB-backed job rows consumed by a worker | API server restarts do not strand `provisioning` rows; repair scan is not needed. |
| 4 | GitHub webhook loss during deploys | Accept loss for beta; add periodic delivery-log scan for missed events | GitHub does not automatically retry failed webhook deliveries. A periodic job queries GitHub App delivery logs and replays missed deliveries. |
| 5 | Host registry and capacity | Single-host config for beta; no `agent_hosts` table yet | 1 customer = 1 VM makes host registry unnecessary at beta scale. Add when multi-customer packing is real. |
| 6 | Filesystem isolation | One runtime user per VM (no per-identity host users) | 1 customer = 1 VM means no cross-customer host filesystem access. Per-identity users add ops cost for zero isolation gain at this scale. |
| 7 | Archive retention | GDPR compliance is the hard requirement; no separate retention promise | Keep the shortest operational retention, delete on verified erasure requests, document in customer terms. |
| 8 | Shared reconcile API shape | Explicit `ReconcilePauseAgentIdentity` / `ReconcileResumeAgentIdentity` with `pause_agents` / `resume_agents` buckets | Marketplace pause requires `contract_id`; agent identities have none. Separate types keep the domains clean. |
| 9 | Dispute while provisioning | No dispute pause for beta; any dispute tears down | Disputes are not a beta objective. Any DA dispute event triggers `tearing_down` regardless of current state. Simplest correct code. |

### I.10 Escalations requiring founder acknowledgement

These are not implementation gaps; they are explicit risk acceptances needed
before beta or public launch:

1. **Prompt-injection key exfiltration:** beta mounts the shared Anthropic key
   into customer containers. A malicious GitHub issue can instruct the agent to
   read and leak it. Founder must accept this for trusted beta or move the
   proxy/sidecar into v1.
2. **Dispute handling:** any Decent Agents dispute tears the identity down rather
   than pausing it. Founder must accept loss of warm workspace/cache state on
   disputed accounts.
3. **Cancel/re-subscribe UX:** re-subscribing creates a new identity even inside
   the 7-day grace window. Founder must accept this or add revival semantics.
4. **Unit economics:** beta uses one VM per customer. Founder spreadsheet must
   verify CHF 49/month covers VM + shared Anthropic usage before opening beyond
   trusted beta.

J. ACCEPTANCE CHECKLIST
=======================

Implementation maps each box to owner files or measurable evidence. NOT
implemented in this spec; this is the contract for the future PR.

| # | Acceptance criterion                                                       | File:Line                                                    |
|---|----------------------------------------------------------------------------|--------------------------------------------------------------|
| 1 | Migration creates 4 new tables (subscriptions, identities, runs, repos)    | `api/migrations_pg/046_agent_identities.sql:1-220`           |
| 2 | DB module `agent_subscriptions.rs` with insert/get/update + idempotent     | `api/src/database/agent_subscriptions.rs:1-150`              |
|   | upsert on stripe_subscription_id                                            |                                                              |
| 3 | DB module `agent_identities.rs` with state-machine helpers (provision,     | `api/src/database/agent_identities.rs:1-300`                 |
|   | mark_ready, pause, resume, tear_down, destroy) -- pause/resume mirror     |                                                              |
|   | the dispute primitives in `contracts/dispute.rs`                            |                                                              |
| 4 | DB module `agent_runs.rs` with idempotent insert on github_delivery_id,    | `api/src/database/agent_runs.rs:1-140`                       |
|   | billing_period_start_ns stamping, and end_run helper                       |                                                              |
| 5 | DB module `agent_repos.rs` with add_repo / link_subscription /             | `api/src/database/agent_repos.rs:1-140`                      |
|   | soft_delete_repo / resolve_active_repo                                      |                                                              |
| 6 | Stripe webhook arms route DA subscription events through new DA branch     | `api/src/openapi/webhooks.rs:465-692` (edit)                 |
|   | keyed by `DECENT_AGENTS_STRIPE_PRICE_ID`; event and DB-backed job are      |                                                              |
|   | durable before 2xx; Pro subscription path stays separate                  |                                                              |
| 7 | #413 exposes identity/repo resolver used by #414 `/api/v1/webhooks/github` | `api/src/database/agent_repos.rs:1-140`; #414 integration   |
| 8 | dc-agent provisioner `agent_container.rs` with provision/stop/start/      | `dc-agent/src/provisioner/agent_container.rs:1-400`          |
|   | terminate; reuses `docker_common.rs` helpers                               |                                                              |
| 9 | Periodic job for 7-day grace + cap-paused resume (coordinate with #410)    | shared periodic-job module; path follows #410 if it lands first |
|10 | #413 stores no GitHub token; #414 mint/cache helper supplies dispatch token | migration grep + `agent_repos` schema; #414 integration      |
|11 | Slug generator (wordlist + suffix)                                         | `api/src/decent_agents/slug.rs:1-60`                         |
|12 | Runtime image source ported into product repo + CI workflow                | `agent/Dockerfile:1-140`; `.github/workflows/decent-agents-runtime.yml:1-50` |
|13 | Common reconcile API adds explicit agent lifecycle actions per section    | `common/src/api_types.rs`: `ReconcilePauseAgentIdentity`, `ReconcileResumeAgentIdentity`, |
|   | D.8: pause/resume + provision/teardown buckets; dc-agent loop processes   | `ReconcileProvisionAgentIdentity`, `ReconcileTeardownAgentIdentity`; `ReconcileResponse.{pause_agents,resume_agents,provision_agent_identity,teardown_agent_identity}` (all `#[serde(default)]`) |
|   | each bucket serially (D.8.4); failure is reported via the next reconcile  | `dc-agent/src/main.rs`: reconcile loop branches; `ReconcileRequest.agent_identity_results` for failure reporting |
|   | call (D.8.5)                                                              |                                                              |
|14 | Tests: idempotent subscription insert, state-machine reject of bad         | `api/src/database/agent_subscriptions.rs:tests:*`            |
|   | transitions, pause/resume credit, slug uniqueness, no persisted GitHub     | `api/src/database/agent_identities.rs:tests:*`               |
|   | credentials, webhook idempotency on replay                                  | `api/src/openapi/webhooks.rs:tests:*`                        |

K. EFFORT ESTIMATE
==================

Honest, broken down by section. Sessions = ~3-hour focused agent
sessions in the founder's pipeline (matches the Phase-2 Stripe
disputes shape).

| Section                                                       | Est. sessions | Wall clock | Notes                                                |
|---------------------------------------------------------------|---------------|------------|------------------------------------------------------|
| B (migration + 4 DB modules + tests)                          | 3             | ~9h        | New tables straightforward; tests are the long pole. |
| C (state machine wiring -- helpers in agent_identities.rs)    | 2             | ~6h        | Mirrors dispute primitives; copy-and-adapt.          |
| D (provisioner)                                               | 3             | ~9h        | docker_common.rs extraction is the risk.             |
| E (webhook routing + agent_repos)                             | 2             | ~6h        | Lots of glue; idempotency tests.                     |
| F (GitHub App token handoff + key validation coordination)     | 1             | ~3h        | Mostly #414 helper reuse; no new DB secret storage.  |
| Periodic job framework (shared with #410)                     | 1             | ~3h        | Coordinate with #410 implementer.                    |
| Container image build pipeline                                | 1             | ~3h        | YAML + Dockerfile diff.                              |
| Integration test (provision -> run -> destroy on staging)     | 1             | ~3h        | Real Hetzner host (cheap CCX22; clean up same day).  |
| **Total in-codebase**                                         | **14**        | **~42h**   |                                                      |

External / wait-time:
- Stripe Connect or test-mode product setup for the CHF 49 SKU: ~1h
  manual founder work, but unblocks before any code merges.
- Hetzner image build + register host for Decent Agents: ~2h once.
- DNS for any future status page: ~30 min.
- ghcr.io setup for the runtime image: ~1h (likely already done).
- GitHub App private-key secrets vault entry: 15 min (owned by #414).

External wait-time total: ~5h, cleanly off the critical path.

Cumulative: implementation ready in ~14 sessions / 2 working weeks
of focused agent time. NOT including #414 (GitHub App) or #415
(billing) which are separate streams.

## Confidence summary

| Section | Confidence | Largest risk                                                                 |
|---------|------------|------------------------------------------------------------------------------|
| A       | 9/10       | Concept naming may shift if "subscription" is overloaded by the website tier |
| B       | 8/10       | Column choices are stable; migration ordering vs #410 may collide            |
| C       | 9/10       | Pause/resume semantics already battle-tested via Phase-2 disputes            |
| D       | 8/10       | 1-customer-per-VM simplifies provisioning; runtime image stability unknown    |
| E       | 8/10       | Idempotency is straightforward; webhook signature handling reuse needs care |
| F       | 9/10       | App-key rotation is centralized; #414 helper must be reused consistently    |
| G       | 9/10       | Handoff is clean; #415 inherits this schema                                  |
| H       | 6/10       | Margin per customer not modelled in this spec                                |
| I       | 7/10       | DR / multi-host posture pushes the most v2 work                              |

## Final notes

- This spec adds NO product code. All file paths in the acceptance
  checklist are FUTURE locations.
- The migration should land before or together with #414 / #415. If #414
  lands first, the #414 migration creates its tables without the
  `agent_runs.github_delivery_id` FK and a follow-up migration adds the
  FK after this spec's tables exist.
- The 7-day grace timer is the riskiest single piece because it
  shares plumbing with #410. Coordinate sequencing.
- Decision: `agent_subscriptions.account_id` carries `accounts.id`
  (BYTEA). Do not use username strings for billing joins.
- `agent_runs.run_secret_hash` is the SHA-256 of a 32-byte cryptographic random
  used by the #414 mid-run token refresh endpoint. The plaintext secret is
  passed to the container once via env and never persisted; the refresh-token
  handler hashes the supplied secret and compares with `subtle::ConstantTimeEq`
  against the stored hash. Cleared on terminal state. See #414 section F.
