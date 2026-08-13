-- Rename the operator account to the platform brand.
-- The operator resells cloud resources under the "Decent Cloud" name; the old
-- "hetzner-reseller" label is no longer user-facing (the reseller-program feature
-- was removed). username stays URL-safe (no spaces); provider_profiles.name is the
-- human-facing display name. Idempotent: only rewrites the legacy values.
UPDATE accounts SET username = 'decent-cloud' WHERE username = 'hetzner-reseller';
UPDATE provider_profiles SET name = 'Decent Cloud' WHERE name = 'hetzner-reseller';
