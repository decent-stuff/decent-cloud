-- Track when cancelled contracts have been terminated by dc-agent
ALTER TABLE contract_sign_requests ADD COLUMN terminated_at_ns INTEGER;
