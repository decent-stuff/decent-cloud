-- Idempotency for wallet top-ups: a Stripe checkout session may only credit
-- the wallet ONCE, even if Stripe replays the checkout.session.completed
-- webhook (Stripe uses at-least-once delivery).
--
-- Money-safety bug this closes: without a uniqueness guarantee, the wallet
-- credit path debited money a second time for the same session id on replay.
-- `credit_wallet_balance_idempotent` catches the unique_violation (SQLSTATE
-- 23505) raised here and returns an idempotent AlreadyProcessed result.
--
-- Partial index scoped to top-up entries with a non-NULL reference (the
-- checkout session id). Refunds (entry_type='rental_refund') and debits are
-- intentionally NOT covered: a refund's reference is the contract id, and a
-- contract may legitimately accrue multiple refund/adjustment rows; only
-- top-ups are keyed 1:1 to a single Stripe checkout session.
CREATE UNIQUE INDEX wallet_ledger_topup_reference_unique
    ON wallet_ledger (reference)
    WHERE entry_type = 'topup' AND reference IS NOT NULL;
