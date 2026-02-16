-- Migration 017: Add error_message and gateway_subdomain to cloud_resources
-- error_message: stores provisioning/termination error details so users can see why a resource failed.
-- gateway_subdomain: full DNS FQDN (e.g., "abc123.hz-nbg1.dev-gw.decent-cloud.org") for direct lookup.

ALTER TABLE cloud_resources ADD COLUMN error_message TEXT;
ALTER TABLE cloud_resources ADD COLUMN gateway_subdomain TEXT;

COMMENT ON COLUMN cloud_resources.error_message IS 'Error details when status is failed — visible to users for self-service debugging';
COMMENT ON COLUMN cloud_resources.gateway_subdomain IS 'Full DNS FQDN for this resource (e.g., slug.hz-location.gw.decent-cloud.org)';
