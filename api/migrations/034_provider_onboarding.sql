-- Provider onboarding fields for Help Center
ALTER TABLE provider_profiles ADD COLUMN support_email TEXT;
ALTER TABLE provider_profiles ADD COLUMN support_hours TEXT;
ALTER TABLE provider_profiles ADD COLUMN support_channels TEXT; -- JSON array
ALTER TABLE provider_profiles ADD COLUMN regions TEXT; -- JSON array
ALTER TABLE provider_profiles ADD COLUMN payment_methods TEXT; -- JSON array
ALTER TABLE provider_profiles ADD COLUMN refund_policy TEXT;
ALTER TABLE provider_profiles ADD COLUMN sla_guarantee TEXT;
ALTER TABLE provider_profiles ADD COLUMN unique_selling_points TEXT; -- JSON array
ALTER TABLE provider_profiles ADD COLUMN common_issues TEXT; -- JSON array of {question, answer}
ALTER TABLE provider_profiles ADD COLUMN onboarding_completed_at INTEGER; -- timestamp
