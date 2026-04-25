-- Decent Agents beta waitlist signup capture (issue #423).
-- Public unauthenticated endpoint; soft-launch gating only.
CREATE TABLE agents_waitlist (
    id            BIGSERIAL PRIMARY KEY,
    email         TEXT        NOT NULL UNIQUE,
    github_handle TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    source        TEXT,
    notes         TEXT
);

CREATE INDEX agents_waitlist_created_at_desc_idx
    ON agents_waitlist (created_at DESC);
