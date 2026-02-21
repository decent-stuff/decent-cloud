CREATE TABLE contract_events (
    id BIGSERIAL PRIMARY KEY,
    contract_id BYTEA NOT NULL REFERENCES contract_sign_requests(contract_id),
    event_type TEXT NOT NULL,
    old_status TEXT,
    new_status TEXT,
    actor TEXT NOT NULL,
    details TEXT,
    created_at BIGINT NOT NULL
);

CREATE INDEX idx_contract_events_contract ON contract_events(contract_id, created_at);
