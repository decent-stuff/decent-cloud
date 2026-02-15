-- Migration 016: Link cloud_resources to contracts for recipe provisioning
-- When a recipe contract is accepted, a cloud_resource is created and linked to it.
-- The provisioning service picks it up, creates the VM, and executes the recipe script.

-- Link cloud_resource to the marketplace contract that triggered it (NULL for personal resources)
ALTER TABLE cloud_resources ADD COLUMN contract_id BYTEA REFERENCES contract_sign_requests(contract_id);

-- Snapshot of the recipe script from the offering at contract creation time
ALTER TABLE cloud_resources ADD COLUMN post_provision_script TEXT;

-- Fast lookup of cloud_resource by contract_id
CREATE INDEX idx_cloud_resources_contract ON cloud_resources(contract_id) WHERE contract_id IS NOT NULL;

-- Add 'deleted' to status check (terminated resources)
ALTER TABLE cloud_resources DROP CONSTRAINT IF EXISTS cloud_resources_status_check;
ALTER TABLE cloud_resources ADD CONSTRAINT cloud_resources_status_check
    CHECK (status IN ('provisioning', 'running', 'stopped', 'deleting', 'deleted', 'failed'));

COMMENT ON COLUMN cloud_resources.contract_id IS 'Links to the marketplace contract that triggered this resource (NULL for personal self-provisioned resources)';
COMMENT ON COLUMN cloud_resources.post_provision_script IS 'Snapshotted recipe script from the offering, executed via SSH after VM creation';
