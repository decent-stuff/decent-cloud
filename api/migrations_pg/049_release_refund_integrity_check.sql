-- R2/R3 / A3: DB-enforced "refunded <= payment".
--
-- refund_amount_e9s had no upper bound relative to payment_amount_e9s; a buggy
-- caller could make the platform refund more than it collected. The code paths
-- now use the shared prorated refund calc; this CHECK is the un-bypassable
-- backstop that refunded can never exceed payment. (Stripe-only: no funds are
-- ever pre-released to providers, so there is no `total_released_e9s` to
-- account for — refund is bounded purely by the collected payment.)
-- COALESCE handles the NULLable refund_amount_e9s.
ALTER TABLE contract_sign_requests
    ADD CONSTRAINT release_refund_not_exceed_payment
    CHECK (
        COALESCE(refund_amount_e9s, 0) <= payment_amount_e9s
    );
