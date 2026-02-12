-- Add gateway_subdomain column to store the full FQDN
-- Format: {slug}.{dc_id}.{gw_prefix}.{domain} (e.g., k7m2p4.a3x9f2b1.dev-gw.decent-cloud.org)
ALTER TABLE contract_sign_requests ADD COLUMN IF NOT EXISTS gateway_subdomain TEXT;
