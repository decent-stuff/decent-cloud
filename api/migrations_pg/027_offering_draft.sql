-- Add is_draft to provider_offerings; draft offerings are hidden from public marketplace search
ALTER TABLE provider_offerings ADD COLUMN IF NOT EXISTS is_draft BOOLEAN NOT NULL DEFAULT false;
