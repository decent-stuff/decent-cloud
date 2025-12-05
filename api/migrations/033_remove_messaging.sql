-- Remove messaging feature (deprecated in favor of Chatwoot)
-- Drop tables in correct order due to foreign key constraints

DROP TABLE IF EXISTS message_notifications;
DROP TABLE IF EXISTS message_read_receipts;
DROP TABLE IF EXISTS messages;
DROP TABLE IF EXISTS message_thread_participants;
DROP TABLE IF EXISTS message_threads;
