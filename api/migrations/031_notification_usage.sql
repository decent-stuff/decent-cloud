-- Track notification usage per provider per day for rate limiting
CREATE TABLE IF NOT EXISTS notification_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider_id TEXT NOT NULL,
    channel TEXT NOT NULL,  -- 'telegram', 'sms', 'email'
    date TEXT NOT NULL,     -- YYYY-MM-DD format
    count INTEGER NOT NULL DEFAULT 0,
    UNIQUE(provider_id, channel, date)
);

CREATE INDEX IF NOT EXISTS idx_notification_usage_provider_date ON notification_usage(provider_id, date);
