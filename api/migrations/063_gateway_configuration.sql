-- Gateway configuration for DC-level reverse proxy (Traefik)
-- VMs get a unique gateway_slug for subdomain routing and port allocation

-- Add gateway columns to contract_sign_requests
ALTER TABLE contract_sign_requests ADD COLUMN gateway_slug TEXT;
ALTER TABLE contract_sign_requests ADD COLUMN gateway_ssh_port INTEGER;
ALTER TABLE contract_sign_requests ADD COLUMN gateway_port_range_start INTEGER;
ALTER TABLE contract_sign_requests ADD COLUMN gateway_port_range_end INTEGER;

-- Gateway slug must be unique when set
CREATE UNIQUE INDEX idx_gateway_slug ON contract_sign_requests(gateway_slug)
  WHERE gateway_slug IS NOT NULL;
