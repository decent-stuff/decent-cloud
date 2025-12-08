-- Drop deprecated chatwoot_portal_slug column from user_notification_config
-- Portal slug is now stored in provider_profiles.chatwoot_portal_slug
ALTER TABLE user_notification_config DROP COLUMN chatwoot_portal_slug;
