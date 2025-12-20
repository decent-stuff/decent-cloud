-- Per-offering provisioner configuration
-- Allows providers to specify different provisioner types per offering
-- NULL values = use agent's default provisioner (backward compatible)

ALTER TABLE provider_offerings ADD COLUMN provisioner_type TEXT;
ALTER TABLE provider_offerings ADD COLUMN provisioner_config TEXT;
