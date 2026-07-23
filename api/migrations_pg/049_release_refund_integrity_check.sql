-- R2/R3 / A3: DB-enforced "released + refunded <= payment".
--
-- total_released_e9s had no upper bound and refund_amount_e9s had no
-- relationship to it; a buggy caller (or a TOCTOU race between the daily
-- release loop and the cancel/refund path) could make the platform pay out
-- more than it collected. reject_contract also refunded the raw gross on top
-- of already-released funds. The code paths now use the shared net refund
-- calc and a conditional atomic release UPDATE; this CHECK is the
-- un-bypassable backstop that released + refunded can never exceed payment.
-- COALESCE handles the NULLable refund_amount_e9s / total_released_e9s.
ALTER TABLE contract_sign_requests
    ADD CONSTRAINT release_refund_not_exceed_payment
    CHECK (
        COALESCE(total_released_e9s, 0) + COALESCE(refund_amount_e9s, 0)
        <= payment_amount_e9s
    );
