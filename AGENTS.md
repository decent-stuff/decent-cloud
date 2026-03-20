# PROJECT KNOWLEDGE BASE

**Generated:** 2026-03-20 Europe/Zurich
**Scope:** `repo/` submodule
**Mirror:** keep `AGENTS.md` and `CLAUDE.md` byte-for-byte aligned

## OVERVIEW
`repo/` is the real product root: Rust workspace (`api`, `cli`, `common`, `dc-agent`, `ic-canister`, `ledger-map`), SvelteKit frontend (`website`), Python tooling, CI, scripts, and vendored third-party source for integration debugging.

## OPERATING POSTURE
- Build the smallest real proof-of-concept first, prove it works end-to-end, then write tests and production code. You are beyond guessing — you use tools to build a standalone working PoC and only then plan architecture, tests, and production code.
- Read the codebase deeply before changing it; follow existing patterns unless they are clearly harmful.
- If you are NOT confident 9/10 or 10/10 that you can ship production-ready code in one pass, **STOP AND SAY SO**. Ensure the user is aware. Suggest ways to improve the odds.
- Prefer elegant simplification over additive complexity; remove duplication instead of working around it.
- Be always brutally honest and objective.

## STRUCTURE
```text
repo/
|- api/                    # central API server + admin/test CLI + DB layer
|- cli/                    # user-facing CLI
|- common/                 # shared Rust business primitives
|- dc-agent/               # provider-side agent runtime
|- ic-canister/            # Internet Computer canister
|- ledger-map/             # storage library, also published externally
|- website/                # SvelteKit frontend + Playwright/Vitest
|- tools/provider-scraper/ # Python crawler for provider catalogs/docs
|- scripts/                # dev/test/browser helpers
|- cf/                     # deployment stack + envs
`- third_party/            # vendored external source for debugging only
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| API boot, doctor, env validation | `api/src/main.rs` | `Serve`, `Doctor`, `SyncDocs`, `Setup` |
| API endpoints | `api/src/openapi/` | Domain-grouped handlers |
| API persistence | `api/src/database/` | Large SQLx layer; see child `AGENTS.md` |
| Provider agent runtime | `dc-agent/src/` | Polling loop, provisioning, gateway, setup |
| Shared Rust domain types | `common/src/` | Cross-cutting business primitives |
| Frontend fetch layer | `website/src/lib/services/api.ts` | Main TS API client |
| Frontend UI flows | `website/src/routes/` | Standard SvelteKit routes/layouts |
| Browser automation helpers | `scripts/` | See child `scripts/AGENTS.md` for browser and auth tooling |
| Provider scrape tooling | `tools/provider-scraper/` | Python project with its own tests |
| Third-party integration references | `third_party/` | Debug contracts, APIs, payloads |

## MANDATORY WORKFLOW (NON-NEGOTIABLE)

Every task follows this exact sequence. No exceptions. You may NEVER deviate from this workflow. If prerequisites are missing or you are NOT 90+% confident the code will be ready for production — STOP and ask for acknowledgement.

### 1. Verify Prerequisites
- Confirm real services, credentials, env vars, and infrastructure exist before coding. YOU MUST HAVE everything you need to do your job properly.
- Check credentials: run `scripts/dc-secrets list` and `scripts/dc-secrets export` to verify secrets are available.
- **If anything is missing: STOP immediately and ask.** Do not guess, stub, or silently mock what should be real.

### 2. Build a Working PoC (NOT SKIPPABLE)
- Build the smallest thing that proves the feature works end-to-end against **real services**.
- Start the API server locally if needed (`cargo build -p api --bin api-server && ./target/debug/api-server serve`).
- Use real API endpoints, real databases, real external services.
- Exercise the full path: input -> processing -> storage -> output.
- Fix blocking bugs discovered during the PoC, including pre-existing ones that prevent validation.

### 3. Prove It Works
- Run the PoC and **show evidence** that it works: execute CLI commands, HTTP requests, or UI interactions against the running system.
- Verify the output matches expectations.
- Test the happy path AND at least one error path.
- Clean up test data created during validation.

### 4. Write Failing Tests (NOT SKIPPABLE)
- Now that you know the feature works, write tests that codify the behavior.
- Write tests BEFORE refactoring or cleaning up the PoC code.
- Tests must fail without your changes (verify this if possible).
- Cover positive and negative paths.
- Avoid overlapping existing coverage.

### 5. Re-evaluate Confidence
- Based on the codebase analysis and your PoC, if implementation confidence is below 9/10, **STOP and ask** for guidance before continuing.

### 6. Write Production Code
- Clean the PoC into minimal production code.
- Follow all project rules (DRY, YAGNI, fail-fast, etc.).
- Ensure clippy/tests pass in the changed project.

### 7. Close the Loop
- Report realistic completion state (e.g. "80% done").
- Add still-required work to `TODO.md` and remove completed items to reduce noise.

## LOCAL DEVELOPMENT
### Default Rule
- Use local services, not `dev.decent-cloud.org`, for AI-agent development. Local stacks are under your control; staging is not.

### PostgreSQL
```bash
docker exec agent-postgres-1 pg_isready -U test
```
- Inside the containerized local workflow, PostgreSQL is reachable via hostname `postgres`.
- The compose-level local stack currently provides PostgreSQL only; Chatwoot is not part of `agent/docker-compose.yml`.

### Running The API Server Locally
```bash
cargo build -p api --bin api-server
eval "$(scripts/dc-secrets export)"
./target/debug/api-server serve

DATABASE_URL=postgres://test:test@postgres:5432/test \
CREDENTIAL_ENCRYPTION_KEY="$(openssl rand -hex 32)" \
CANISTER_ID="ggi4a-wyaaa-aaaai-actqq-cai" \
API_SERVER_PORT=59011 \
./target/debug/api-server serve

curl http://localhost:59011/api/v1/health
```

### Running The Website Locally
```bash
cd website
npm run dev
```
- Website defaults to API at `localhost:59011` unless overridden.

### Credentials (dc-secrets)
All secrets are stored in SOPS-encrypted files under `secrets/`. Use `scripts/dc-secrets` to manage them:
- `scripts/dc-secrets export` - print all credentials as key=value (used by entrypoint.sh automatically)
- `scripts/dc-secrets set shared/env KEY=value` - add/update a credential
- `scripts/dc-secrets edit shared/env` - interactive edit in $EDITOR
- `scripts/dc-secrets list` - list all secret files

### Seeding Test Data
```bash
DC_WEB_URL=http://localhost:5173 DC_API_URL=http://localhost:59011 \
node scripts/dc-auth.js seed-ux-data

DC_WEB_URL=http://localhost:5173 DC_API_URL=http://localhost:59011 \
node scripts/dc-auth.js seed-contracts
```
- `seed-ux-data` starts a heartbeat daemon to keep the provider online; stop it with `kill $(cat /tmp/dc-keepalive-*.pid)`.

## PROJECT RULES
- **MINIMIZE CLOUD SPENDING**: When testing against paid cloud providers (Hetzner, AWS, etc.), ALWAYS use the cheapest possible server type (e.g., `cx22` on Hetzner), ALWAYS delete resources immediately after verification, and NEVER leave VMs running unattended. Every test VM must be cleaned up in the same session it was created.
- Adjust and extend existing code instead of creating parallel implementations. Before you start coding, PLAN how existing code can be adjusted in the most concise way.
- New code must stay minimal, DRY, YAGNI, KISS, and fail fast. Code must be idiomatic (e.g. use `match`).
- NEVER silently ignore failures or return results. Avoid patterns like `let _ = ...`. In case of error, provide failure details (e.g. with `"{:#?}"`) for troubleshooting.
- **BE LOUD ABOUT MISCONFIGURATIONS**: When optional features are disabled due to missing config, always log a clear warning. Use `tracing::warn!` with actionable messages like "X not set — Y will NOT work! Set X to enable." Never silently skip functionality.
- Every function and execution path needs meaningful tests; positive and negative paths both matter. Tests MUST ASSERT MEANINGFUL BEHAVIOR and MAY NOT overlap coverage with other tests.
- Prefer crate-local `cargo clippy --tests` and `cargo nextest run` for the area you changed. Fix warnings and errors before moving on.
- If you fix one instance of a bug pattern, check the rest of the codebase for the same issue and FIX ALL INSTANCES.
- Keep user-facing work actually usable: update UI pages, menus, sidebars, and CLI surfaces when the feature requires it.
- ALWAYS REMOVE ALL DUPLICATION AND COMPLEXITY. No backward-compatibility excuses. This is a monorepo — change all that's needed to end up with clean code and clean architecture.

## BROWSER TESTING
Use `scripts/browser.js` directly from Bash — it works in ALL contexts (main session, subagents, CI). No setup needed: every call launches a fresh browser, performs the operation, and closes it automatically. Detailed command reference, auth helpers, and known pitfalls live in `scripts/AGENTS.md`.

**Playwright E2E (repo-local):**
```bash
cd website
E2E_AUTO_SERVER=1 npx playwright test <spec-or-dir>
```
This auto-starts local website/API on `59010/59011`. If startup fails because ports are already in use, kill stale `vite` / `api-server` processes first.

### Browser Rules
- ALWAYS run `scripts/browser.js errs` after shipping any UI change to catch JS errors before the user does.
- Use `snap` for structure/existence checks and `shot` only for layout verification.
- Use `dc-auth.js` or `--seed` when a browser flow requires an authenticated session.
- **Fallback:** When browser automation isn't sufficient, use `api-cli` to test the full rental flow directly against the API.

## DEPLOY-TIME VALIDATION
- Validate all feature configuration at startup or deploy time, NEVER at request time. Users must not discover misconfigurations through runtime errors.
- If a feature requires an env var, validate it when the server starts. If malformed, refuse to start.
- If a feature requires an external service, check connectivity during `api-server doctor` (runs as a deploy gate in docker-compose via `service_completed_successfully`).
- New env-dependent features must update:
  1. `serve_command()` startup validation
  2. `doctor_command()` checks
  3. `api/.env.example` and `cf/.env.example` (documentation templates)
  4. docker-compose env sections
  5. `scripts/dc-secrets set shared/env <KEY>=<value>`

## ARCHITECTURAL ISSUES THAT REQUIRE A HUMAN DECISION
Stop work, document the issue in `TODO.md`, and ask how to proceed if you find:
- duplicate/conflicting API endpoints
- conflicting schema definitions or business logic implementations
- circular dependencies
- inconsistent data models
- security vulnerabilities or auth bypasses
- race conditions or concurrency hazards
- breaking changes to public APIs

Do NOT simply "fix" tests or code to work around these issues. The symptom fix masks the root cause and creates technical debt. **Example:** If tests fail because endpoint A shadows endpoint B with the same path, do NOT update tests to match endpoint A's response format. Instead, flag that two endpoints conflict and ask which one should be kept.

## POST-CHANGE CHECKLIST
1. **Run locally**: Build a local debug binary and run it with all required env vars against real services to ensure code behaves as expected. Fix any issues you encounter, even if unrelated to your changes.
2. **Verify endpoints/payloads**: Run HTTP requests against real endpoints (e.g. staging instances) to verify endpoints and payload formats *before* writing code. Required when task involves interaction with other services.
3. **UI/Navigation**: If the feature is user-facing, update UI components and sidebar/navigation menus as needed.
4. **Test coverage**: Add non-overlapping tests — each test must assert meaningful behavior unique to that test.
5. **E2E tests**: Add end-to-end tests for user-facing features where appropriate.
6. **Zombie code removal**: Search for and remove unused functions/structs/modules, deprecated code paths, legacy comments, orphaned imports, dead feature flags.
7. **Clean build**: Run `cargo clippy --tests` and `cargo nextest run` in the project you changed and fix ANY warnings or errors.
8. **Minimal diff**: Check `git diff` and confirm changes are minimal and aligned with project rules.
9. **Commit**: Only commit when the implementation is fully done and verification is clean.

## AUTOMATION AND CONFIG CHECKS
- Automate external-service setup whenever APIs allow it.
- If manual setup remains necessary, teach `api-server doctor` to validate it and explain fixes.
- Document manual setup steps in doctor output, not just markdown.

## BACKGROUND TASK POLLING
- Poll background tasks no more frequently than every 10 seconds.

## THIRD-PARTY SOURCE
- `third_party/` contains vendored external code such as Chatwoot and other integrations; use it to inspect implementation details and API contracts, not as first-party architecture to extend casually.

## MCP / EXTERNAL REFERENCE TOOLS
- Use Context7 when you need current library/API docs.
- Use web search when official/project docs are insufficient.

## TESTING AND DEPLOYMENT GUIDE
### Runtime Architecture
```text
api-cli -> API Server <- dc-agent
             |              |
        Cloudflare DNS     Caddy
```

### Build Commands
```bash
cargo build -p api --bin api-cli
cargo build -p api --bin api-server
cargo build -p dc-agent
```

### Identity Management
```bash
api-cli identity generate --name e2e-test
api-cli identity list
api-cli identity show e2e-test
```

### Environment Selection
```bash
api-cli contract list --identity test
api-cli --api-url https://dev-api.decent-cloud.org contract list --identity test
api-cli --env prod contract list --identity test
```

### Contract Lifecycle
- States: `requested -> pending -> accepted -> provisioning -> provisioned -> active`
- Terminal: `cancelled`, `rejected`, `failed`

```bash
# Discover offerings
api-cli --api-url https://dev-api.decent-cloud.org contract list-offerings --in-stock-only
# Create contract (--skip-payment for testing)
api-cli --api-url https://dev-api.decent-cloud.org contract create \
  --identity e2e-test --offering-id 11 \
  --ssh-pubkey "ssh-ed25519 AAAA..." --skip-payment
# Wait for provisioning
api-cli --api-url https://dev-api.decent-cloud.org contract wait <ID> \
  --state active --timeout 120 --identity e2e-test
# Get details (gateway_subdomain, gateway_ssh_port, port range)
api-cli --api-url https://dev-api.decent-cloud.org contract get <ID> --identity e2e-test
# Cancel (triggers VM termination + full cleanup)
api-cli --api-url https://dev-api.decent-cloud.org contract cancel <ID> --identity e2e-test
```

### E2E Test Commands
```bash
# Quick lifecycle test (no VM)
api-cli --api-url https://dev-api.decent-cloud.org e2e lifecycle --identity e2e-test

# Full provisioning test (requires running dc-agent)
api-cli --api-url https://dev-api.decent-cloud.org e2e provision \
  --identity e2e-test --offering-id 11 \
  --ssh-pubkey "$(cat ~/.ssh/id_ed25519.pub)" \
  --verify-ssh --cleanup

# Full suite (health + lifecycle + provisioning + DNS)
export CLOUDFLARE_API_TOKEN=<token> CLOUDFLARE_ZONE_ID=<zone_id>
export CF_GW_PREFIX=dev-gw CF_DOMAIN=decent-cloud.org
api-cli --api-url https://dev-api.decent-cloud.org e2e all \
  --identity e2e-test \
  --ssh-pubkey "$(cat ~/.ssh/id_ed25519.pub)"
```
Options: `--skip-provision` to skip VM test, `--skip-dns` to skip DNS test.

### Gateway Operations
```bash
api-cli gateway ssh --host <SUBDOMAIN> --port 20000 --identity-file ~/.ssh/id_ed25519
api-cli gateway tcp --host <SUBDOMAIN> --external-port 20001
api-cli gateway contract <CONTRACT_ID> --identity e2e-test
```

### DNS Operations
```bash
api-cli dns create --subdomain test-slug --ip 203.0.113.1
api-cli dns get --subdomain test-slug
api-cli dns list
api-cli dns delete --subdomain test-slug
```
- Requires `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ZONE_ID`; optional `CF_GW_PREFIX`, `CF_DOMAIN`.

### dc-agent Setup
```bash
dc-agent setup token \
  --token <AGENT_TOKEN> \
  --setup-proxmox \
  --gateway-dc-id my-dc-01 \
  --gateway-public-ip 203.0.113.1
```
This single command: registers agent, generates keypair, creates Proxmox API token + OS templates, registers gateway with central API for acme-dns credentials, installs Caddy with acmedns plugin, configures networking (IP forwarding, NAT, firewall), writes per-provider wildcard TLS config, writes config, installs systemd services.

Additional flags: `--gateway-domain <DOMAIN>` (default: `decent-cloud.org`), `--gateway-gw-prefix <PREFIX>` (default: `gw`, use `dev-gw` for staging), `--gateway-port-start/end` (default: `20000`/`59999`), `--gateway-ports-per-vm` (default: `10`), `--non-interactive`, `--force`.

### dc-agent Diagnostics
```bash
dc-agent doctor                      # full check
dc-agent doctor --no-verify-api      # skip API auth
dc-agent doctor --no-test-provision  # skip VM test
```
Checks: config, provisioner connectivity, gateway status (Caddy running, ports 80/443 open), API authentication, optional provision/terminate cycle.

### dc-agent Test Provisioning
```bash
dc-agent test-provision                                       # provision + terminate
dc-agent test-provision --keep --ssh-pubkey "ssh-ed25519 ..."  # keep VM running
dc-agent test-provision --test-gateway                         # include gateway routing
dc-agent test-provision --test-gateway --skip-dns              # gateway without DNS
```

### Gateway Naming
- Gateway subdomains are `{slug}.{dc_id}.{gw_prefix}.{domain}`.
- Examples: `k7m2p4.dc-lk.dev-gw.decent-cloud.org` (staging), `k7m2p4.a3x9f2b1.gw.decent-cloud.org` (prod).
- Per-provider wildcard cert `*.{dc_id}.{gw_prefix}.{domain}` via DNS-01 with acme-dns (scoped to each provider).

### Typical End-to-End Agent Flow
```bash
api-cli identity generate --name test
api-cli --api-url https://dev-api.decent-cloud.org health api
api-cli --api-url https://dev-api.decent-cloud.org contract list-offerings --in-stock-only
api-cli --api-url https://dev-api.decent-cloud.org contract create \
  --identity test --offering-id <ID> \
  --ssh-pubkey "$(cat ~/.ssh/id_ed25519.pub)" --skip-payment
api-cli --api-url https://dev-api.decent-cloud.org contract wait <CONTRACT_ID> \
  --state active --timeout 300 --identity test
api-cli --api-url https://dev-api.decent-cloud.org contract get <CONTRACT_ID> --identity test
ssh -o StrictHostKeyChecking=no -p <GATEWAY_SSH_PORT> root@<GATEWAY_SUBDOMAIN>
api-cli --api-url https://dev-api.decent-cloud.org contract cancel <CONTRACT_ID> --identity test
```

### Deploying dc-agent Changes To A Host
```bash
cargo build -p dc-agent --release
scp target/release/dc-agent root@proxmox-host:/usr/local/bin/
ssh root@proxmox-host systemctl restart dc-agent
ssh root@proxmox-host journalctl -u dc-agent -f
ssh root@proxmox-host dc-agent doctor --no-test-provision
```

### Key Environment Variables
| Variable | Used by | Purpose |
|----------|---------|---------|
| `CLOUDFLARE_API_TOKEN` | api-cli dns/e2e | Cloudflare DNS management |
| `CLOUDFLARE_ZONE_ID` | api-cli dns/e2e | Cloudflare zone identifier |
| `CF_GW_PREFIX` | api-cli, api-server | Gateway DNS prefix |
| `CF_DOMAIN` | api-cli, api-server | Base domain |
| `CF_API_TOKEN` | api-server | Server-side Cloudflare token |
| `CF_ZONE_ID` | api-server | Server-side zone ID |
| `DATABASE_URL` | api-server, api-cli | PostgreSQL connection string |
| `STRIPE_SECRET_KEY` | api-cli health | Stripe verification |
| `TELEGRAM_BOT_TOKEN` | api-cli health | Telegram verification |
| `MAILCHANNELS_API_KEY` | api-cli health | Email verification |

## KNOWN ISSUES
- DNS propagation can delay Let's Encrypt visibility after record creation.
- First wildcard cert issuance on a new host often requires checking `journalctl -u caddy` if Cloudflare config is wrong.
