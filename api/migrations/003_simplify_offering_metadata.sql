-- Migration to simplify offering metadata by moving payment_methods, features, and operating_systems
-- from separate normalized tables into the main provider_offerings table as TEXT columns

-- Step 1: Add new columns to provider_offerings table
ALTER TABLE provider_offerings ADD COLUMN payment_methods TEXT;
ALTER TABLE provider_offerings ADD COLUMN features TEXT;
ALTER TABLE provider_offerings ADD COLUMN operating_systems TEXT;

-- Step 2: Migrate existing data from normalized tables to main table
-- For each offering, aggregate the related records into comma-separated strings
-- SQLite GROUP_CONCAT uses comma as default separator, but we can specify it explicitly
UPDATE provider_offerings
SET payment_methods = (
    SELECT GROUP_CONCAT(payment_method, ',')
    FROM provider_offerings_payment_methods
    WHERE provider_offerings_payment_methods.offering_id = provider_offerings.id
)
WHERE EXISTS (
    SELECT 1 FROM provider_offerings_payment_methods
    WHERE provider_offerings_payment_methods.offering_id = provider_offerings.id
);

UPDATE provider_offerings
SET features = (
    SELECT GROUP_CONCAT(feature, ',')
    FROM provider_offerings_features
    WHERE provider_offerings_features.offering_id = provider_offerings.id
)
WHERE EXISTS (
    SELECT 1 FROM provider_offerings_features
    WHERE provider_offerings_features.offering_id = provider_offerings.id
);

UPDATE provider_offerings
SET operating_systems = (
    SELECT GROUP_CONCAT(operating_system, ',')
    FROM provider_offerings_operating_systems
    WHERE provider_offerings_operating_systems.offering_id = provider_offerings.id
)
WHERE EXISTS (
    SELECT 1 FROM provider_offerings_operating_systems
    WHERE provider_offerings_operating_systems.offering_id = provider_offerings.id
);

-- Step 3: Drop indexes on the normalized tables
DROP INDEX IF EXISTS idx_provider_offerings_payment_methods_offering;
DROP INDEX IF EXISTS idx_provider_offerings_features_offering;
DROP INDEX IF EXISTS idx_provider_offerings_os_offering;

-- Step 4: Drop the normalized tables
DROP TABLE IF EXISTS provider_offerings_payment_methods;
DROP TABLE IF EXISTS provider_offerings_features;
DROP TABLE IF EXISTS provider_offerings_operating_systems;
