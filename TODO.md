# TODO

## API-CLI Testing Framework

**Goal:** Enable automated E2E testing of the full VM provisioning flow without the website frontend, for CI/CD pipelines and AI agent-assisted testing.

### Remaining Work
- [x] **Test contract create/wait/cancel** - Full lifecycle via api-cli against dev API + Proxmox dc-agent
- [x] **Test gateway commands** - Port forwarding, Caddy TLS, bandwidth monitoring
- [x] **Test DNS commands** - Cloudflare A record create/delete via central API
- [x] **Full E2E test run** - Provision → verify SSH → cleanup cycle

### E2E Test Results (2026-02-13)
Tested on Proxmox VE 9.1 at 203.189.67.78 with dc-agent 0.4.9.

**Verified working:**
- VM provisioning from template (clone → configure → start → IP via guest agent): ~26s
- SSH key injection via cloud-init
- SSH connectivity via gateway port forwarding (port 20000)
- Gateway: iptables DNAT port forwarding (SSH on port 20000)
- Gateway: Caddy TLS termination with Let's Encrypt certs (HTTP-01)
- DNS: Cloudflare A record create/delete via central API (`slug.dc-id.dev-gw.decent-cloud.org`)
- Full cleanup: VM terminate + iptables flush + Caddy config remove + DNS delete + port deallocation
- Contract lifecycle: create → accepted → provisioning → active → cancelled (all transitions)
- E2E all suite: health check + contract lifecycle + provisioning + DNS (4/4 pass)

**Bugs found and fixed:**
1. `CloudflareDns::from_env()` accepted empty env vars → added `.filter(|s| !s.is_empty())`
2. `CF_GW_PREFIX` not passed to Docker containers → added to both compose files
3. `CF_*` credentials missing from `.env.dev` → added Cloudflare config section
4. `admin off` in Caddyfile broke `caddy reload` → changed to `admin localhost:2019`
5. `test-provision` cleanup passed `contract_id` instead of `slug` to `cleanup_gateway` → use `instance.gateway_slug`
6. Config field renamed `datacenter` → `dc_id` but server config not updated
7. `Database::new()` runs migrations on connect → api-cli crashed on dev DB with existing tables → added `Database::connect()` for migration-free connections
8. `uuid::Uuid::parse_str` used for hex-encoded contract IDs → api-cli crash on 64-char hex IDs → switched to `hex::decode`
9. `wait_for_contract_status` missed target when contract progressed past it (e.g. `provisioned` → `active`) → added status progression ranking
10. `set_payment_status_for_testing` DB call was redundant (ICPay auto-succeeds payment) → removed DB dependency from E2E tests
11. Heartbeat `active_contracts` reported pending-to-provision count instead of running VM count → use reconcile instance count
12. SSH verification silently warned on DNS propagation delay → added retry loop (6 attempts, 10s interval) with troubleshooting info on failure

**Known timing issue:** Caddy cert acquisition can fail on first attempt if DNS hasn't propagated to Let's Encrypt resolvers. Caddy retries automatically after 60s and succeeds.

### Running E2E Tests

```bash
# Generate test identity (one-time)
api-cli identity generate --name e2e-test

# Quick lifecycle test (no VM provisioning, ~1s)
api-cli --api-url https://dev-api.decent-cloud.org e2e lifecycle --identity e2e-test

# Full provisioning test (~60s, requires running dc-agent on Proxmox)
api-cli --api-url https://dev-api.decent-cloud.org e2e provision \
  --identity e2e-test --offering-id 11 \
  --ssh-pubkey "$(cat ~/.ssh/id_ed25519.pub)" \
  --verify-ssh --cleanup

# Full E2E suite (lifecycle + provisioning + DNS)
export CLOUDFLARE_API_TOKEN=<token> CLOUDFLARE_ZONE_ID=<zone_id> CF_GW_PREFIX=dev-gw CF_DOMAIN=decent-cloud.org
api-cli --api-url https://dev-api.decent-cloud.org e2e all \
  --identity e2e-test \
  --ssh-pubkey "$(cat ~/.ssh/id_ed25519.pub)"

# Individual contract commands
api-cli --api-url https://dev-api.decent-cloud.org contract create --identity e2e-test --offering-id 11 --ssh-pubkey "..." --skip-payment
api-cli --api-url https://dev-api.decent-cloud.org contract wait <id> --state active --timeout 120 --identity e2e-test
api-cli --api-url https://dev-api.decent-cloud.org contract get <id> --identity e2e-test
api-cli --api-url https://dev-api.decent-cloud.org contract cancel <id> --identity e2e-test
api-cli --api-url https://dev-api.decent-cloud.org contract list --identity e2e-test
```

---

## Provider Provisioning Agent

**Spec:** [2025-12-07-provider-provisioning-agent-spec.md](docs/2025-12-07-provider-provisioning-agent-spec.md)
**Status:** MVP complete through Phase 7 (Credential Encryption)

### Phase 7 Follow-ups
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

### Open Issues

#### 10. Hardcoded Token Value ($1 USD)

**Issue:** Token USD value hardcoded instead of fetched from exchanges.

**Location:** `ic-canister/src/canister_backend/generic.rs:75-78`
