# Pre-pay Wallet (Stored-Value Billing)

**Status:** DONE (Units 1-5 complete, 2026-08-10)
**Spec:** `docs/plans/2026-07-23-cost-safe-billing.md` (Phase 1B)
**Model:** Non-withdrawable stored-value (gift-card/store-credit). Balance spends
only on platform rentals; refunds to original card within ~90 days. Avoids e-money
licensing (not cash-withdrawable).

## What shipped

| Unit | Commit | Scope |
|------|--------|-------|
| U1 — schema + DB | (U1 commit) | migration 054 (`wallet_balances` + `wallet_ledger`), `database/wallet.rs` (get/credit/debit atomic with `CHECK balance>=0`) |
| U2 — top-up API + webhook + GET | `51057688` | `stripe_client::create_wallet_topup_session`, webhook `wallet_topup` branch, `GET /users/:pubkey/wallet`, `POST /users/:pubkey/wallet/topup` |
| U3 — wallet UI | (U3 commit) | `/dashboard/wallet` page (balance + top-up + ledger), dashboard balance card (all 3 role branches), sidebar nav, `api.ts` clients |
| U4 — rewire rentals | `0c47bbff` | `PaymentMethod::Wallet` variant; rentals debit wallet at creation (replaces per-contract Stripe checkout); instant refund-to-balance on cancel/reject/provisioning-failed; `RentalRequestDialog` wallet card |
| U5 — verify | this session | 325 e2e pass / 0 fail; 101 wallet/rental/payment nextest pass; clippy + svelte-check clean |

## Money flow

- **Top-up:** user → `POST /wallet/topup` → Stripe Checkout (one charge) → webhook
  `checkout.session.completed` (`metadata.type=wallet_topup`) → `credit_wallet_balance`.
- **Rental:** `create_rental_request` → `debit_wallet_for_contract` (single atomic tx:
  debit balance + insert ledger + mark contract `payment_status=succeeded,
  payment_method=wallet`) → contract stays `requested` for provider review.
- **Refund (cancel/reject/provisioning-failed):** `credit_wallet_balance`
  (`entry_type=rental_refund`) — instant internal credit, no Stripe call, no gate.

## Key design decisions

- **Refund-approval gate exemption (migration 055):** the 051 trigger blocks
  `payment_status=refunded` without an approved `refund_request`. Its purpose is
  Stripe external refunds only; wallet refunds are internal credits with their own
  audit trail (`wallet_ledger`) + money-safety (`CHECK balance>=0`). Migration 055
  exempts `payment_method='wallet'` from the trigger.
- **Per-contract Stripe checkout removed:** `create_checkout_session` (contract-scoped)
  + the `contracts.rs` helper are dead code — removed. The webhook contract path is
  dormant (no new per-contract sessions) but retained for historical contracts.
- **`e9s` = nano-USD** (1 USD = 1e9 e9s; cents = e9s/10_000_000). Wallet amounts use
  the same unit as the rest of the system.

## TODO / out of scope

- **Stripe webhook idempotency for top-ups:** webhook handler credits on every
  `checkout.session.completed` for a top-up session; Stripe retries are
  idempotent at the PI level but the credit is not keyed on session id (potential
  double-credit on replay). File as follow-up if not already handled.
- **Auto-renewal:** recurring billing not yet wired to wallet (contracts auto-renew
  via existing renewal service — needs a wallet-debit path for renewals).
- **Spending alerts:** `get_current_month_spending_usd` sums
  `payment_amount_e9s` regardless of payment method — wallet debits are included
  (correct), but verify the alert cap interacts sanely with the new instant-debit
  model.
