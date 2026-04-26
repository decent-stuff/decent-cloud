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
   GitHub repo: webhook from #414 GitHub App lookup -> identity
   POST /api/v1/decent-agents/dispatch       (exposes identity_id)
                       |
                       v
          api-server -> ssh-into Hetzner host -> docker exec <slug> ...
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
    stripe_customer_id       TEXT         NOT NULL,
    -- Normalized billing status only. Compute pauses live in
    -- agent_identities.state; do NOT write cap/dispute pauses here.
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

CREATE UNIQUE INDEX idx_agent_subscriptions_stripe_sub
    ON agent_subscriptions (stripe_subscription_id);
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
    -- Lifecycle state -- see section C.2.
    state                    TEXT         NOT NULL CHECK (state IN
                                            ('provisioning','ready','paused',
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
    created_at_ns            BIGINT       NOT NULL,
    updated_at_ns            BIGINT       NOT NULL
);

CREATE UNIQUE INDEX idx_agent_identities_subscription
    ON agent_identities (subscription_id);
CREATE UNIQUE INDEX idx_agent_identities_slug
    ON agent_identities (slug);
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
CREATE INDEX idx_agent_runs_status
    ON agent_runs (status)
    WHERE status IN ('queued','running');
CREATE UNIQUE INDEX idx_agent_runs_event
    ON agent_runs (github_delivery_id);
```

If #414 lands its `github_webhook_deliveries` table first, add a
foreign key from `agent_runs.github_delivery_id` to
`github_webhook_deliveries.github_delivery_id` in the same migration.
If #413 lands first, #414 adds that FK in its migration.

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
    -- Nullable until the GitHub installation is linked to a Decent Cloud
    -- account with an active agent_subscriptions row. Unlinked installs are
    -- persisted for audit but never dispatched.
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
| any              | canceled | `customer.subscription.deleted`, `unpaid`, `incomplete_expired`, OR dispute lost          |

Edge cases:
- `past_due` and identity `paused` may coexist. Billing recovery moves
  subscription status back to `active`; compute resumes only when the
  corresponding dispute/cap pause reason is cleared.
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
                                      |             \  resume (dispute won
                                      |              \  or cap window reset)
                                      | pause         \
                          (dispute or cap exhaustion)  \
                                      v                 |
                                +--------+              |
                                | paused |--------------+
                                +----+---+
                                     |
                  subscription.deleted | dispute lost | hard-fail
                                     v
                              +-------------+
                              | tearing_down|  (stop container, archive HOME,
                              +------+------+   stop accepting GitHub events)
                                     |
                          7-day grace job fires
                                     v
                              +-----------+
                              | destroyed |  (container removed, HOME archived
                              +-----------+   to S3 with 90-day retention,
                                              row stays for audit)
```

Transitions and triggers:

| From          | To            | Trigger                                                              |
|---------------|---------------|----------------------------------------------------------------------|
| (insert)      | provisioning  | async provisioning task started after `subscription.created`         |
| provisioning  | ready         | dc-agent reports container running AND first health check passes     |
| provisioning  | tearing_down  | provisioning failed permanently (after retry budget; alert ops)      |
| ready         | paused        | dispute pause OR cap exhaustion -- writes `pause_reason`             |
| paused        | ready         | dispute resumed OR cap window rolled over                            |
| ready, paused | tearing_down  | `subscription.deleted` OR `charge.dispute.closed` lost               |
| tearing_down  | destroyed     | grace timer (7 days from `teardown_started_at_ns`) fires periodic job |
| any           | destroyed     | operator-initiated kill (admin SQL only, audit-logged)               |

Edge cases:
- `provisioning -> tearing_down` skips `ready` because the container
  never came up. Section D defines retries before we give up.
- `paused` (cap exhaustion) MUST distinguish `pause_reason` from
  `paused` (dispute) so the resume path knows which side to consult.
  Reasons used: `stripe_dispute:<id>`, `cap_exhausted:<cycle_end_ns>`,
  `operator:<memo>`. The `agent_identities.pause_reason` column is
  TEXT to keep this open-ended.
- Identity `paused` does NOT pause the Stripe subscription -- billing keeps
  running. The customer is still paying for the cycle; pause just
  stops compute. This matches the dispute Phase-2 pattern.
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

### D.2 ReconcilePauseInstance bucket reuse

The `ReconcilePauseInstance` bucket (added in commit `e50ea8e5`,
`common/src/api_types.rs`) IS reused. Adds two pause reason codes the
agent provisioner recognizes:
- `stripe_dispute:<id>` -- Phase-2 path, already wired.
- `cap_exhausted:<cycle_end_ns>` -- new code path; the periodic
  cap-renewal job (#415) clears this when the cycle rolls over by
  emitting a `ReconcileResumeInstance` bucket entry.

Both reasons route to the same `Provisioner::stop()` call; the
`agent_container` provisioner's `stop()` honors the
"do-not-destroy" semantic (Docker container `pause` or `stop`
without `--rm`, NOT `terminate`). This matches the trait's intent
documented at `dc-agent/src/provisioner/mod.rs:107-124`.

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
`agent/docker-compose.yml:21`). On the Hetzner host the directory is
owned by a per-identity uid (`dc-agent-<slug>`) so a kernel exploit
inside one container cannot read or mutate a sibling's workspaces.
Confidence: 7/10 -- per-identity uids on the host adds ops cost; for
v1 a single shared `dc-agent` user with chmod 0700 dirs may be
acceptable. Decision deferred to implementation; mark as a security
review checkpoint.

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

These limits combined: 1 Hetzner CCX43 (16 cores / 64 GB / 600 GB
NVMe) supports ~10 concurrent identities at full burst, comfortably
~15-20 at typical 30% utilisation. Section I.1 expands.

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
  |                            |--SSH to host: mkdir HOME--------->|
  |                            |--SSH to host: write git/opencode |
  |                            |  config; no GitHub token stored  |
  |                            |--SSH to host: docker run -d ----->>
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
- If container start fails: same retry budget, same alert. Hetzner
  capacity is the most likely cause; alert triggers manual host
  rotation.

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
  |                             |--SSH host: docker stop-------->|
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
  |                             |--SSH host: tar HOME -> S3----->|
  |                             |--SSH host: docker rm----------->|
  |                             |--SSH host: rm -rf HOME-------->|
  |                             |                                |
  |                             |--UPDATE agent_identities       |
  |                             |  SET state='destroyed',        |
  |                             |      destroyed_at_ns=now,      |
  |                             |      archive_location='s3://...'
```

The 7-day grace timer reuses the same periodic-job framework as #410
stale-pending cleanup. **Shared infrastructure dependency**: this spec
and #410 both add a periodic-job table or both consume the same one;
implementation must coordinate (suggest: introduce
`api/src/database/periodic_jobs.rs` once, used by both). Confidence
9/10 -- the founder confirmed in the brief that #410 is in flight.

E. WEBHOOK INTEGRATION WITH #414 (GITHUB APP)
=============================================

### E.1 Routing

```text
GitHub App -> POST /api/v1/decent-agents/github-events
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
                          api-server -> SSH host -> docker exec dc-<slug> \
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
GitHub (which retries on non-2xx) hit the unique constraint and are
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
- Per-customer Claude usage is billed against the platform key;
  margin protection is the cap layer (#415), not key-level isolation.
- v2 BYOK: `agent_identities.anthropic_api_key_sealed` column added
  later, encrypted by the BYOK spec's chosen row-level secret scheme.
  NOT spec'd here.

G. CAP ENFORCEMENT (HANDOFF TO #415)
====================================

This spec deliberately does NOT track caps. #415 owns:
- 20 active agent-hours/month (counted from `agent_runs.duration_ms`
  summed per `(identity_id, billing_cycle)`).
- 3M Claude Sonnet tokens/month (counted from
  `agent_runs.claude_input_tokens + claude_output_tokens` summed
  per `(identity_id, billing_cycle)`).

What this spec MUST guarantee for #415:
1. **Identity is the unit of cap enforcement.** One identity = one
   quota bucket per billing cycle. Implementation already aligns
   (foreign keys from `agent_runs.identity_id`).
2. **`agent_runs` rows feed the cap calculation.** They MUST be
   durable (no UPDATE-with-loss), populated by the time the agent
   exits, and idempotent on `github_delivery_id`. Already specified
   above.
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

I. OPEN DESIGN QUESTIONS / RISKS
================================

Each item: question, recommendation, confidence (1-10).

### I.1 Container per host density

Question: how many identities per Hetzner box?
Recommendation: target 10 active identities per CCX43 (16 cores / 64 GB /
600 GB NVMe). Hard cap 15 to leave headroom for caches and bursts.
Confidence: 7/10 (real number depends on customer mix; founder has
not yet load-tested the runtime image at scale).

Math:
- CPU: 10 x 2-core limit = 20 vCPU vs 16 physical -> oversubscription
  ratio 1.25x. Acceptable because typical agent activity is bursty
  (cargo check ~80s, then idle); average utilization < 30%.
- Memory: 10 x 4 GB = 40 GB vs 64 GB -> 24 GB free for OS + caches.
- Disk: 10 x 50 GB = 500 GB vs 600 GB -> 100 GB free for image
  layers + system.
- Network: 1 Gbps shared; per-customer ingress is GitHub webhooks
  (negligible) and egress is `git pull` + Anthropic API calls.

Cost: Hetzner CCX43 ~70 EUR/month -> ~7 EUR per customer in
infra. CHF 49 - 7 = ~CHF 42 gross; minus Anthropic 3M-token
allowance (variable) and Stripe fees (~3%).

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

Question: should v1 keep a PAT fallback?
Recommendation: **no**. #414 chooses GitHub App with 15-minute
installation tokens. This spec is intentionally App-only so identity
provisioning does not need row-level GitHub secret storage, token
expiry polling, or per-customer PAT support.
Confidence: 9/10. A future PAT/BYOK-style escape hatch would add a
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
Recommendation: **yes**. Tar the HOME at `tearing_down -> destroyed`
transition, push to S3 at
`s3://decent-stuff-agent-archive/<slug>-<destroyed_at_ns>.tar.gz`,
populate `agent_identities.archive_location`. Retention: 90 days
(matches GDPR-style data-minimization; long enough for "I cancelled
by accident" recovery, short enough to not become a liability).
Confidence: 7/10 (90 days is a guess; legal review needed before
launch).

J. ACCEPTANCE CHECKLIST
=======================

Implementation maps each box to exactly one file. NOT implemented in
this spec; this is the contract for the future PR.

| # | Acceptance criterion                                                       | File:Line                                                    |
|---|----------------------------------------------------------------------------|--------------------------------------------------------------|
| 1 | Migration creates 4 new tables (subscriptions, identities, runs, repos)    | `api/migrations_pg/046_agent_identities.sql:1-220`           |
| 2 | DB module `agent_subscriptions.rs` with insert/get/update + idempotent     | `api/src/database/agent_subscriptions.rs:1-150`              |
|   | upsert on stripe_subscription_id                                            |                                                              |
| 3 | DB module `agent_identities.rs` with state-machine helpers (provision,     | `api/src/database/agent_identities.rs:1-300`                 |
|   | mark_ready, pause, resume, tear_down, destroy) -- pause/resume mirror     |                                                              |
|   | the dispute primitives in `contracts/dispute.rs`                            |                                                              |
| 4 | DB module `agent_runs.rs` with idempotent insert on github_delivery_id +   | `api/src/database/agent_runs.rs:1-140`                       |
|   | end_run helper (tokens, status, summary)                                   |                                                              |
| 5 | DB module `agent_repos.rs` with add_repo / link_subscription /             | `api/src/database/agent_repos.rs:1-140`                      |
|   | soft_delete_repo / resolve_active_repo                                      |                                                              |
| 6 | Stripe webhook arms route DA subscription events through new code path     | `api/src/openapi/webhooks.rs:465-692` (edit)                 |
|   | (price_id check distinguishes DA-tier subscriptions from existing Pro)     |                                                              |
| 7 | New endpoint POST /api/v1/decent-agents/github-events                      | `api/src/openapi/decent_agents.rs:1-150` (NEW)               |
| 8 | dc-agent provisioner `agent_container.rs` with provision/stop/start/      | `dc-agent/src/provisioner/agent_container.rs:1-400`          |
|   | terminate; reuses `docker_common.rs` helpers                               |                                                              |
| 9 | Periodic job for 7-day grace + cap-paused resume (shared with #410)        | `api/src/database/periodic_jobs.rs:1-200` (shared)           |
|10 | GitHub App token handoff uses #414 mint/cache helper; no persisted PAT     | `api/src/openapi/decent_agents.rs:1-180`                     |
|11 | Slug generator (wordlist + suffix)                                         | `api/src/decent_agents/slug.rs:1-60`                         |
|12 | Runtime image source ported into product repo + CI workflow                | `agent/Dockerfile:1-140`; `.github/workflows/decent-agents-runtime.yml:1-50` |
|13 | Tests: idempotent subscription insert, state-machine reject of bad         | `api/src/database/agent_subscriptions.rs:tests:*`            |
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
| D       | 7/10       | Container density depends on real customer mix; load-test before launch     |
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
