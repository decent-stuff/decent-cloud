-- Drop the demo/synthetic example-provider seed (PRODUCT-DIRECTION.md F2).
--
-- Migration 002_seed_data.sql seeds 10 demo offerings (plus a fake provider,
-- pools, delegations, status) under a placeholder pubkey — the readable ASCII
-- string "example-offering-provider-identifier". Those rows are doubly stale
-- (retired ICP currency, clearly-fake pubkey) yet they surfaced in the
-- marketplace and misled users. The product direction mandates an honest EMPTY
-- catalog.
--
-- 002 is intentionally left untouched: editing an already-applied migration
-- changes its checksum and breaks existing environments on next boot (sqlx
-- enforces checksums on applied migrations). Instead, this migration deletes
-- the demo rows AFTER 002 seeds them. Result:
--   * Fresh DBs: 002 seeds demos -> 053 deletes them -> 0 demos.
--   * Existing prod/stage: 053 deletes the lingering demos on next deploy.
--
-- The cleanup is GUARDED by the exact fake pubkey value, so it cannot match any
-- real provider (no real ed25519 public key is a 48-byte readable ASCII string).
-- Delete order respects FK constraints (mirrors the test helper
-- `delete_example_data` in api/src/database/offerings/tests.rs):
--   1. provider_agent_delegations  (FK -> provider_registrations, no cascade)
--   2. agent_pools                 (FK -> provider_registrations, no cascade)
--   3. provider_agent_status       (no FK; PK on provider_pubkey)
--   4. provider_offerings          (no FK on pubkey; child tables cascade/set-null
--                                   via offering_id: visibility_allowlist,
--                                   provider_offering_sli_reports, cloud_accounts)
--   5. provider_profiles           (provider_profiles_contacts + auto_accept_rules
--                                   cascade from profile delete)
--   6. provider_registrations      (provider_offering_sli_reports cascades)

DELETE FROM provider_agent_delegations
    WHERE provider_pubkey = E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572';

DELETE FROM agent_pools
    WHERE provider_pubkey = E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572';

DELETE FROM provider_agent_status
    WHERE provider_pubkey = E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572';

DELETE FROM provider_offerings
    WHERE pubkey = E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572';

DELETE FROM provider_profiles
    WHERE pubkey = E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572';

DELETE FROM provider_registrations
    WHERE pubkey = E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572';
