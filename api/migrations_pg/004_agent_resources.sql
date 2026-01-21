-- Add resources JSONB column to provider_agent_status for hardware inventory
-- This enables auto-generation of offerings based on agent capabilities

ALTER TABLE provider_agent_status
ADD COLUMN resources JSONB;

COMMENT ON COLUMN provider_agent_status.resources IS 'Hardware resource inventory (CPU, memory, storage, GPU, templates) reported by agent';
