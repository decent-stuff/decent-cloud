-- Add resources JSONB column to provider_agent_status for hardware inventory
-- This enables auto-generation of offerings based on agent capabilities

ALTER TABLE provider_agent_status
ADD COLUMN resources JSONB;

-- Index for querying agents with GPU capability
CREATE INDEX idx_agent_status_has_gpu
ON provider_agent_status ((resources->'gpuDevices'))
WHERE resources IS NOT NULL AND jsonb_array_length(resources->'gpuDevices') > 0;

COMMENT ON COLUMN provider_agent_status.resources IS 'Hardware resource inventory (CPU, memory, storage, GPU, templates) reported by agent';
