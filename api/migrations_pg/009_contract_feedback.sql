-- Contract feedback for user surveys (structured, not reviews)
-- Renters can submit Y/N feedback after contract completion:
-- 1. Did service match description?
-- 2. Would you rent from this provider again?

CREATE TABLE contract_feedback (
    id BIGSERIAL PRIMARY KEY,
    -- One feedback per contract
    contract_id BYTEA NOT NULL UNIQUE REFERENCES contract_sign_requests(contract_id) ON DELETE CASCADE,
    -- Who submitted (the renter)
    requester_pubkey BYTEA NOT NULL,
    -- Who received (the provider)
    provider_pubkey BYTEA NOT NULL,
    -- Structured Y/N feedback
    service_matched_description BOOLEAN NOT NULL,
    would_rent_again BOOLEAN NOT NULL,
    -- Timestamp
    created_at_ns BIGINT NOT NULL
);

-- Index for provider feedback aggregation
CREATE INDEX idx_contract_feedback_provider ON contract_feedback(provider_pubkey);
-- Index for renter's feedback history
CREATE INDEX idx_contract_feedback_requester ON contract_feedback(requester_pubkey);
