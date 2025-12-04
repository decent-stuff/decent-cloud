-- Add Chatwoot user ID to accounts for Platform API operations
ALTER TABLE accounts ADD COLUMN chatwoot_user_id INTEGER;
