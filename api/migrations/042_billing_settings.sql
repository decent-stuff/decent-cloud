-- Add billing settings fields to accounts table for saved billing information
-- These fields allow users to save their billing details for faster checkout

ALTER TABLE accounts ADD COLUMN billing_address TEXT;
ALTER TABLE accounts ADD COLUMN billing_vat_id TEXT;
ALTER TABLE accounts ADD COLUMN billing_country_code TEXT; -- 2-letter ISO country code (e.g., DE, FR, US)
