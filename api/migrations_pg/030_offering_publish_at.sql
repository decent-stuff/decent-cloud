-- Add publish_at to provider_offerings; when is_draft=true and publish_at <= NOW(),
-- the background scheduler auto-publishes the offering by setting is_draft=false.
ALTER TABLE provider_offerings ADD COLUMN IF NOT EXISTS publish_at TIMESTAMPTZ;
