# TODO

- Easy: the login buttons for google login vs seed phrase (create new + import) and quite different - improve consistency, design, and UX

**Specs:**
- [docs/specs/2026-02-14-decent-recipes.md](docs/specs/2026-02-14-decent-recipes.md)
- [docs/specs/2026-02-14-hetzner-provisioner.md](docs/specs/2026-02-14-hetzner-provisioner.md)
- [docs/specs/2026-02-14-self-provisioning-platform.md](docs/specs/2026-02-14-self-provisioning-platform.md)

---

## Next Priority

### Cloud Provisioning — Incomplete

1. **Set gateway fields on cloud-provisioned contracts** — `update_contract_provisioned_by_cloud_resource()` writes `provisioning_instance_details` JSON but does NOT set `gateway_subdomain`, `gateway_ssh_port`, `gateway_port_range_start`, or `gateway_port_range_end` on the contract row. These must be set for gateway routing to work (like dc-agent's `mark_contract_provisioned_for_agent()` does).
2. **Contract expiration → resource deletion** — Cancel triggers cleanup, but contract expiration (`end_timestamp_ns` reached) does not. Add a background job to `cleanup_service.rs` that finds expired contracts and calls `mark_contract_resource_for_deletion`.
3. **Hetzner E2E test** — Add an e2e test that creates a contract with a Hetzner offering, waits for active, verifies cloud resource creation and gateway_subdomain, tests SSH connectivity, and cancels.

### Cloud Provisioning — Longer-term

4. **Recipe marketplace UI** — Website needs UI for browsing/purchasing recipe offerings and viewing provisioned recipe instances with connection details.
5. **Recipe script versioning** — Scripts are snapshotted at contract creation. Consider a `recipe_versions` table so authors can update scripts and buyers can upgrade.

---

## Provider Provisioning Agent (dc-agent)

**Spec:** [2025-12-07-provider-provisioning-agent-spec.md](docs/2025-12-07-provider-provisioning-agent-spec.md)
**Status:** MVP complete through Phase 7 (Credential Encryption). Phase 8 (Hetzner) implemented server-side in api-server via `HetznerBackend`, not in dc-agent.

- Phase 9: Docker, DigitalOcean, Vultr provisioners

---

## Provider Trust & Reliability System

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
