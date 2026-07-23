# Cost-Safe Billing & Money-Flow Architecture

**Date:** 2026-07-23
**Type:** Research / architecture document (NO code changes)
**Hard requirement:** It must be *architecturally impossible* for the platform to pay out or reimburse more than it has collected after fees. This document maps the current money flow, enumerates every hole, verifies the Stripe compliance constraints, and proposes a DB-invariant-backed design.

> **Honesty note.** Every code claim is cited `file:line` against `repo/` (the submodule). Every Stripe fact is cited to `docs.stripe.com` / `support.stripe.com` / `stripe.com`. Anything I could *not* verify is called out explicitly in [§7](#7-what-i-could-not-verify). The compliance constraints are stated bluntly, not softened.

---

## TL;DR

1. **There is no pre-pay balance model and no escrow today.** Each rental is a one-off **Stripe Hosted Checkout** redirect (`api/src/openapi/contracts.rs:104`, confirmed redirect-based). "Balance" in the UI (`website/src/routes/dashboard/transfers/+page.svelte`) is the **on-chain ICP token ledger**, not fiat (`api/src/database/tokens.rs:69`).
2. **No Stripe Connect, no `application_fee_amount`, no provider payout via Stripe exists.** The only "payout" path is ICPay-only accounting (`payment_release_service.rs`) plus a manual admin "payout" (`api/src/openapi/admin.rs:686`). This is consistent with **Phase 1** (sole provider = owner = merchant of record) but is **incompatible with Phase 2** (third-party providers).
3. **Money invariants are enforced by application code, not by the database.** `payment_status`, `total_released_e9s`, `refund_amount_e9s` are free-text/`BIGINT` columns with **zero CHECK constraints** (`001_schema.sql:466-535`). The strongest guard — `acquire_provisioning_lock` (`provisioning.rs:898-899`) — is a conditional `UPDATE`, but the provider's own accept/reject path (`rental.rs:273`) **does not check `payment_status` at all**.
4. **The "5–10% markup covers fees" model breaks badly on small rentals.** With the US Stripe rate (2.9% + $0.30), a 10% markup is only profitable at provider-cost **≥ $4.41**; at the 5% markup floor the break-even is **≥ $15.35**. Per-rental Checkout pays the $0.30 fixed fee *on every rental* — the single biggest argument for pre-pay top-ups that amortize it.
5. **Phase 1 needs no Connect** (owner sells own compute; standard Stripe is compliant). **Phase 2 *requires* Stripe Connect** (collecting from users and paying third-party providers without Connect is aggregation, a Stripe-TOS violation — [support.stripe.com](https://support.stripe.com/questions/restrictions-for-marketplaces-not-using-stripe-connect)).

---

## 1. Current money-flow map

### 1.1 Entry points and boot

- **Env / secret validation:** `STRIPE_SECRET_KEY` and `STRIPE_WEBHOOK_SECRET` are *optional* (`api/src/main.rs:939-944`, `main.rs:195-200`). The server **boots without Stripe configured** — payments silently disable but the server runs. There is no fail-fast requirement that Stripe be set in production (only a `WARN`).
- **Webhook route:** registered at `/api/v1/webhooks/stripe` (`main.rs:1349-1351`) → `openapi::webhooks::stripe_webhook` (`api/src/openapi/webhooks.rs:161`). Signature verified with constant-time HMAC (`webhooks.rs:134-157`).
- **Webhook events handled:** `checkout.session.completed`, `invoice.paid`, `invoice.payment_failed`, `customer.subscription.{created,updated,deleted}`, and the four `charge.dispute.*` events (`webhooks.rs:219-731`). `payment_intent.*` events are deliberately NOT used (`webhooks.rs:725-728`).

### 1.2 Handlers touching money (file:line + role)

| Handler / file:line | Role in money flow |
|---|---|
| `POST /contracts` → `create_rental_request` (`openapi/contracts.rs:588`) | Creates contract; computes `payment_amount_e9s`; for Stripe, calls `create_stripe_checkout_session` and returns a `checkout_url`. Self-rental & ICPay set `payment_status='succeeded'` immediately. |
| `create_stripe_checkout_session` (`openapi/contracts.rs:104`) | Converts `payment_amount_e9s`→cents (`/10_000_000`), builds a Hosted-Checkout session, returns the redirect URL. **No `application_fee_amount`, no `transfer_data`, no Connect.** |
| `StripeClient::create_checkout_session` (`stripe_client.rs:44`) | Mode = `Payment`; `invoice_creation.enabled=true`; metadata `contract_id`. One line item = the full rental price. |
| `POST /contracts/verify-checkout` → `verify_checkout_session` (`openapi/contracts.rs:1278`) | Manual fallback that re-fetches the session from Stripe and calls `update_checkout_session_payment` (the same path the webhook uses). |
| `stripe_webhook` → `checkout.session.completed` (`webhooks.rs:220`) | The authoritative "paid" transition: `update_checkout_session_payment` sets `payment_status='succeeded'` (`payment.rs:22-33`). Contract stays `requested` pending provider review. |
| `PUT /contracts/:id/cancel` → `cancel_contract` (`openapi/contracts.rs:884` → `rental.rs:570`) | Prorated refund via `calculate_net_refund_e9s` (`payment.rs:174`). |
| Provider `reject_contract` (`rental.rs:366`) | **Full** refund (`full_refund = payment_amount_e9s`). |
| `timeout_cleanup_service.rs:192` (`cleanup_failed_provisioning`) | Stuck provisioning → `provisioningfailed` + full auto-refund via `issue_audited_refund`. |
| `webhooks.rs:934` (`handle_dispute_closed` lost) | `terminate_contract_for_dispute_lost` + `process_dispute_lost_refund` (`dispute.rs:459`). |
| `payment_release_service.rs:42` (`process_releases_once`) | **ICPay-only** daily provider release: creates `payment_releases` rows, bumps `total_released_e9s`. **Does not move money.** |
| `POST /admin/payouts` (`openapi/admin.rs:690`) | Manual admin trigger; calls `IcpayClient::create_payout` to a wallet address, marks releases `paid_out`. **Stripe path absent.** |
| `transfers.rs:14` / `tokens.rs:69` | ICP token ledger (crypto) reads only. Not fiat. |
| `subscriptions.rs` + `create_subscription_checkout` (`stripe_client.rs:353`) | SaaS subscription billing (the platform's own plan tiers), unrelated to per-rental escrow. |

### 1.3 Money-related tables and their actual constraints

From `api/migrations_pg/001_schema.sql` (lines cited) and later migrations:

- **`contract_sign_requests`** (`001_schema.sql:456-536`) — the central money row.
  - `payment_amount_e9s BIGINT NOT NULL` (`:466`) — **no `CHECK (>= 0)`**.
  - `payment_status TEXT NOT NULL DEFAULT 'pending'` (`:511`) — **free text, no CHECK constraint** (the migration comment at `:474` explicitly notes `status` is free-text; `payment_status` is the same).
  - `total_released_e9s BIGINT DEFAULT 0` (`:516`) — **no `CHECK (<= payment_amount_e9s)`**.
  - `refund_amount_e9s BIGINT` (`:521`) — **no upper-bound CHECK**.
  - Stripe columns: `stripe_checkout_session_id`, `stripe_payment_intent_id`, `stripe_customer_id`, `stripe_invoice_id`, `stripe_refund_id` (`:508-523`); split done by `042_rename_payment_intent_to_session.sql`.
  - Tax columns: `tax_amount_e9s`, `reverse_charge`, `customer_tax_id` etc. (`:525-530`).
- **`payment_releases`** (`001_schema.sql:1081-1093`) — provider-release accounting.
  - `release_type`/`status` have CHECKs (`:1084,1089`); `amount_e9s BIGINT NOT NULL` has **no per-contract cap** (no `CHECK (SUM(amount_e9s) <= payment_amount_e9s)` is expressible per-row; there is no aggregate guard either).
- **`refund_audit`** (`044_refund_audit.sql`) — good: `idempotency_key TEXT NOT NULL UNIQUE`, request/response payloads, `ON CONFLICT` collapses retries (`refund_audit.rs:62-90`). **No FK to contracts** (intentional; outlives the contract).
- **`pending_stripe_receipts`** (`payment.rs:463`) — receipt retry queue, not money.
- **`spending_alerts`** (`028_spending_alerts.sql`) — *notifications only* (`monthly_limit_usd`, `alert_at_pct`); **not a spend gate** — see `check_spending_alert_and_notify` best-effort call at `contracts.rs:789`.
- **`cloud_accounts`** (`014_cloud_accounts.sql`) / **`cloud_resources`** — provisioning state, **not money**. `cloud_resources` has a `contract_id` FK and an atomic reservation sub-select (`rental.rs:207-229`) but no payment precondition on the reservation.

### 1.4 The flow, end to end (Phase 1, Stripe path)

```
User clicks Rent
  → POST /contracts  (contracts.rs:588)
      create_rental_request (rental.rs:8)
        payment_amount_e9s = monthly_price * duration/720   (rental.rs:125-130)  ← NO markup applied here
        INSERT contract, status='requested', payment_status='pending'  (rental.rs:170-204)
      create_stripe_checkout_session (contracts.rs:104)
        Stripe Checkout Session (mode=Payment), returns URL
  ← { checkout_url }   (frontend: window.location.href = checkoutUrl — REDIRECT, confirmed)
User pays on Stripe Hosted Checkout
  → webhook checkout.session.completed (webhooks.rs:220)
      update_checkout_session_payment → payment_status='succeeded' (payment.rs:22)
      (contract still 'requested'; awaits provider accept)
Provider accepts → status accepted→provisioning→provisioned→active
  acquire_provisioning_lock (provisioning.rs:879) gates on payment_status='succeeded' (provisioning.rs:899)
User cancels / provider rejects / dispute lost / provisioning fails
  → prorated or full refund via Stripe Refund (issue_audited_refund, refund_audit.rs:176)
NO provider payout via Stripe ever occurs.
```

**Implication for a pre-pay model:** today *every rental is its own charge*. A redirect checkout per rental cannot support a balance debit (you charge the card fresh each time, paying the $0.30 fixed fee each time) and cannot support escrow (funds land on the platform balance with no separation per rental). Pre-pay requires a fundamentally different collection primitive (§5).

---

## 2. Negative-balance / over-payout / under-collection holes

Each is a concrete scenario with `file:line`. Severity: 🔴 can move money the platform doesn't have; 🟠 pays out more than collected-after-fees; 🟡 provisions/credits without payment or erodes margin.

### 2.1 🔴 No DB invariant links `payment_status` to provisioning — provider accept ignores payment

- `update_contract_status` (`rental.rs:273-360`) validates authorization and *status transition*, but **never reads `payment_status`**. A provider can drive `requested→pending→accepted` on a contract whose Stripe checkout never completed (`payment_status='pending'`).
- The hard gate is only `acquire_provisioning_lock` (`provisioning.rs:898-899`: `AND payment_status='succeeded'`) — a single conditional `UPDATE`. If any provisioning path bypasses the lock (recipe/self-provisioned triggers at `contracts.rs:748-749` call `try_trigger_cloud_provisioning` without re-checking payment), a VM can be provisioned against an unpaid contract. **Scenario:** lost webhook + provider manually accepts → resource provisioned → refund-owed on a payment that never landed.
- **Root cause:** application-level guard, not a constraint. Same fragility class as the #410 guard.

### 2.2 🟠 `total_released_e9s` can exceed `payment_amount_e9s` (ICPay payout path)

- `process_releases_once` (`payment_release_service.rs:90-92`) computes each release as `payment_amount_e9s * period_duration / total_duration` and adds it to a running total (`:127-128`). There is **no check** that `new_total_released <= payment_amount_e9s`, and **no DB CHECK** (`001_schema.sql:516`).
- **Scenario:** clock skew, a doubled cycle after an outage, or a `start_timestamp_ns`/`end_timestamp_ns` edit can make `period_duration` sum exceed `total_duration`. `total_released_e9s` silently exceeds `payment_amount_e9s`. `calculate_net_refund_e9s` then returns a clamped-but-wrong refund (`payment.rs:189`: `saturating_sub` floors at 0), so **the customer is under-refunded** (the platform keeps money it shouldn't) — and the provider was over-paid relative to what was net-collected.

### 2.3 🟠 Refund-vs-release TOCTOU race (ICPay)

- `calculate_net_refund_e9s` (`payment.rs:174-190`) *reads* `total_released_e9s` then later the cancel path writes `refund_amount_e9s` — in **separate transactions**, while the daily release loop (`payment_release_service.rs`) can bump `total_released_e9s` in between.
- **Scenario:** cancel reads `total_released=30` of `payment=100`, computes refund on that basis; release loop adds another `20` to `total_released` concurrently; the customer is refunded against stale data. Net: platform has released `50` and refunded as if only `30` was released → payout+refund > collected. The only protection today is that this is ICPay-only and ICPay refunds are best-effort (`payment.rs:288-295` logs and continues).

### 2.4 🟡 `reject_contract` refunds the full `payment_amount_e9s` without subtracting released funds

- `reject_contract` (`rental.rs:400`): `full_refund = contract.payment_amount_e9s`. It does **not** call `calculate_net_refund_e9s`. Acceptable *only* because reject is restricted to `requested/pending/accepted` (`rental.rs:389-394`) where releases shouldn't have started — but `get_contracts_for_release` selects `status IN ('active','provisioned')` (`payment.rs:324`), so a contract that briefly hit `active` and got one daily release, then gets rejected via a buggy state machine, would refund 100% on top of the release. Fragile invariant enforced only by status-transition reachability, not the DB.

### 2.5 🟡 Fee-erosion: the markup cannot cover Stripe fees on small rentals

US Stripe online-card rate = **2.9% + $0.30** ([stripe.com/pricing](https://stripe.com/pricing)). The markup is applied by pricing it into the offering's `monthly_price` (there is **no platform-fee layer** in code — `rental.rs:125-130`). Break-even where markup covers fees, with provider cost `H` and markup fraction `m`:

```
net = m·H − (0.029·H·(1+m) + 0.30) = 0
→ H = 0.30 / (0.971·m − 0.029)
```

| Markup `m` | Break-even `H` | Below this, every rental LOSES money |
|---|---|---|
| 10% | **$4.41** | < $4.41 |
| 7.5% | $6.97 | < $6.97 |
| 5% (floor) | **$15.35** | < $15.35 |

A cheap hourly compute rental billed at, say, $2 loses ~$0.27 per transaction at a 10% markup. Worse, **partial refunds refund the percentage fee but not the $0.30** ([Stripe: refunds return variable fees, not the fixed per-transaction fee](https://docs.stripe.com/refunds) — *flagged*: the fixed-fee-on-partial-refund behavior is widely described but I did not find a single docs.stripe.com sentence stating it verbatim; treat the direction as reliable, the exact citation as **unverified**, §7). The fix is structural: **one top-up charge funds many rentals**, amortizing the $0.30 (§5).

### 2.6 🟡 `update_icpay_payment_status` accepts any string

- `payment.rs:229-243` binds `new_status` straight into `payment_status` with **no allow-list**. A compromised/replayed ICPay webhook could set `payment_status` to an arbitrary value, confusing every downstream guard that compares `== "succeeded"`.

### 2.7 🟡 `cancel_contract`/`reject_contract` refund records succeed even when Stripe returns no refund id

- `issue_audited_refund` returns `None` when `stripe_client` is not configured (`refund_audit.rs:204-206`) and the cancel path treats that as success, writing `payment_status='refunded'` (`rental.rs:694-708`). In a deploy with `STRIPE_SECRET_KEY` unset, **a contract can be marked refunded with no money returned to the customer** — the platform keeps the funds and the user sees "refunded". Loud only via a `WARN` log (`rental.rs:664-668`). Combined with Stripe being optional at boot (§1.1), this is a real misconfiguration footgun.

### 2.8 🟠 No dispute "funds already withdrawn" guard on the refund side

- `handle_dispute_funds_withdrawn` (`webhooks.rs:1048`) only *records* the withdrawal and pages ops. `process_dispute_lost_refund` (`dispute.rs:486-497`) recomputes the prorated refund from `payment_amount_e9s` **without consulting `funds_withdrawn_at_ns` or whether Stripe already pulled the full disputed amount**. If a partial prorated refund is issued *after* Stripe already clawed the full disputed charge, the platform pays the customer twice for the same window. (Mitigated today because dispute-lost uses `dispute:{id}` idempotency, but that only prevents *re-issuing the same refund*, not issuing a refund Stripe didn't intend.)

### 2.9 🟡 Self-rental / ICPay "succeeded immediately" can provision before money is confirmed

- Self-rental sets `payment_status='succeeded'` and `payment_amount_e9s=0` (`rental.rs:125,155`) — free, fine. But ICPay also sets `succeeded` immediately at *request* time (`rental.rs:155`), and the real confirmation arrives later via webhook (`payment.rs:211`). Between request and webhook confirmation, `payment_status='succeeded'` is asserted **before the ICPay transaction is settled** — so the provisioning gate (`provisioning.rs:899`) can release a VM on an ICPay payment that later fails. There is no "pending until confirmed" ICPay state in the same way Stripe has `pending→succeeded`.

---

## 3. Stripe compliance facts (web-verified)

### 3.1 ✅ Aggregation / money-transmission prohibition → Connect is mandatory for marketplaces

> "If you are a marketplace using Stripe but not processing payments through Stripe Connect, you may be in violation of Stripe's Terms of Service for **'aggregation.'**"
> — [support.stripe.com: Restrictions for marketplaces not using Stripe Connect](https://support.stripe.com/questions/restrictions-for-marketplaces-not-using-stripe-connect)

"Personal or peer-to-peer money transmission" is on [Stripe's Prohibited/Restricted Businesses list](https://stripe.com/en-br/legal/restricted-businesses). Collecting from user A and paying third-party provider B out of your own merchant balance, outside of Connect, is exactly the aggregation pattern Stripe forbids. **Connect is the compliant path for any flow where the platform moves money to third parties.**

### 3.2 ✅ Connect account types — but **Standard/Express/Custom are deprecated for new integrations**

> "The information on this page applies only to platforms that already use legacy connected account types… If you're setting up a new Connect platform, or your integration uses the Accounts v2 API, see the Interactive platform guide."
> "Stripe recommends that you use **controller properties** instead of account types."
> — [docs.stripe.com: Connected account types](https://docs.stripe.com/connect/accounts)

For **legacy** reference (relevant if Decent Cloud intentionally picks a legacy type):

| | Standard | Express | Custom |
|---|---|---|---|
| Integration effort | Lowest | Low | Significantly higher |
| Onboarding / KYC | Stripe | Stripe | Platform (or Stripe Onboarding) |
| Dispute/refund liability | Mixed | Platform | Platform |
| Charge types | Direct only | Destination, Separate+transfers, Direct | Destination, Separate+transfers, Direct |
| Extra cost | — | Yes | Yes |

([stripe.com/connect/pricing](https://stripe.com/connect/pricing): Express/Custom add **0.25% / min 25¢ per payout** + **$2/active user/month** in some configurations; **flagged** — exact current fees should be re-checked on the pricing page at build time, §7.) For a **new** platform, build against the Accounts v2 API + controller properties, not the deprecated type labels.

### 3.3 ✅ `application_fee_amount` / destination charges — how the platform takes its cut

- **Destination charges** ([docs.stripe.com/connect/destination-charges](https://docs.stripe.com/connect/destination-charges)): charge on the platform, `transfer_data[destination]` auto-moves funds to the connected account, `application_fee_amount` is the platform's cut (capped at the charge total), Stripe fees deducted from the platform's portion.
- **Refunds on destination charges:** "by default the destination account keeps the funds… leaving the platform account to cover the negative balance from the refund. To pull back the funds… set `reverse_transfer=true`." Use `refund_application_fee=true` to also return the fee. → **A destination-charge refund that forgets `reverse_transfer=true` makes the platform eat the refund out of its own balance.**
- **Async-payment safety (advantage of destination over separate):** "If the async payment fails, Stripe automatically reverses the transfer." (Destination charges doc.)

### 3.4 ✅ Separate charges and transfers — the escrow primitive (recommended for Phase 2)

[docs.stripe.com/connect/separate-charges-and-transfers](https://docs.stripe.com/connect/separate-charges-and-transfers):

- Charge on platform account now; **transfer to the connected account later** (e.g., on rental completion). This is the escrow/fund-hold model.
- **"We recommend using separate charges and transfers only when you're responsible for negative balances of your connected accounts."** — the platform is on the hook.
- **`source_transaction`** ties a transfer to a charge. **"the amount of the transfer must not exceed the amount of the source charge"** and **"the sum of the transfers doesn't exceed the source charge."** → This is the *Stripe-enforced* `payout ≤ collected` invariant. Use it.
- **Refunds do NOT reverse transfers automatically:** "refunding a charge has no impact on any associated transfers… reconcile… by reversing transfers." → **Refund-before-transfer** is the safe order; once funds leave the connected account's available balance, `TransferReversal` fails ("only possible to reverse a transfer if the connected account's available balance is greater than the reversal amount").
- **Async-payment hazard (disadvantage vs destination):** "If you create a transfer and the payment subsequently fails, your platform's balance is debited for the transfer amount." → **never transfer before `charge.succeeded`.**
- **Funds segregation** ([docs.stripe.com/connect/funds-segregation](https://docs.stripe.com/connect/funds-segregation)): a *private-preview* feature that holds each payment's funds in a protected state before transfer, preventing them from being spent on unrelated platform operations. Clean escrow accounting — but **gated; requires a Stripe account manager**. Do not build a hard dependency on it without access.

### 3.5 Fund-hold / "escrow" duration limits

- Stripe's public Connect docs **do not publish a single hard cap** on how long separate-charges funds may sit before transfer. The closest concrete figure is the negative-balance rule: **"When a connected account holds a negative balance amount for 180 days, Stripe transfers a portion of your balance to zero out that account's balance."** ([docs.stripe.com/connect/account-balances](https://docs.stripe.com/connect/account-balances)).
- The legacy transfers doc notes "Escrow has a…" in a truncation I could not fully resolve ([docs.stripe.com/connect/legacy-transfers](https://docs.stripe.com/connect/legacy-transfers)) — **flagged unverified**, §7. Practical guidance: **settle to providers on a defined cadence (daily/weekly), do not hold indefinitely.** Stripe may, at its discretion, treat long-held balances as a money-transmission concern. Treat "escrow" here as *short-hold settlement timing*, not indefinite custody.

### 3.6 ✅ E-money vs stored-value — the make-or-break distinction for a pre-pay balance

This is a **financial-regulation** question, not purely a Stripe one. The safe boundary, consistent with Stripe's own design surface:

| Model | What it is | Regulation |
|---|---|---|
| **Stored value / gift-card** | Pre-paid balance redeemable **only for platform services** (compute), **never withdrawable to cash** | Generally **not** regulated e-money issuance in most jurisdictions (treated like store credit / gift cards); still subject to consumer-protection and escheatment rules |
| **E-money** | Pre-paid balance **withdrawable to cash / refundable to original payment method on demand** | **Regulated** money transmission / e-money issuance; requires licenses (MSB in the US, EMI in the EU) |

Stripe's primitives support the safe side: the **Customer Balance / Customer credit balance** ([docs.stripe.com/invoicing/customer/balance](https://docs.stripe.com/invoicing/customer/balance)) is a **bookkeeping credit applied to future invoices** — exactly a non-withdrawable stored-value ledger. Stripe also offers a **Customer Balance payment method** ([docs.stripe.com/payments/customer-balance](https://docs.stripe.com/payments/customer-balance)) where customers pre-fund via VBAN — that one moves real cash and must be treated more carefully.

**Recommendation for Decent Cloud:** keep the pre-pay balance *non-withdrawable to cash*. Refunds of unused balance should go back to the **original payment method within a bounded window** (e.g., 90 days), at the platform's discretion and net of fees — this is a refund of a purchase, not an on-demand withdrawal. Avoid "withdraw your balance to your bank anytime" framing; that is the e-money line.

> ⚠️ **This is a legal determination, not an engineering one.** I am reporting the widely-accepted boundary, not certifying Decent Cloud's compliance. Get a money-transmission opinion for every operating jurisdiction before shipping a balance (§7).

---

## 4. Phase boundary

### Phase 1 — what exists now (sole provider = owner = merchant of record)

- **Owner sells their own compute.** There is exactly one provider entity (the owner) setting offering prices that already embed the intended markup. No third party is paid by the platform.
- **Standard Stripe (no Connect) is compliant here:** the platform is the merchant of record collecting for its own service. No aggregation.
- **Provider "payout" is a non-event:** the owner keeps the platform balance; the ICPay `payment_releases` + admin payout (`admin.rs:690`) paths are vestigial for the Stripe flow.
- **What exists in code:** per-rental Hosted Checkout (`contracts.rs:104`), `checkout.session.completed` webhook (`webhooks.rs:220`), prorated/full refunds (`rental.rs`, `dispute.rs`), dispute handling (`dispute.rs`), the #409/#410 timeout guards, `refund_audit` idempotency.
- **What Phase 1 still needs to be cost-safe (no new business model, just invariants):** §5.2 DB constraints; gate `update_contract_status` on `payment_status='succeeded'` (or rely solely on the locked provisioning path and forbid unlocked provisioning); bound `total_released_e9s`; make `payment_status` an enum; make Stripe **required at boot in production**.

### Phase 2 — third-party providers join (Stripe Connect becomes mandatory)

- **Trigger:** any provider other than the owner receives funds derived from a user payment. The moment the platform pays a third party from collected user funds, §3.1 applies → **Connect is mandatory.**
- **Required and currently absent:**
  - Stripe Connect onboarding for providers (Accounts v2 API + controller properties — not legacy Express/Custom labels).
  - Charge type decision: **destination charges** (simpler, auto-reverses on async failure, but funds leave immediately so escrow = "we hold the application_fee and the timing is the transfer") **vs separate charges and transfers** (true escrow: hold full amount, transfer on completion). For a rental-completion settlement model, **separate charges and transfers** is the better fit.
  - `application_fee_amount` (destination) **or** transfer-amount-reduction (separate) to take the platform cut.
  - `source_transaction` on every transfer (Stripe-enforces `payout ≤ collected`).
  - Refund-before-transfer ordering; `TransferReversal` only works while the provider's balance can cover it.
  - Dispute handling already exists (`dispute.rs`) but must add **transfer reversal** on `charge.dispute.closed=lost`, not just a customer-side refund.

---

## 5. Cost-safety architecture proposal

The hard requirement is **architecturally impossible to go negative**. That means *invariants enforced by the database*, not by code paths that can be forgotten. Everything below is specified as DB-level constraints / preconditions.

### 5.1 The money model

```
user_pays = provider_cost + platform_fee
platform_fee = max( markup_pct * provider_cost , payment_fees )
escrow      = hold user_pays on platform until rental completes
payout      = provider_cost  (to provider, Phase 2 via Connect transfer)
platform_keeps = user_pays − payout − stripe_fees_actual
```

### 5.2 Hard invariants (DB-enforced — the non-negotiables)

These belong in a migration as `CHECK` constraints, generated columns, and conditional triggers. Cite the current absence (§1.3) as the gap.

1. **No provision without confirmed payment.**
   - `contract_sign_requests`: add `CHECK (status NOT IN ('provisioned','active') OR payment_status='succeeded')`. (Excludes self-rental/free via `payment_amount_e9s=0`.)
   - Make `payment_status` an `ENUM('pending','succeeded','refunded','disputed','failed')` (today free text — `001_schema.sql:511`).
   - Gate `update_contract_status` (provider accept) on `payment_status='succeeded'` in the same transaction that flips status, not in a separate read.

2. **Collected ≥ released + refunded (the core no-negative invariant).**
   - Add a generated/persisted column `net_holdable_e9s = payment_amount_e9s − total_released_e9s − COALESCE(refund_amount_e9s,0)`.
   - `CHECK (total_released_e9s + COALESCE(refund_amount_e9s,0) <= payment_amount_e9s)`.
   - Enforce via a trigger that does `total_released_e9s = total_released_e9s + NEW.amount` inside `UPDATE … WHERE new_total <= payment_amount_e9s RETURNING …` (0 rows = refused, like the provisioning lock pattern at `provisioning.rs:898`).

3. **Refund ≤ available balance, and refund ≤ original payment.**
   - `calculate_net_refund_e9s` (`payment.rs:174`) already does `gross − already_released`; promote that to a DB precondition so a buggy caller can't bypass it. `reject_contract` (`rental.rs:400`) must use the same net calculation, not the raw `payment_amount_e9s`.

4. **Payout ≤ collected (Phase 2).**
   - Rely on Stripe's `source_transaction` cap ("sum of transfers doesn't exceed the source charge" — §3.4). Mirror it locally: `CHECK` that `SUM(payment_releases.amount_e9s) <= payment_amount_e9s` per contract, enforced by the trigger above.
   - Never call `Transfer` before `charge.succeeded` (async-failure debit hazard, §3.4).

5. **Refund-before-transfer / reverse-on-dispute.**
   - Phase 2: a refund or lost dispute must issue `reverse_transfer=true` (`refund_application_fee=true`) on destination charges, or a `TransferReversal` on separate charges — **before** relying on the connected account's balance.

6. **Fee floor honored at top-up.**
   - `platform_fee = max(markup_pct * provider_cost, stripe_fee_estimate)` so the fixed $0.30 is always covered. Reject top-ups below the break-even `H` for the chosen markup (§2.5).

### 5.3 Pre-pay balance coexisting with / replacing per-rental Checkout

- **Replace, don't coexist, for the rental flow.** Per-rental redirect Checkout is incompatible with a balance model: it charges the card fresh each time (paying $0.30 each time, §2.5) and cannot escrow per-rental.
- **Top-up = one Stripe charge → credits `account_balance` (stored-value, §3.6).** Amortizes the $0.30 across all rentals funded by that top-up. E.g., a $50 top-up = one $0.30 fee; ten $5 rentals then cost only ~2.9% each, no fixed fee.
- **Rental = debit `account_balance`** inside the same transaction that sets `payment_status='succeeded'` and reserves the resource. The §5.2 invariant "collected ≥ released + refunded" now spans *balance debits across many rentals* — express it as: **`SUM(ledger.debits) for user ≤ SUM(ledger.credits from successful top-ups) − fees`**, enforced by a `ledger` table with a `running_balance` generated by a trigger that refuses to go negative (`CHECK (balance >= 0)` on the materialized balance, updated atomically).
- **Escrow with pre-pay:** the balance debit at rental start *is* the collection. Hold it as "in escrow" (a sub-ledger or `escrow_e9s` column) until the rental completes; on completion, move it to `released_e9s` (Phase 2: triggers the Stripe transfer). On cancel/dispute, move unused escrow back to `available_balance` (a refund-to-balance, not a cash refund — avoids e-money). Cash refunds only for the original top-up within a bounded window.
- **Settlement cadence:** release escrow to providers daily/weekly (§3.5 — don't hold indefinitely). Capped by `source_transaction` so the platform never fronts money.

### 5.4 Summary table — what changes per phase

| Concern | Phase 1 (now) | Phase 1 fix needed | Phase 2 add |
|---|---|---|---|
| Collection | Per-rental Checkout | → pre-pay top-up balance | unchanged |
| Provider payout | none (owner keeps) | none | Connect separate-charges transfer w/ `source_transaction` |
| Platform cut | baked into price | keep (or move to `platform_fee` column) | `application_fee_amount` / transfer reduction |
| Escrow | none | balance sub-ledger hold | Stripe-side hold via separate charges (+ funds segregation if granted) |
| No-negative invariant | app code only | §5.2 DB constraints | + `source_transaction` Stripe cap |
| Compliance | standard Stripe OK | confirm stored-value stance w/ lawyer | Connect mandatory (§3.1) |

---

## 6. Risk register & build sequence

### Risks

| # | Risk | Impact | Mitigation |
|---|---|---|---|
| R1 | Provider accept bypasses `payment_status` (`rental.rs:273`) | 🔴 provision without payment | §5.2 invariant #1 |
| R2 | `total_released_e9s` unbounded (`001_schema.sql:516`) | 🟠 over-payout | §5.2 invariant #2 trigger |
| R3 | Refund-vs-release TOCTOU (`payment.rs:174` + release loop) | 🟠 double-spend of one payment | single-transaction atomic debit/credit |
| R4 | Fee-erosion on small rentals (§2.5) | 🟡 loss per rental | pre-pay top-ups; fee floor |
| R5 | Stripe optional at boot; "refunded" with no refund (§2.7) | 🟡 user harm, audit lies | require `STRIPE_SECRET_KEY` in prod; fail refund if client missing |
| R6 | Phase 2 without Connect | 🔴 TOS termination, frozen funds | Connect is a hard gate before any third-party provider |
| R7 | Long fund holds flagged as money transmission (§3.5) | 🟠 compliance | defined settlement cadence; no indefinite escrow |
| R8 | "Balance withdrawable to cash" → e-money | 🔴 licensing required | keep balance non-withdrawable; bounded refund window |
| R9 | Dispute double-refund (§2.8) | 🟠 over-reimburse | consult `funds_withdrawn_at_ns` before refund; Phase 2 reverse transfer |
| R10 | ICPay `payment_status` set to arbitrary string (§2.6) | 🟡 guard confusion | enum + allow-list |

### Recommended build sequence

**Phase 1A — close the holes without changing the business model (smallest, safest first):**
1. Make `payment_status` an `ENUM`; allow-list `update_icpay_payment_status` (R10).
2. Add `CHECK (total_released_e9s + refund_amount_e9s <= payment_amount_e9s)` + atomic release trigger (R2, R3).
3. Gate `update_contract_status` (provider transitions) on `payment_status='succeeded'` (R1).
4. Require `STRIPE_SECRET_KEY`/`STRIPE_WEBHOOK_SECRET` in prod boot; fail refund loudly if no client (R5).
5. Make `reject_contract` use `calculate_net_refund_e9s` (R3 variant).

**Phase 1B — pre-pay balance (the structural fee fix):**
6. Introduce `account_balance`/`ledger` tables with a `CHECK (balance >= 0)` trigger-enforced balance.
7. Top-up flow = one Stripe charge → credit balance (stored-value, non-withdrawable).
8. Rental = atomic balance debit + escrow hold; settlement cadence (R4, R7).
9. Keep cash-refund window bounded; lawyer review for stored-value posture (R8).

**Phase 2 — marketplace (only after Connect is live):**
10. Connect onboarding (Accounts v2 API + controller properties).
11. Switch rental collection to separate charges and transfers (escrow); every transfer uses `source_transaction`.
12. Wire `reverse_transfer`/`refund_application_fee` on refunds and lost disputes.
13. Re-confirm Express-equivalent fees on [stripe.com/connect/pricing](https://stripe.com/connect/pricing) at build time.

---

## 7. What I could NOT verify

- **§2.5 / fixed fee on partial refunds:** Stripe is widely understood to refund the *variable* fee but **not** the $0.30 fixed fee on partial refunds. I could not locate a single sentence on `docs.stripe.com/refunds` stating this verbatim. Direction is reliable; **exact citation unverified** — confirm before relying on the break-even math in a contract.
- **§3.5 / hard escrow hold cap:** the public Connect docs do not state a maximum number of days separate-charges funds may be held pre-transfer. The 180-day figure is about *negative* balances on connected accounts, not platform holds. The `legacy-transfers` doc references "Escrow has a…" but truncated in my fetch. **No verified hard cap; treat "escrow" as short-hold settlement, not indefinite custody, and ask Stripe directly.**
- **§3.2 / current Express/Custom fees:** [stripe.com/connect/pricing](https://stripe.com/connect/pricing) is the source of truth but I cited the 0.25%-per-payout + $2/active-user figures from secondary summaries; **re-check the live pricing page before budgeting.**
- **§3.6 / e-money vs stored-value for Decent Cloud specifically:** this is a legal opinion, not an engineering fact. I report the *commonly accepted boundary* (non-withdrawable service credit = stored-value; cash-withdrawable = e-money). **A money-transmission lawyer must confirm for each operating jurisdiction.** I did not find Stripe publishing a definitive "your balance is/isn't e-money" ruling for arbitrary custom balances.
- **§2.1 / which exact provisioning paths bypass the lock:** I confirmed `acquire_provisioning_lock` (`provisioning.rs:898`) is the hard gate and that `try_trigger_cloud_provisioning` is called without a re-check after auto-accept, but a full enumeration of every provisioning entry point (recipe vs cloud vs self-provisioned) was not exhaustively traced — flag as needing a focused audit before relying on the lock as the *sole* gate.
- **Accounts v2 API specifics:** I confirmed legacy account types are deprecated and that "controller properties" is the new path, but did not deep-read the Accounts v2 / Interactive platform guide docs; treat the Phase 2 onboarding design as needing its own doc-verification pass.

---

## Appendix A — key file:line index

- Boot / Stripe optional: `api/src/main.rs:939-944`
- Webhook handler: `api/src/openapi/webhooks.rs:161` (paid transition `:274-295`)
- Rental create + amount: `api/src/database/contracts/rental.rs:8`, `rental.rs:125-130`, `rental.rs:155`
- Checkout session create: `api/src/openapi/contracts.rs:104`, `api/src/stripe_client.rs:44`
- Paid update (webhook): `api/src/database/contracts/payment.rs:11-33`
- Cancel refund (net): `rental.rs:570`, `payment.rs:174-190`
- Reject refund (full): `rental.rs:366-400`
- Dispute refund: `api/src/database/contracts/dispute.rs:459`
- Refund idempotency/audit: `api/src/refund.rs`, `api/src/database/refund_audit.rs:176`
- Provisioning gate: `api/src/database/contracts/provisioning.rs:898-899`; provider accept (no payment check): `rental.rs:273`
- Release service: `api/src/payment_release_service.rs:42` (ICPay-only); release query `payment.rs:307-331`
- Admin payout: `api/src/openapi/admin.rs:690` (ICPay-only)
- Timeout guards (#409/#410): `api/src/timeout_cleanup_service.rs`; scan guards `api/src/database/contracts/timeouts.rs:185-195`
- Schema: `api/migrations_pg/001_schema.sql:456-535` (contract), `:1081-1093` (releases); `044_refund_audit.sql`; `042_rename_payment_intent_to_session.sql`
- "Balance" is ICP not fiat: `api/src/database/tokens.rs:69`, `api/src/openapi/transfers.rs:70`
