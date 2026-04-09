ALTER TABLE user_notifications ADD COLUMN price_direction TEXT CHECK (price_direction IN ('up', 'down'));
