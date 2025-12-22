-- Add pools and online agents for example provider
-- This ensures example offerings show up in marketplace with correct online status

-- Example provider pubkey (hex: "example-offering-provider-identifier")
-- Note: This migration must run after 053_agent_pools.sql which creates the tables

-- Register example provider if not exists
INSERT OR IGNORE INTO provider_registrations (pubkey, signature, created_at_ns)
VALUES (x'6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572', X'00', 0);

-- Create pools for regions where example offerings exist
INSERT OR IGNORE INTO agent_pools (pool_id, provider_pubkey, name, location, provisioner_type, created_at_ns)
VALUES
    ('example-na', x'6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572', 'Example NA Pool', 'na', 'manual', 0),
    ('example-europe', x'6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572', 'Example Europe Pool', 'europe', 'manual', 0),
    ('example-apac', x'6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572', 'Example APAC Pool', 'apac', 'manual', 0);

-- Create agent delegations for each pool
INSERT OR IGNORE INTO provider_agent_delegations (provider_pubkey, agent_pubkey, permissions, expires_at_ns, label, signature, created_at_ns, pool_id)
VALUES
    (x'6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572', x'6578616d706c652d6167656e742d6e612d0000000000000000000000000000000000', '[]', NULL, 'Example NA Agent', X'00', 0, 'example-na'),
    (x'6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572', x'6578616d706c652d6167656e742d65752d0000000000000000000000000000000000', '[]', NULL, 'Example EU Agent', X'00', 0, 'example-europe'),
    (x'6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572', x'6578616d706c652d6167656e742d61702d0000000000000000000000000000000000', '[]', NULL, 'Example APAC Agent', X'00', 0, 'example-apac');

-- Mark provider as online with recent heartbeat
-- Note: provider_agent_status tracks per-provider status, not per-agent
INSERT OR REPLACE INTO provider_agent_status (provider_pubkey, online, last_heartbeat_ns, updated_at_ns)
VALUES
    (x'6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572', 1, CAST(strftime('%s', 'now') AS INTEGER) * 1000000000, CAST(strftime('%s', 'now') AS INTEGER) * 1000000000);
