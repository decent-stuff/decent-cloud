-- Migration: Add template support for provider offerings
-- This enables per-offering template selection for instant provisioning
-- Part of "Pre-configured Offering Templates" feature

-- Add template_name column to provider_offerings
-- This is a human-readable name like "ubuntu-22.04" that providers can set
ALTER TABLE provider_offerings
ADD COLUMN template_name TEXT;

-- Add index for filtering by template
CREATE INDEX idx_provider_offerings_template_name
ON provider_offerings(template_name)
WHERE template_name IS NOT NULL;

-- Comment for documentation
COMMENT ON COLUMN provider_offerings.template_name IS
'Human-readable template name (e.g. ubuntu-22.04, debian-12) that maps to a VM template in the provisioner';

-- Note: The actual template VMID or ID will be stored in the existing provisioner_config JSON field
-- Format: {"template_vmid": 9000} for Proxmox or similar for other provisioners