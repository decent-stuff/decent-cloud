-- Track Chatwoot message events for response time metrics
CREATE TABLE IF NOT EXISTS chatwoot_message_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_id TEXT NOT NULL,
    chatwoot_conversation_id INTEGER NOT NULL,
    chatwoot_message_id INTEGER NOT NULL UNIQUE,
    sender_type TEXT NOT NULL CHECK (sender_type IN ('customer', 'provider')),
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_chatwoot_events_contract ON chatwoot_message_events(contract_id);
CREATE INDEX IF NOT EXISTS idx_chatwoot_events_conversation ON chatwoot_message_events(chatwoot_conversation_id);
