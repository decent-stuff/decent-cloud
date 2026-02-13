# TODO

## HIGHEST PRIORITY: Deploy Gateway TLS Isolation

**Spec:** [2026-02-13-gateway-tls-isolation-spec.md](docs/specs/2026-02-13-gateway-tls-isolation-spec.md)
**Status:** Code complete, needs deployment and end-to-end verification

### Deploy to Dev
- [ ] **Set `API_PUBLIC_URL`** in dev API docker-compose/env (`https://dev-api.decent-cloud.org`)
- [ ] **Deploy api-server** to dev with migration 011 (acme_dns_accounts table)
- [ ] **Verify endpoint** — `POST /api/v1/acme-dns/update` responds 401 without credentials

### End-to-End Verification
- [ ] **Test gateway registration** — `POST /api/v1/agents/gateway/register` returns credentials with server_url pointing to our API
- [ ] **Test TXT proxying** — Call `POST /api/v1/acme-dns/update` with returned credentials, verify TXT record appears in Cloudflare at `_acme-challenge.{dc_id}.{gw_prefix}.decent-cloud.org`
- [ ] **Deploy dc-agent** — Build and deploy to Proxmox host, run `dc-agent setup token --gateway-dc-id <id>` against dev API
- [ ] **Verify Caddy cert** — Confirm Caddy obtains wildcard cert `*.{dc_id}.dev-gw.decent-cloud.org` via the new flow

### Cleanup After Verification
- [ ] **Remove old CNAME records** — Delete any `_acme-challenge.*.dev-gw` CNAME records from Cloudflare (replaced by TXT)
- [ ] **Verify no `ACME_DNS_SERVER_URL`** references remain in deployment configs

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
