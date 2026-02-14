-- Password reset request support
-- Allows users to request password resets that agents can pick up and execute

-- Add column to track password reset requests
ALTER TABLE contract_provisioning_details
    ADD COLUMN IF NOT EXISTS password_reset_requested_at_ns BIGINT;

-- Index for efficient querying of pending resets
CREATE INDEX IF NOT EXISTS idx_contract_provisioning_details_password_reset_pending
    ON contract_provisioning_details(password_reset_requested_at_ns)
    WHERE password_reset_requested_at_ns IS NOT NULL;

COMMENT ON COLUMN contract_provisioning_details.password_reset_requested_at_ns IS
    'Timestamp (nanoseconds) when user requested a password reset. Agent clears this after reset.';
