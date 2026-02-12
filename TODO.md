# TODO

## API-CLI Testing Framework

**Goal:** Enable automated E2E testing of the full VM provisioning flow without the website frontend, for CI/CD pipelines and AI agent-assisted testing.

### Remaining Work
- [ ] **Test contract create/wait/cancel** - Requires running provider with dc-agent
- [ ] **Test gateway commands** - Requires active contracts with assigned gateways
- [ ] **Test DNS commands** - Requires Cloudflare credentials in environment
- [ ] **Full E2E test run** - Provision → verify SSH → cleanup cycle

---

## Provider Provisioning Agent

**Spec:** [2025-12-07-provider-provisioning-agent-spec.md](docs/2025-12-07-provider-provisioning-agent-spec.md)
**Status:** MVP complete through Phase 7 (Credential Encryption)

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

### Future: Integrate Feedback into Trust Score
- [x] **User Feedback System** - Implemented: POST/GET `/contracts/{id}/feedback`, GET `/providers/{pubkey}/feedback-stats`
- [x] **Trust score integration** - Feedback stats (service match rate, would-rent-again rate) integrated into trust score with penalties (<50%), bonus (>80% with 5+ reviews), and minimum 3 responses threshold

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

## Rental State Machine

### Remaining (Low Priority)
- [ ] **Contract archival** - Old contracts stay in DB indefinitely

---

## Architectural Issues Requiring Review

**Priority:** MEDIUM - Technical debt

### Open Issues

#### 7. Timestamp Handling with `.unwrap_or(0)`

**Issue:** `.timestamp_nanos_opt().unwrap_or(0)` silently uses epoch time on error.

**Locations:** `api/src/database/agent_delegations.rs:264, 362, 410`

**Note:** Only fails for dates beyond ~2262, so extremely unlikely.

#### 10. Hardcoded Token Value ($1 USD)

**Issue:** Token USD value hardcoded instead of fetched from exchanges.

**Location:** `ic-canister/src/canister_backend/generic.rs:75-78`

---

## Nice-to-Have Improvements

### Cosmetic: Username-Based URLs

**Priority:** LOW

Add `GET /accounts/by-username/{username}` endpoint for cleaner URLs.
