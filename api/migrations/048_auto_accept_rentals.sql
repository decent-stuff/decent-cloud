-- Add auto_accept_rentals setting to provider_profiles
-- When enabled, new rental contracts skip provider approval step
-- and go directly to 'accepted' status after payment succeeds
ALTER TABLE provider_profiles ADD COLUMN auto_accept_rentals INTEGER NOT NULL DEFAULT 1;
