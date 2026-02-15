-- Migration 015: SSH key tracking for cloud resources
-- Allows proper cleanup of Hetzner SSH keys on termination

ALTER TABLE cloud_resources
  ADD COLUMN external_ssh_key_id TEXT;

COMMENT ON COLUMN cloud_resources.external_ssh_key_id IS 'Backend-specific SSH key ID for cleanup (e.g., Hetzner SSH key ID)';
