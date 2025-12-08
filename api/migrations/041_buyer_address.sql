-- Add buyer_address field to contract_sign_requests for B2B invoice compliance
ALTER TABLE contract_sign_requests ADD COLUMN buyer_address TEXT;
