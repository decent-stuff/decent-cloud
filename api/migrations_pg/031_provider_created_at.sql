ALTER TABLE provider_profiles
ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ DEFAULT NOW();

-- Backfill existing rows: use updated_at as proxy for creation date
UPDATE provider_profiles
SET created_at = updated_at
WHERE created_at IS NULL;
