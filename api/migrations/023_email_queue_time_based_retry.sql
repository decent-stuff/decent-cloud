-- Email Queue: Time-based retry with 7-day window
-- Changes from attempt-based to time-based failure detection
-- Adds user notification tracking for message senders

-- Track which account to notify on failure (for message notifications, this is the sender)
ALTER TABLE email_queue ADD COLUMN related_account_id BLOB;

-- Track if we've notified the user about retry/permanent failure
ALTER TABLE email_queue ADD COLUMN user_notified_retry INTEGER NOT NULL DEFAULT 0;
ALTER TABLE email_queue ADD COLUMN user_notified_gave_up INTEGER NOT NULL DEFAULT 0;

-- Index for efficient lookup of related account notifications
CREATE INDEX idx_email_queue_related_account ON email_queue(related_account_id);
