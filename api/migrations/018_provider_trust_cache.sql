-- Add cached trust score columns to provider_profiles
-- Updated when trust metrics are fetched

ALTER TABLE provider_profiles ADD COLUMN trust_score INTEGER DEFAULT NULL;
ALTER TABLE provider_profiles ADD COLUMN has_critical_flags INTEGER DEFAULT 0;
