-- Provider notification configuration for support escalations
CREATE TABLE IF NOT EXISTS provider_notification_config (
    provider_pubkey BLOB PRIMARY KEY,
    chatwoot_portal_slug TEXT,
    notify_via TEXT NOT NULL CHECK (notify_via IN ('telegram', 'sms', 'email')),
    telegram_chat_id TEXT,
    notify_phone TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (provider_pubkey) REFERENCES provider_profiles(pubkey)
);
