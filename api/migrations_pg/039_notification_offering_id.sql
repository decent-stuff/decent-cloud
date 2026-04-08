ALTER TABLE user_notifications ADD COLUMN offering_id BIGINT;
CREATE INDEX idx_user_notifications_offering_id ON user_notifications(offering_id) WHERE offering_id IS NOT NULL;
