-- Add device_name to account public keys for device identification
ALTER TABLE account_public_keys ADD COLUMN device_name TEXT;
