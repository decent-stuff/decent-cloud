-- Migration 014: Cloud accounts and resources for self-provisioning
-- Allows any user to connect cloud accounts (Hetzner, Proxmox) and self-provision resources

-- Cloud account connections (Hetzner tokens, Proxmox API tokens, etc.)
CREATE TABLE cloud_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id BYTEA NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    backend_type TEXT NOT NULL CHECK (backend_type IN ('hetzner', 'proxmox_api')),
    name TEXT NOT NULL,
    credentials_encrypted TEXT NOT NULL,
    
    -- Backend-specific config (JSON)
    config TEXT,
    
    -- Validation status
    is_valid BOOLEAN NOT NULL DEFAULT TRUE,
    last_validated_at TIMESTAMPTZ,
    validation_error TEXT,
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT unique_account_backend_name UNIQUE (account_id, backend_type, name)
);

CREATE INDEX idx_cloud_accounts_account ON cloud_accounts(account_id);
CREATE INDEX idx_cloud_accounts_backend ON cloud_accounts(backend_type);

-- Self-provisioned resources
CREATE TABLE cloud_resources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    cloud_account_id UUID NOT NULL REFERENCES cloud_accounts(id) ON DELETE CASCADE,
    
    -- Backend identifiers
    external_id TEXT NOT NULL,  -- Backend's server/VM ID
    
    -- Resource details
    name TEXT NOT NULL,
    server_type TEXT NOT NULL,
    location TEXT NOT NULL,
    image TEXT NOT NULL,
    ssh_pubkey TEXT NOT NULL,           -- SSH public key for VM access
    status TEXT NOT NULL DEFAULT 'provisioning' CHECK (status IN ('provisioning', 'running', 'stopped', 'deleting', 'failed')),
    
    -- Connection info
    public_ip TEXT,
    ssh_port INTEGER DEFAULT 22,
    ssh_username TEXT DEFAULT 'root',
    
    -- Gateway routing (same pattern as dc-agent contracts)
    gateway_slug TEXT UNIQUE,              -- e.g., "k7m2p4"
    gateway_ssh_port INTEGER,              -- e.g., 20001
    gateway_port_range_start INTEGER,      -- e.g., 20002
    gateway_port_range_end INTEGER,        -- e.g., 20011
    
    -- Link to marketplace offering (if listed)
    offering_id BIGINT REFERENCES provider_offerings(id) ON DELETE SET NULL,
    listing_mode TEXT NOT NULL DEFAULT 'personal' CHECK (listing_mode IN ('personal', 'marketplace')),
    
    -- Provisioning lock (prevents concurrent operations)
    provisioning_locked_at TIMESTAMPTZ,
    provisioning_locked_by TEXT,  -- API server instance ID
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    terminated_at TIMESTAMPTZ,
    
    CONSTRAINT unique_external_id UNIQUE (cloud_account_id, external_id)
);

CREATE INDEX idx_cloud_resources_account ON cloud_resources(cloud_account_id);
CREATE INDEX idx_cloud_resources_status ON cloud_resources(status);
CREATE INDEX idx_cloud_resources_offering ON cloud_resources(offering_id);
CREATE INDEX idx_cloud_resources_gateway_slug ON cloud_resources(gateway_slug);
CREATE INDEX idx_cloud_resources_provisioning_lock ON cloud_resources(provisioning_locked_at) WHERE provisioning_locked_at IS NOT NULL;

-- Comments for documentation
COMMENT ON TABLE cloud_accounts IS 'Cloud account connections for self-provisioning (Hetzner, Proxmox API)';
COMMENT ON TABLE cloud_resources IS 'Self-provisioned cloud resources with optional marketplace listing';
COMMENT ON COLUMN cloud_accounts.credentials_encrypted IS 'AES-256-GCM encrypted credentials (see server_credential_encryption.rs)';
COMMENT ON COLUMN cloud_resources.gateway_slug IS 'Unique subdomain slug for gateway routing (e.g., "k7m2p4")';
COMMENT ON COLUMN cloud_resources.listing_mode IS 'personal: self-use only, marketplace: available for rent';
