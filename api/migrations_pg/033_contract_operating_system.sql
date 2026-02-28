-- Add operating_system column to store user's OS selection for the rented VM
ALTER TABLE contract_sign_requests ADD COLUMN IF NOT EXISTS operating_system TEXT;
