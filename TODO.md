# TODO

## HIGH PRIORITY: Self-Hosted Resource Management ✅ COMPLETE

**Goal:** Transform platform from pure marketplace into unified resource management + marketplace. Users can manage own infrastructure, rent from others, and deploy value-add services on top.

All three phases complete:
- Phase 1: Visibility & Self-Rental
- Phase 2: Shared Visibility
- Phase 3: Post-Provision Scripts

---

## API-CLI Testing Framework

**Goal:** Enable automated E2E testing of the full VM provisioning flow without the website frontend, for CI/CD pipelines and AI agent-assisted testing.

### Completed ✓
- Identity management (generate, import, list, show, delete) with storage at `~/.dc-test-keys/`
- Ed25519ph signing client matching API authentication requirements
- Account operations (create, get, update-email, add-ssh-key, list-ssh-keys)
- Contract operations (list-offerings, create, get, wait, list, cancel)
- Offering/Provider listing commands
- Health checks for all external services (API, Stripe, Telegram, Cloudflare, MailChannels)
- Gateway connectivity testing structure (ssh, tcp, contract)
- E2E test scaffolding (provision, lifecycle, all)
- `--skip-payment` flag with `set_payment_status_for_testing()` DB function

### Remaining Work
- [ ] **Test contract create/wait/cancel** - Requires running provider with dc-agent
- [ ] **Test gateway commands** - Requires active contracts with assigned gateways
- [ ] **Test DNS commands** - Requires Cloudflare credentials in environment
- [ ] **Full E2E test run** - Provision → verify SSH → cleanup cycle

---

## Streamlined Rental UX ✅ COMPLETE

**Goal:** Eliminate provider approval bottleneck; enable instant provisioning like AWS/GCP/Azure

Complete:
- Auto-Accept Mode
- Instant SSH Key Injection
- Pre-configured Offering Templates

---

## Provider Provisioning Agent

**Spec:** [2025-12-07-provider-provisioning-agent-spec.md](docs/2025-12-07-provider-provisioning-agent-spec.md)
**Status:** MVP complete through Phase 7 (Credential Encryption)

### Completed ✓ (Phases 1-7)
- Agent skeleton, TOML config, Ed25519 auth, Proxmox provisioner
- Setup wizard, test provisioning, delegated agent keys
- Agent pools with location-based routing
- Health & Reputation (API + dashboard)
- Credential Encryption (Ed25519→X25519 + XChaCha20Poly1305)

### Phase 7 Follow-ups (Low Priority)
- [ ] **Consider on-demand password reset** - dc-agent SSHs into VM and runs `passwd` on user request
- [ ] **Add AAD binding** - Include contract_id in encryption context
- [ ] **Multi-device limitation** - Consider account-level key derivation for cross-device access

### Future Phases
- Phase 8: Hetzner Cloud provisioner
- Phase 9: Docker, DigitalOcean, Vultr provisioners

---

## Provider Trust & Reliability System

### External Benchmarking Integration
Integrate with or scrape external sources for additional trust signals:
- https://serververify.com/ - Server verification and uptime data
- https://www.vpsbenchmarks.com/ - VPS performance benchmarks
- Price comparison vs market average

### Service Quality Verification
- Automated health checks on provisioned services
- Uptime monitoring and SLA compliance tracking

### User Feedback System (Structured, Not Reviews)
- Post-contract structured survey: "Did service match description?" Y/N
- "Would you rent from this provider again?" Y/N

---

## Notification System

### Paid Notification Tiers
- Define pricing for additional notifications beyond free tier
- Integrate with payment system (Stripe/ICPay)
- Track paid quota separately from free tier

---

## ICPay Integration

### Manual Payout Requirement
**ICPay does NOT have a programmatic payout API.** Provider payouts must be done manually:
1. View pending releases: `GET /api/v1/admin/payment-releases`
2. Create payouts in icpay.org dashboard (Payouts section)
3. Mark as paid: `POST /api/v1/admin/payouts`

### Future: Automated Payouts
To automate payouts, implement direct ICRC-1 transfers from platform wallet using `ic-agent`.

---

## Rental State Machine (Low Priority Issues)

1. **"active" status never assigned** - Contracts stay "provisioned" forever
2. **No centralized state transition validator** - Risk of invalid transitions
3. **Float arithmetic for payment calculation** - Consider integer-only math
4. **No contract archival** - Old contracts stay in DB indefinitely

---

## Architectural Issues Requiring Review

**Priority:** MEDIUM - Technical debt

### Open Issues

#### 6. Inconsistent Error Handling Patterns

**Issue:** Multiple error handling strategies used inconsistently.

**Patterns found:**
1. Custom error types: `TransferError`, `CryptoError`, `AuthError`, `LedgerError`
2. `anyhow::Result<T>` with `.context()`
3. `.ok()` to discard errors
4. `.unwrap_or_default()` to silence failures
5. `panic!()` in public APIs

**Recommendation:** Establish project-wide error handling policy.

#### 7. Timestamp Handling with `.unwrap_or(0)`

**Issue:** `.timestamp_nanos_opt().unwrap_or(0)` silently uses epoch time on error.

**Locations:** `api/src/database/agent_delegations.rs:264, 362, 410`

**Note:** Only fails for dates beyond ~2262, so extremely unlikely.

#### 10. Hardcoded Token Value ($1 USD)

**Issue:** Token USD value hardcoded instead of fetched from exchanges.

**Location:** `ic-canister/src/canister_backend/generic.rs:75-78`

#### 12. Half-Implemented Canister Proxy

**Issue:** ICP canister method proxy returns stub error.

**Location:** `api/src/main.rs:214-227`

#### 14. Manual Setup Steps That Could Be Automated

**Issue:** Several setup steps require manual intervention when APIs support automation.

**Examples:**
- Stripe webhook creation (can use Stripe API)
- DKIM DNS record setup (can use Cloudflare API)
- Proxmox template VM creation

**Recommendation:** Add `api-server setup-<service>` commands.

### Fixed Issues (2026-02-02)

- ✅ Issue 1: Duplicate API Response Types - Moved to `dcc-common` crate
- ✅ Issue 2: Chatwoot Client Error Detail Loss - Proper error context added
- ✅ Issue 3: Hex Decoding Without Validation - Match error handling added
- ✅ Issue 4: Commented-Out ICRC3 Modules - Dead code removed
- ✅ Issue 5: Hardcoded Localhost URLs - Doctor check + startup warning added, port inconsistency fixed
- ✅ Issue 8: Geographic Region Mapping Duplicate - Moved to `dcc-common`
- ✅ Issue 9: CLI Commands with `todo!()` Stubs - Non-functional commands removed
- ✅ Issue 11: Hardcoded Secrets - Reviewed, not an issue
- ✅ Issue 13: Missing `api-server doctor` Checks - All service checks added

---

## Nice-to-Have Improvements

### Cosmetic: Username-Based URLs

**Priority:** LOW

Add `GET /accounts/by-username/{username}` endpoint for cleaner URLs.

---
