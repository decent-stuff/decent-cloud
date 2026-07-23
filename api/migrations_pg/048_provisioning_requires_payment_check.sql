-- R1 / A2: DB-enforced "no provision without payment".
--
-- update_contract_status validated authorization and the status state machine
-- but never read payment_status, so a provider could drive
-- requested -> accepted -> provisioning -> provisioned/active on a contract
-- whose Stripe checkout never completed (payment_status='pending'). The only
-- existing gate was the later acquire_provisioning_lock conditional UPDATE.
--
-- This CHECK makes the invariant un-bypassable even via direct SQL: a contract
-- may only be Provisioned or Active when funds were collected
-- (payment_status='succeeded') OR it is free / self-rental
-- (payment_amount_e9s=0). The code-level gate in update_contract_status is the
-- loud, caller-visible check; this is the backstop. Existing active contracts
-- already satisfy this (they were gated through payment before reaching active).
ALTER TABLE contract_sign_requests
    ADD CONSTRAINT provisioning_requires_payment
    CHECK (
        status NOT IN ('provisioned', 'active')
        OR payment_status = 'succeeded'
        OR payment_amount_e9s = 0
    );
