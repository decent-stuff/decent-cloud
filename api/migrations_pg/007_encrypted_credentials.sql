-- Encrypted credentials support (Phase 7)
-- Adds encrypted credentials storage and expiration for auto-cleanup

-- Add encrypted credentials column (replaces unused instance_credentials)
-- Format: JSON containing version, ephemeral_pubkey, nonce, ciphertext
ALTER TABLE contract_provisioning_details
    ALTER COLUMN instance_credentials TYPE TEXT;

-- Add expiration timestamp for auto-deletion (7 days after provisioning)
ALTER TABLE contract_provisioning_details
    ADD COLUMN IF NOT EXISTS credentials_expires_at_ns BIGINT;

-- Index for efficient cleanup of expired credentials
CREATE INDEX IF NOT EXISTS idx_contract_provisioning_details_expires
    ON contract_provisioning_details(credentials_expires_at_ns)
    WHERE credentials_expires_at_ns IS NOT NULL;

-- Comment documenting the encrypted format
COMMENT ON COLUMN contract_provisioning_details.instance_credentials IS
    'Encrypted VM credentials (XChaCha20Poly1305). Format: JSON {version, ephemeral_pubkey, nonce, ciphertext}. Can only be decrypted by contract requester.';

COMMENT ON COLUMN contract_provisioning_details.credentials_expires_at_ns IS
    'Timestamp (nanoseconds) when credentials should be auto-deleted. Typically 7 days after provisioning.';
