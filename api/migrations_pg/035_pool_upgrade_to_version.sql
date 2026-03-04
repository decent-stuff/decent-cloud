-- Add upgrade_to_version column to agent_pools for remote agent upgrades.
-- When set, the API heartbeat response includes this version so agents self-upgrade.
ALTER TABLE agent_pools ADD COLUMN upgrade_to_version TEXT;
