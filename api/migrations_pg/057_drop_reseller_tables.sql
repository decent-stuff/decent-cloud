-- Drop dormant reseller-program tables (feature removed; all were empty).
-- Order children before parents due to FK refs: reseller_commissions and
-- reseller_commissions_mapping reference reseller_accounts.
DROP TABLE IF EXISTS reseller_orders;
DROP TABLE IF EXISTS reseller_relationships;
DROP TABLE IF EXISTS reseller_commissions_mapping;
DROP TABLE IF EXISTS reseller_commissions;
DROP TABLE IF EXISTS reseller_accounts;
