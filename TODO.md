# TODO

- Easy: the login buttons for google login vs seed phrase (create new + import) and quite different - improve consistency, design, and UX

**Specs:**
- [docs/specs/2026-02-14-decent-recipes.md](docs/specs/2026-02-14-decent-recipes.md)
- [docs/specs/2026-02-14-hetzner-provisioner.md](docs/specs/2026-02-14-hetzner-provisioner.md)
- [docs/specs/2026-02-14-self-provisioning-platform.md](docs/specs/2026-02-14-self-provisioning-platform.md)

---

## Next Priority

### Cloud Provisioning

**E2E verified locally:** add-account → provision (cx23/nbg1) → VM running (real IP, SSH reachable) → delete → Hetzner fully cleaned up (server + SSH key deleted).

### Cloud Provisioning — Completed

1. ~~**Set gateway fields on cloud-provisioned contracts**~~ — Done: `update_contract_provisioned_by_cloud_resource()` now sets `gateway_slug`, `gateway_subdomain`, `gateway_ssh_port` on the contract row. SSH port = 22 (direct), no port range (public IP VMs).
2. ~~**Contract expiration → resource deletion**~~ — Done: `cleanup_service.rs` calls `expire_and_cleanup_cloud_contracts()` which finds expired active contracts with cloud resources and marks them for deletion.
3. ~~**Gateway DNS + routing not wired**~~ — Done: `cloud_provisioning_service` creates DNS A records via Cloudflare on provision, deletes them on termination. DNS failure is non-fatal.
4. ~~**Automated E2E test**~~ — Done: `api-cli e2e cloud-provision` tests full lifecycle (add account → provision cx22 → verify SSH → delete → cleanup).
5. ~~**Remaining `let _ =` violations**~~ — Done: Fixed in `providers.rs`, `gateway.rs`, `main.rs`.

### Cloud Provisioning — Known Limitations

6. **Multi-instance race** — If two API server instances share the same DB (e.g., dev + local during testing), both provisioning services race on the same resources. The 10-minute lock timeout prevents corruption but can cause delayed provisioning or double-attempt waste. Not a prod issue if only one instance runs, but fragile.
7. **Hetzner server type availability is location-dependent** — CPX (old gen) types don't work in all locations. CX23+ (new gen) work in nbg1, hel1, fsn1. No server-side validation before sending to Hetzner API; user sees a cryptic "unsupported location for server type" error.
8. **No provisioning error details stored** — When provisioning fails, status is set to `failed` but the error message is only in server logs, not in the database. Users see "failed" with no explanation.

### Cloud Provisioning — Longer-term

9. **Recipe marketplace UI** — Website needs UI for browsing/purchasing recipe offerings and viewing provisioned recipe instances with connection details.
10. **Recipe script versioning** — Scripts are snapshotted at contract creation. Consider a `recipe_versions` table so authors can update scripts and buyers can upgrade.
11. **UI: show SSH access instructions** — Provisioned resources have IP + SSH key but the UI doesn't show how to connect. Generate and display the `ssh` command.
12. **Store provisioning error in DB** — Add `error_message TEXT` column to `cloud_resources` and populate it on failure for user-facing diagnostics.

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
