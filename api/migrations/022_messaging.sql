-- Messaging Infrastructure Migration
-- Enables contract-based messaging between requesters and providers

-- Conversation threads (1 thread per contract, expandable later)
CREATE TABLE message_threads (
    id BLOB PRIMARY KEY NOT NULL,
    contract_id BLOB NOT NULL UNIQUE,
    subject TEXT NOT NULL,
    created_at_ns INTEGER NOT NULL,
    last_message_at_ns INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'open', -- 'open', 'resolved', 'closed'
    FOREIGN KEY (contract_id) REFERENCES contract_sign_requests(contract_id) ON DELETE CASCADE
);

CREATE INDEX idx_message_threads_contract_id ON message_threads(contract_id);
CREATE INDEX idx_message_threads_status ON message_threads(status);
CREATE INDEX idx_message_threads_last_message_at_ns ON message_threads(last_message_at_ns DESC);

-- Core message storage
CREATE TABLE messages (
    id BLOB PRIMARY KEY NOT NULL,
    thread_id BLOB NOT NULL,
    sender_pubkey TEXT NOT NULL,
    sender_role TEXT NOT NULL DEFAULT 'user', -- 'user', 'assistant', 'system' (AI-compatible)
    body TEXT NOT NULL,
    created_at_ns INTEGER NOT NULL,
    FOREIGN KEY (thread_id) REFERENCES message_threads(id) ON DELETE CASCADE
);

CREATE INDEX idx_messages_thread_id ON messages(thread_id);
CREATE INDEX idx_messages_sender_pubkey ON messages(sender_pubkey);
CREATE INDEX idx_messages_created_at_ns ON messages(created_at_ns);

-- Track who's in each thread
CREATE TABLE message_thread_participants (
    thread_id BLOB NOT NULL,
    pubkey TEXT NOT NULL,
    role TEXT NOT NULL, -- 'requester', 'provider'
    joined_at_ns INTEGER NOT NULL,
    PRIMARY KEY (thread_id, pubkey),
    FOREIGN KEY (thread_id) REFERENCES message_threads(id) ON DELETE CASCADE
);

CREATE INDEX idx_message_thread_participants_pubkey ON message_thread_participants(pubkey);

-- Track read status per user
CREATE TABLE message_read_receipts (
    message_id BLOB NOT NULL,
    reader_pubkey TEXT NOT NULL,
    read_at_ns INTEGER NOT NULL,
    PRIMARY KEY (message_id, reader_pubkey),
    FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE
);

CREATE INDEX idx_message_read_receipts_reader ON message_read_receipts(reader_pubkey);

-- Queue for email notifications about messages
CREATE TABLE message_notifications (
    id BLOB PRIMARY KEY NOT NULL,
    message_id BLOB NOT NULL,
    recipient_pubkey TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending', -- 'pending', 'sent', 'skipped'
    created_at_ns INTEGER NOT NULL,
    sent_at_ns INTEGER,
    FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE
);

CREATE INDEX idx_message_notifications_status ON message_notifications(status);
CREATE INDEX idx_message_notifications_created_at_ns ON message_notifications(created_at_ns);
