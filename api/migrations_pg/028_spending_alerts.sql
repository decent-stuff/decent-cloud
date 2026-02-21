-- Add spending alert configuration per user (keyed by hex-encoded pubkey)
CREATE TABLE spending_alerts (
    pubkey TEXT PRIMARY KEY,
    monthly_limit_usd DOUBLE PRECISION NOT NULL,
    alert_at_pct INTEGER NOT NULL DEFAULT 80 CHECK (alert_at_pct >= 1 AND alert_at_pct <= 100),
    last_notified_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
