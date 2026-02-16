# TODO

- Login buttons for Google login vs seed phrase (create new + import) are inconsistent — improve consistency, design, and UX

**Specs:**
- [docs/specs/2026-02-14-decent-recipes.md](docs/specs/2026-02-14-decent-recipes.md)
- [docs/specs/2026-02-14-hetzner-provisioner.md](docs/specs/2026-02-14-hetzner-provisioner.md)
- [docs/specs/2026-02-14-self-provisioning-platform.md](docs/specs/2026-02-14-self-provisioning-platform.md)

---

## Cloud Provisioning

### Known Limitations

- **Multi-instance race** — If two API server instances share the same DB, both provisioning services race on the same resources. The 10-minute lock timeout prevents corruption but can cause delayed provisioning or double-attempt waste. Not a prod issue if only one instance runs.
- **Hetzner server type availability is location-dependent** — No server-side validation before sending to Hetzner API; user sees a cryptic "unsupported location for server type" error.

### Longer-term

- **Recipe marketplace UI** — Website needs UI for browsing/purchasing recipe offerings and viewing provisioned recipe instances with connection details.
- **Recipe script versioning** — Scripts are snapshotted at contract creation. Consider a `recipe_versions` table so authors can update scripts and buyers can upgrade.

---

## Provider Provisioning Agent (dc-agent)

**Spec:** [2025-12-07-provider-provisioning-agent-spec.md](docs/2025-12-07-provider-provisioning-agent-spec.md)
**Status:** MVP complete through Phase 7 (Credential Encryption). Phase 8 (Hetzner) implemented server-side in api-server via `HetznerBackend`, not in dc-agent.

- Phase 9: Docker, DigitalOcean, Vultr provisioners

---

## Provider Trust & Reliability System

DB tables (`contract_health_checks`) and query methods exist but no automated health check scheduling is implemented.

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
Implement direct ICRC-1 transfers from platform wallet using `ic-agent`.

---

## Rental State Machine

- [ ] **Contract archival** — Old contracts stay in DB indefinitely

---

## Architectural Issues Requiring Review

### Hardcoded Token Value ($1 USD)

**Issue:** Token USD value hardcoded instead of fetched from exchanges.
**Location:** `ic-canister/src/canister_backend/generic.rs:75-78`
**FIXME in code:** `refresh_last_token_value_usd_e6()` always returns `1_000_000` ($1 USD). Needs ICPSwap/KongSwap integration.
