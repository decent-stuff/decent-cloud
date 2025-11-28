-- Email Queue Migration
-- Stores emails for reliable delivery with retry logic

CREATE TABLE email_queue (
    id BLOB PRIMARY KEY NOT NULL,
    to_addr TEXT NOT NULL,       -- RFC 2822 format: "Name <email@example.com>" or multiple
    from_addr TEXT NOT NULL,     -- RFC 2822 format: "Name <email@example.com>"
    subject TEXT NOT NULL,
    body TEXT NOT NULL,
    is_html INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'pending', -- 'pending', 'sent', 'failed'
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 3,
    last_error TEXT,
    created_at INTEGER NOT NULL,
    last_attempted_at INTEGER,
    sent_at INTEGER
);

CREATE INDEX idx_email_queue_status ON email_queue(status);
CREATE INDEX idx_email_queue_created_at ON email_queue(created_at);
