-- Rename provider_notification_config to user_notification_config
-- Remove FK constraint (allow all users, not just providers)
-- Change from single notify_via to boolean flags for multi-channel

-- Create new table with updated schema
CREATE TABLE IF NOT EXISTS user_notification_config (
    user_pubkey BLOB PRIMARY KEY,
    chatwoot_portal_slug TEXT,
    notify_telegram INTEGER NOT NULL DEFAULT 0,
    notify_email INTEGER NOT NULL DEFAULT 0,
    notify_sms INTEGER NOT NULL DEFAULT 0,
    telegram_chat_id TEXT,
    notify_phone TEXT,
    notify_email_address TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Migrate existing data
INSERT OR IGNORE INTO user_notification_config
    (user_pubkey, chatwoot_portal_slug, notify_telegram, notify_email, notify_sms,
     telegram_chat_id, notify_phone, created_at, updated_at)
SELECT
    provider_pubkey,
    chatwoot_portal_slug,
    CASE WHEN notify_via = 'telegram' THEN 1 ELSE 0 END,
    CASE WHEN notify_via = 'email' THEN 1 ELSE 0 END,
    CASE WHEN notify_via = 'sms' THEN 1 ELSE 0 END,
    telegram_chat_id,
    notify_phone,
    created_at,
    updated_at
FROM provider_notification_config;

-- Drop old table
DROP TABLE IF EXISTS provider_notification_config;
