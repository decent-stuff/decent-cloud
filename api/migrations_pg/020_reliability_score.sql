-- Add cached reliability score to provider_profiles for efficient offering queries
ALTER TABLE provider_profiles ADD COLUMN IF NOT EXISTS reliability_score DOUBLE PRECISION;
