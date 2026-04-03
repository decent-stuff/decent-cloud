-- SSH key rotation support
-- Allows users to request SSH key rotation that agents can pick up and execute

-- Add column to track SSH key rotation requests
ALTER TABLE contract_provisioning_details
    ADD COLUMN IF NOT EXISTS ssh_key_rotation_requested_at_ns BIGINT;

-- Index for efficient querying of pending rotations
CREATE INDEX IF NOT EXISTS idx_contract_provisioning_details_ssh_key_rotation_pending
    ON contract_provisioning_details(ssh_key_rotation_requested_at_ns)
    WHERE ssh_key_rotation_requested_at_ns IS NOT NULL;

COMMENT ON COLUMN contract_provisioning_details.ssh_key_rotation_requested_at_ns IS
    'Timestamp (nanoseconds) when user requested an SSH key rotation. Agent clears this after injecting the new key.';
