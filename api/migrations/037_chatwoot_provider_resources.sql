-- Chatwoot resources created for each provider during onboarding
ALTER TABLE provider_profiles ADD COLUMN chatwoot_inbox_id INTEGER;
ALTER TABLE provider_profiles ADD COLUMN chatwoot_team_id INTEGER;
ALTER TABLE provider_profiles ADD COLUMN chatwoot_portal_slug TEXT;
