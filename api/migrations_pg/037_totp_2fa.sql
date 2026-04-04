-- TOTP two-factor authentication support (ticket #80)

ALTER TABLE accounts ADD COLUMN totp_secret TEXT;
ALTER TABLE accounts ADD COLUMN totp_enabled BOOLEAN NOT NULL DEFAULT FALSE;

CREATE TABLE totp_backup_codes (
    id BYTEA PRIMARY KEY DEFAULT gen_random_bytes(16),
    account_id BYTEA NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    code_hash BYTEA NOT NULL,
    created_at BIGINT NOT NULL,
    used_at BIGINT
);

CREATE INDEX idx_totp_backup_codes_account ON totp_backup_codes(account_id);

COMMENT ON COLUMN accounts.totp_secret IS 'AES-256-GCM encrypted TOTP secret (base64-encoded)';
COMMENT ON COLUMN accounts.totp_enabled IS 'Whether TOTP two-factor authentication is enabled for this account';
