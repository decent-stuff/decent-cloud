-- Drop the external_providers table.
-- After the reseller program removal, this table was write-only (no production read path).
-- The sole writer was api-cli's seed-external-provider command, which is also removed.
DROP TABLE IF EXISTS external_providers;
