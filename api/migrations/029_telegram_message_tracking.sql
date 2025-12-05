-- Telegram message tracking for reply handling
-- Maps Telegram message IDs to Chatwoot conversation IDs
CREATE TABLE IF NOT EXISTS telegram_message_tracking (
    telegram_message_id INTEGER PRIMARY KEY,
    conversation_id INTEGER NOT NULL,
    provider_chat_id TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

-- Index for cleanup of old entries
CREATE INDEX IF NOT EXISTS idx_telegram_tracking_created_at ON telegram_message_tracking(created_at);
