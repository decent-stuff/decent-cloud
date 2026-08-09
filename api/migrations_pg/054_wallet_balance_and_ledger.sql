-- Pre-pay wallet: stored-value balance + immutable ledger.
-- Design: docs/plans/2026-07-23-cost-safe-billing.md (Phase 1B, steps 6-9).
--
-- Model: non-withdrawable stored-value (store-credit / gift-card model, NOT
-- regulated e-money — see spec §3.6). Users top up via a single Stripe charge
-- that credits their USD balance; rentals debit that balance atomically. This
-- amortizes Stripe's fixed $0.30 fee across many rentals (per-rental Checkout
-- loses money below $4.41 at a 10% markup — spec §2.5).
--
-- Money is nano-USD throughout (balance_e9s / amount_e9s; 1 USD = 1e9 e9s),
-- matching the existing contract payment_amount_e9s convention.
--
-- Hard invariants (spec §5.2), DB-enforced:
--   * balance can NEVER go negative — CHECK constraint on wallet_balances.
--   * the atomic debit (UPDATE ... WHERE balance_e9s >= debit) rejects
--     overdrafts at the row level; the CHECK is the backstop.
--   * the ledger is append-only and records balance_after_e9s for every entry,
--     giving a complete, auditable running balance.

CREATE TABLE wallet_balances (
    pubkey TEXT PRIMARY KEY,
    balance_e9s BIGINT NOT NULL DEFAULT 0 CHECK (balance_e9s >= 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE wallet_ledger (
    id BIGSERIAL PRIMARY KEY,
    pubkey TEXT NOT NULL REFERENCES wallet_balances(pubkey),
    -- Signed: positive = credit (top-up / refund), negative = debit (rental).
    amount_e9s BIGINT NOT NULL,
    -- Running balance immediately AFTER this entry was applied (audit trail).
    balance_after_e9s BIGINT NOT NULL CHECK (balance_after_e9s >= 0),
    entry_type TEXT NOT NULL CHECK (entry_type IN ('topup', 'rental_debit', 'rental_refund', 'adjustment')),
    -- Optional reference: Stripe checkout session id, contract id, etc.
    reference TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_wallet_ledger_pubkey_created
    ON wallet_ledger (pubkey, created_at DESC);
