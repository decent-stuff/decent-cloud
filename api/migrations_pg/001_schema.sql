-- Consolidated PostgreSQL Schema for Decent Cloud API
-- Flattened from 64 SQLite migrations
-- Generated: 2026-01-03
-- Fixed: All missing tables, columns, indexes, and constraints from SQLite migrations

-- Enable UUID generation
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

--------------------------------------------------------------------------------
-- PROVIDER TABLES
--------------------------------------------------------------------------------

-- Provider registrations
CREATE TABLE provider_registrations (
    id BIGSERIAL PRIMARY KEY,
    pubkey BYTEA NOT NULL UNIQUE,
    signature BYTEA NOT NULL,
    created_at_ns BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Provider check-ins
CREATE TABLE provider_check_ins (
    id BIGSERIAL PRIMARY KEY,
    pubkey BYTEA NOT NULL,
    memo TEXT NOT NULL,
    nonce_signature BYTEA NOT NULL,
    block_timestamp_ns BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Provider profiles
CREATE TABLE provider_profiles (
    id BIGSERIAL PRIMARY KEY,
    pubkey BYTEA NOT NULL UNIQUE,
    name TEXT NOT NULL,
    description TEXT,
    website_url TEXT,
    logo_url TEXT,
    why_choose_us TEXT,
    api_version TEXT NOT NULL,
    profile_version TEXT NOT NULL,
    updated_at_ns BIGINT NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    -- Added: onboarding fields (migrations 034, 035)
    support_email TEXT,
    support_hours TEXT,
    support_channels TEXT,           -- JSON array
    regions TEXT,                    -- JSON array
    payment_methods TEXT,            -- JSON array
    refund_policy TEXT,
    sla_guarantee TEXT,
    unique_selling_points TEXT,      -- JSON array
    common_issues TEXT,              -- JSON array of {question, answer}
    onboarding_completed_at BIGINT,
    -- Added: chatwoot (migration 037)
    chatwoot_inbox_id BIGINT,
    chatwoot_portal_slug TEXT,
    chatwoot_team_id BIGINT,
    -- Added: trust fields (migration 018)
    trust_score BIGINT,
    has_critical_flags BOOLEAN DEFAULT FALSE,
    -- Added: auto_accept_rentals (migration 048/049)
    auto_accept_rentals BOOLEAN NOT NULL DEFAULT TRUE,
    -- Added: account_id (migration 050)
    account_id BYTEA
);

-- Provider profile contacts
CREATE TABLE provider_profiles_contacts (
    id BIGSERIAL PRIMARY KEY,
    provider_pubkey BYTEA NOT NULL REFERENCES provider_profiles(pubkey) ON DELETE CASCADE,
    contact_type TEXT NOT NULL,
    contact_value TEXT NOT NULL
);

-- Provider offerings
CREATE TABLE provider_offerings (
    id BIGSERIAL PRIMARY KEY,
    pubkey BYTEA NOT NULL,
    offering_id TEXT NOT NULL,
    offer_name TEXT NOT NULL,
    description TEXT,
    product_page_url TEXT,
    currency TEXT NOT NULL,
    monthly_price DOUBLE PRECISION NOT NULL,
    setup_fee DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    visibility TEXT NOT NULL,
    product_type TEXT NOT NULL,
    virtualization_type TEXT,
    billing_interval TEXT NOT NULL,
    stock_status TEXT NOT NULL,
    processor_brand TEXT,
    processor_amount BIGINT,
    processor_cores BIGINT,
    processor_speed TEXT,
    processor_name TEXT,
    memory_error_correction TEXT,
    memory_type TEXT,
    memory_amount TEXT,
    hdd_amount BIGINT DEFAULT 0,
    total_hdd_capacity TEXT,
    ssd_amount BIGINT DEFAULT 0,
    total_ssd_capacity TEXT,
    unmetered_bandwidth BOOLEAN DEFAULT FALSE,
    uplink_speed TEXT,
    traffic BIGINT,
    datacenter_country TEXT NOT NULL,
    datacenter_city TEXT NOT NULL,
    datacenter_latitude DOUBLE PRECISION,
    datacenter_longitude DOUBLE PRECISION,
    control_panel TEXT,
    gpu_name TEXT,
    min_contract_hours BIGINT,
    max_contract_hours BIGINT,
    created_at_ns BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    payment_methods TEXT,
    features TEXT,
    operating_systems TEXT,
    -- Added: GPU fields (migration 006)
    gpu_count BIGINT,
    gpu_memory_gb BIGINT,
    -- Added: account_id (migration 050)
    account_id BYTEA,
    -- Added: provisioner config (migration 052)
    provisioner_type TEXT,
    provisioner_config TEXT,
    -- Added: agent pool (migration 053)
    agent_pool_id TEXT,
    -- Added: usage billing (migration 059)
    billing_unit TEXT NOT NULL DEFAULT 'month',
    pricing_model TEXT DEFAULT 'flat',
    price_per_unit DOUBLE PRECISION,
    included_units BIGINT,
    overage_price_per_unit DOUBLE PRECISION,
    stripe_metered_price_id TEXT,
    -- Added: external providers (migration 035)
    offering_source TEXT DEFAULT 'provider',
    external_checkout_url TEXT,
    -- Added: subscription support (migration 061)
    is_subscription BOOLEAN DEFAULT FALSE,
    subscription_interval_days BIGINT,
    UNIQUE(pubkey, offering_id)
);

-- Provider trust cache (migration 018)
CREATE TABLE provider_trust_cache (
    pubkey BYTEA PRIMARY KEY,
    trust_score BIGINT NOT NULL,
    updated_at_ns BIGINT NOT NULL
);

-- Provider onboarding tracking (migration 034)
CREATE TABLE provider_onboarding (
    provider_pubkey BYTEA PRIMARY KEY,
    onboarding_step TEXT NOT NULL DEFAULT 'initial',
    completed_steps TEXT,
    onboarding_started_at_ns BIGINT NOT NULL,
    onboarding_completed_at_ns BIGINT,
    last_updated_at_ns BIGINT NOT NULL
);

-- External providers - tracks providers not yet onboarded (migration 035)
CREATE TABLE external_providers (
    id BIGSERIAL PRIMARY KEY,
    pubkey BYTEA NOT NULL UNIQUE,           -- Deterministic pubkey from domain
    name TEXT NOT NULL,
    domain TEXT NOT NULL UNIQUE,
    website_url TEXT NOT NULL,
    logo_url TEXT,
    data_source TEXT NOT NULL,             -- "scraper", "manual_curation"
    created_at_ns BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_external_providers_domain ON external_providers(domain);

--------------------------------------------------------------------------------
-- USER TABLES
--------------------------------------------------------------------------------

-- User registrations
CREATE TABLE user_registrations (
    id BIGSERIAL PRIMARY KEY,
    pubkey BYTEA NOT NULL UNIQUE,
    signature BYTEA NOT NULL,
    created_at_ns BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- User profiles
CREATE TABLE user_profiles (
    id BIGSERIAL PRIMARY KEY,
    pubkey BYTEA NOT NULL UNIQUE REFERENCES user_registrations(pubkey) ON DELETE CASCADE,
    display_name TEXT,
    bio TEXT,
    avatar_url TEXT,
    updated_at_ns BIGINT NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- User contacts
CREATE TABLE user_contacts (
    id BIGSERIAL PRIMARY KEY,
    user_pubkey BYTEA NOT NULL REFERENCES user_registrations(pubkey) ON DELETE CASCADE,
    contact_type TEXT NOT NULL,
    contact_value TEXT NOT NULL,
    verified BOOLEAN DEFAULT FALSE,
    created_at_ns BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- User socials
CREATE TABLE user_socials (
    id BIGSERIAL PRIMARY KEY,
    user_pubkey BYTEA NOT NULL REFERENCES user_registrations(pubkey) ON DELETE CASCADE,
    platform TEXT NOT NULL,
    username TEXT NOT NULL,
    profile_url TEXT,
    created_at_ns BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- User public keys (SSH, GPG, etc.)
CREATE TABLE user_public_keys (
    id BIGSERIAL PRIMARY KEY,
    user_pubkey BYTEA NOT NULL REFERENCES user_registrations(pubkey) ON DELETE CASCADE,
    key_type TEXT NOT NULL,
    key_data TEXT NOT NULL,
    key_fingerprint TEXT,
    label TEXT,
    created_at_ns BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- User notification config (migration 032) - REPLACES provider_notification_config
CREATE TABLE user_notification_config (
    user_pubkey BYTEA PRIMARY KEY,
    notify_telegram BOOLEAN NOT NULL DEFAULT FALSE,
    notify_email BOOLEAN NOT NULL DEFAULT FALSE,
    notify_sms BOOLEAN NOT NULL DEFAULT FALSE,
    telegram_chat_id TEXT,
    notify_phone TEXT,
    notify_email_address TEXT,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);

--------------------------------------------------------------------------------
-- ACCOUNT TABLES
--------------------------------------------------------------------------------

-- Accounts (username-based system)
CREATE TABLE accounts (
    id BYTEA PRIMARY KEY DEFAULT gen_random_bytes(16),
    username TEXT NOT NULL,
    created_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000000000)::BIGINT,
    updated_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000000000)::BIGINT,
    -- Profile fields (migration 004)
    display_name TEXT,
    bio TEXT,
    avatar_url TEXT,
    profile_updated_at BIGINT,
    -- OAuth fields (migration 005)
    auth_provider TEXT DEFAULT 'seed_phrase',
    email TEXT,
    -- Email verification (migration 020)
    email_verified BOOLEAN NOT NULL DEFAULT FALSE,
    -- Last login tracking (migration 019)
    last_login_at BIGINT,
    -- Admin flag (migration 021)
    is_admin BOOLEAN NOT NULL DEFAULT FALSE,
    -- Chatwoot user ID (migration 027)
    chatwoot_user_id BIGINT,
    -- Subscription fields (migration 057)
    stripe_customer_id TEXT,
    subscription_plan_id TEXT DEFAULT 'free',
    subscription_status TEXT DEFAULT 'active',
    subscription_stripe_id TEXT,
    subscription_current_period_end BIGINT,
    subscription_cancel_at_period_end BOOLEAN DEFAULT FALSE,
    -- Billing info (migration 042)
    billing_address TEXT,
    billing_vat_id TEXT,
    billing_country_code TEXT,
    -- Username validation (allows alphanumeric, dots, underscores, @, hyphens)
    CONSTRAINT username_format CHECK (
        username ~ '^[a-zA-Z0-9][a-zA-Z0-9._@-]*[a-zA-Z0-9]$'
        AND LENGTH(username) >= 3
        AND LENGTH(username) <= 64
    )
);

-- Case-insensitive unique username index
CREATE UNIQUE INDEX idx_accounts_username_unique ON accounts(LOWER(username));
CREATE INDEX idx_accounts_username ON accounts(username);
CREATE INDEX idx_accounts_auth_provider ON accounts(auth_provider);
CREATE UNIQUE INDEX idx_accounts_email_unique ON accounts(email) WHERE email IS NOT NULL;
CREATE INDEX idx_accounts_last_login ON accounts(last_login_at);
CREATE INDEX idx_accounts_is_admin ON accounts(is_admin);
CREATE INDEX idx_accounts_stripe_customer ON accounts(stripe_customer_id);
CREATE INDEX idx_accounts_subscription_status ON accounts(subscription_status);
CREATE INDEX idx_accounts_subscription_plan ON accounts(subscription_plan_id);

-- Account public keys (multi-device support)
CREATE TABLE account_public_keys (
    id BYTEA PRIMARY KEY DEFAULT gen_random_bytes(16),
    account_id BYTEA NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    public_key BYTEA UNIQUE NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    added_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000000000)::BIGINT,
    disabled_at BIGINT,
    disabled_by_key_id BYTEA REFERENCES account_public_keys(id),
    device_name TEXT,
    CONSTRAINT public_key_length CHECK (LENGTH(public_key) = 32),
    UNIQUE(account_id, public_key)
);

CREATE INDEX idx_keys_account ON account_public_keys(account_id);
CREATE INDEX idx_keys_pubkey ON account_public_keys(public_key);
CREATE INDEX idx_keys_active ON account_public_keys(account_id, is_active);

-- Signature audit trail
CREATE TABLE signature_audit (
    id BYTEA PRIMARY KEY DEFAULT gen_random_bytes(16),
    account_id BYTEA REFERENCES accounts(id),
    action TEXT NOT NULL,
    payload TEXT NOT NULL,
    signature BYTEA NOT NULL,
    public_key BYTEA NOT NULL,
    timestamp BIGINT NOT NULL,
    nonce BYTEA NOT NULL,
    is_admin_action BOOLEAN NOT NULL DEFAULT FALSE,
    created_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000000000)::BIGINT,
    CONSTRAINT signature_length CHECK (LENGTH(signature) = 64),
    CONSTRAINT audit_public_key_length CHECK (LENGTH(public_key) = 32),
    CONSTRAINT nonce_length CHECK (LENGTH(nonce) = 16)
);

CREATE INDEX idx_audit_nonce_time ON signature_audit(nonce, created_at);
CREATE INDEX idx_audit_account ON signature_audit(account_id);
CREATE INDEX idx_audit_created ON signature_audit(created_at);

-- Account contacts
CREATE TABLE account_contacts (
    id BIGSERIAL PRIMARY KEY,
    account_id BYTEA NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    contact_type TEXT NOT NULL,
    contact_value TEXT NOT NULL,
    verified BOOLEAN DEFAULT FALSE,
    created_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000000000)::BIGINT
);

CREATE INDEX idx_account_contacts_account_id ON account_contacts(account_id);

-- Account socials
CREATE TABLE account_socials (
    id BIGSERIAL PRIMARY KEY,
    account_id BYTEA NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    platform TEXT NOT NULL,
    username TEXT NOT NULL,
    profile_url TEXT,
    created_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000000000)::BIGINT
);

CREATE INDEX idx_account_socials_account_id ON account_socials(account_id);

-- Account external keys (SSH/GPG)
CREATE TABLE account_external_keys (
    id BIGSERIAL PRIMARY KEY,
    account_id BYTEA NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    key_type TEXT NOT NULL,
    key_data TEXT NOT NULL,
    key_fingerprint TEXT,
    label TEXT,
    created_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000000000)::BIGINT
);

CREATE INDEX idx_account_external_keys_account_id ON account_external_keys(account_id);

-- OAuth accounts
CREATE TABLE oauth_accounts (
    id BYTEA PRIMARY KEY DEFAULT gen_random_bytes(16),
    account_id BYTEA NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    external_id TEXT NOT NULL,
    email TEXT,
    created_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000000000)::BIGINT,
    UNIQUE(provider, external_id),
    CONSTRAINT oauth_provider_type CHECK (provider IN ('google_oauth'))
);

CREATE INDEX idx_oauth_accounts_account ON oauth_accounts(account_id);
CREATE INDEX idx_oauth_accounts_provider_external ON oauth_accounts(provider, external_id);
CREATE INDEX idx_oauth_accounts_email ON oauth_accounts(email);

-- Admin accounts (migration 021)
CREATE TABLE admin_accounts (
    account_id BYTEA PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
    granted_at_ns BIGINT NOT NULL,
    granted_by BYTEA
);

--------------------------------------------------------------------------------
-- TOKEN/PAYMENT TABLES
--------------------------------------------------------------------------------

-- Token transfers
CREATE TABLE token_transfers (
    id BIGSERIAL PRIMARY KEY,
    from_account TEXT NOT NULL,
    to_account TEXT NOT NULL,
    amount_e9s BIGINT NOT NULL,
    fee_e9s BIGINT NOT NULL DEFAULT 0,
    memo TEXT,
    created_at_ns BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    block_hash BYTEA,
    block_offset BIGINT
);

-- Token approvals
CREATE TABLE token_approvals (
    id BIGSERIAL PRIMARY KEY,
    owner_account TEXT NOT NULL,
    spender_account TEXT NOT NULL,
    amount_e9s BIGINT NOT NULL,
    expires_at_ns BIGINT,
    created_at_ns BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Escrow (migration 030)
CREATE TABLE escrow (
    id BIGSERIAL PRIMARY KEY,
    contract_id BYTEA NOT NULL UNIQUE,
    payer_pubkey BYTEA NOT NULL,
    payee_pubkey BYTEA NOT NULL,
    amount_e9s BIGINT NOT NULL,
    status TEXT NOT NULL DEFAULT 'held',
    created_at_ns BIGINT NOT NULL,
    released_at_ns BIGINT,
    refunded_at_ns BIGINT
);

CREATE INDEX idx_escrow_contract ON escrow(contract_id);
CREATE INDEX idx_escrow_payer ON escrow(payer_pubkey);
CREATE INDEX idx_escrow_payee ON escrow(payee_pubkey);

--------------------------------------------------------------------------------
-- CONTRACT TABLES
--------------------------------------------------------------------------------

-- Contract sign requests
CREATE TABLE contract_sign_requests (
    id BIGSERIAL PRIMARY KEY,
    contract_id BYTEA NOT NULL UNIQUE,
    requester_pubkey BYTEA NOT NULL,
    requester_ssh_pubkey TEXT NOT NULL,
    requester_contact TEXT NOT NULL,
    provider_pubkey BYTEA NOT NULL,
    offering_id TEXT NOT NULL,
    region_name TEXT,
    instance_config TEXT,
    payment_amount_e9s BIGINT NOT NULL,
    start_timestamp_ns BIGINT,
    end_timestamp_ns BIGINT,
    duration_hours BIGINT,
    original_duration_hours BIGINT,
    request_memo TEXT NOT NULL,
    created_at_ns BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    status TEXT DEFAULT 'pending',
    status_updated_at_ns BIGINT,
    status_updated_by BYTEA,
    provisioning_instance_details TEXT,
    provisioning_completed_at_ns BIGINT,
    -- Added: currency (migrations 013-017) - NO DEFAULT for fail-fast behavior
    currency TEXT NOT NULL,
    -- Added: Chatwoot tracking (migration 024, 027)
    chatwoot_conversation_id INTEGER,
    chatwoot_user_id INTEGER,
    -- Added: SLA tracking (migration 026)
    sla_response_deadline_ns BIGINT,
    sla_provision_deadline_ns BIGINT,
    -- Added: account-based identification (migration 050)
    requester_account_id BYTEA,
    provider_account_id BYTEA,
    -- Added: termination tracking (migration 051)
    terminated_at_ns BIGINT,
    -- Added: agent pool locks (migration 053)
    provisioning_lock_agent BYTEA,
    provisioning_lock_at_ns BIGINT,
    provisioning_lock_expires_ns BIGINT,
    -- Added: subscription support (migration 062)
    stripe_subscription_id TEXT,
    subscription_status TEXT,
    current_period_end_ns BIGINT,
    cancel_at_period_end BOOLEAN DEFAULT FALSE,
    -- Added: gateway configuration (migration 063)
    gateway_slug TEXT,
    gateway_ssh_port INTEGER,
    gateway_port_range_start INTEGER,
    gateway_port_range_end INTEGER,
    -- Added: payment methods (migration 010)
    payment_method TEXT NOT NULL DEFAULT 'icpay',
    stripe_payment_intent_id TEXT,
    stripe_customer_id TEXT,
    -- Added: payment status (migration 011)
    payment_status TEXT NOT NULL DEFAULT 'pending',
    -- Added: ICPay tracking (migrations 025, 030)
    icpay_transaction_id TEXT,
    icpay_payment_id TEXT,
    icpay_refund_id TEXT,
    total_released_e9s BIGINT DEFAULT 0,
    last_release_at_ns BIGINT,
    -- Added: Stripe invoice (migration 044)
    stripe_invoice_id TEXT,
    -- Added: refund tracking (migration 012)
    refund_amount_e9s BIGINT,
    stripe_refund_id TEXT,
    refund_created_at_ns BIGINT,
    -- Added: tax tracking (migration 040)
    tax_amount_e9s BIGINT,
    tax_rate_percent DOUBLE PRECISION,
    tax_type TEXT,
    tax_jurisdiction TEXT,
    customer_tax_id TEXT,
    reverse_charge BOOLEAN DEFAULT FALSE,
    -- Added: buyer address (migration 041)
    buyer_address TEXT,
    -- Added: receipt tracking (migration 038)
    receipt_number BIGINT,
    receipt_sent_at_ns BIGINT
);

-- Contract provisioning details
CREATE TABLE contract_provisioning_details (
    contract_id BYTEA PRIMARY KEY REFERENCES contract_sign_requests(contract_id) ON DELETE CASCADE,
    instance_ip TEXT,
    instance_credentials TEXT,
    connection_instructions TEXT,
    provisioned_at_ns BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Contract status history
CREATE TABLE contract_status_history (
    id BIGSERIAL PRIMARY KEY,
    contract_id BYTEA NOT NULL REFERENCES contract_sign_requests(contract_id) ON DELETE CASCADE,
    old_status TEXT NOT NULL,
    new_status TEXT NOT NULL,
    changed_by BYTEA NOT NULL,
    changed_at_ns BIGINT NOT NULL,
    change_memo TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Contract payment entries
CREATE TABLE contract_payment_entries (
    id BIGSERIAL PRIMARY KEY,
    contract_id BYTEA NOT NULL REFERENCES contract_sign_requests(contract_id) ON DELETE CASCADE,
    pricing_model TEXT NOT NULL,
    time_period_unit TEXT NOT NULL,
    quantity INTEGER NOT NULL,
    amount_e9s BIGINT NOT NULL
);

-- Contract sign replies
CREATE TABLE contract_sign_replies (
    id BIGSERIAL PRIMARY KEY,
    contract_id BYTEA NOT NULL REFERENCES contract_sign_requests(contract_id) ON DELETE CASCADE,
    provider_pubkey BYTEA NOT NULL,
    reply_status TEXT NOT NULL,
    reply_memo TEXT,
    instance_details TEXT,
    created_at_ns BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Contract extensions
CREATE TABLE contract_extensions (
    id BIGSERIAL PRIMARY KEY,
    contract_id BYTEA NOT NULL REFERENCES contract_sign_requests(contract_id) ON DELETE CASCADE,
    extended_by_pubkey BYTEA NOT NULL,
    extension_hours BIGINT NOT NULL,
    extension_payment_e9s BIGINT NOT NULL,
    previous_end_timestamp_ns BIGINT NOT NULL,
    new_end_timestamp_ns BIGINT NOT NULL,
    extension_memo TEXT,
    created_at_ns BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Contract usage tracking
CREATE TABLE contract_usage (
    id BIGSERIAL PRIMARY KEY,
    contract_id BYTEA NOT NULL REFERENCES contract_sign_requests(contract_id),
    billing_period_start BIGINT NOT NULL,
    billing_period_end BIGINT NOT NULL,
    units_used DOUBLE PRECISION NOT NULL DEFAULT 0,
    units_included DOUBLE PRECISION,
    overage_units DOUBLE PRECISION NOT NULL DEFAULT 0,
    estimated_charge_cents BIGINT,
    reported_to_stripe BOOLEAN NOT NULL DEFAULT FALSE,
    stripe_usage_record_id TEXT,
    created_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000000000)::BIGINT,
    updated_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000000000)::BIGINT
);

CREATE INDEX idx_contract_usage_contract ON contract_usage(contract_id);
CREATE INDEX idx_contract_usage_unreported ON contract_usage(reported_to_stripe, billing_period_end);

-- Contract usage events
CREATE TABLE contract_usage_events (
    id BIGSERIAL PRIMARY KEY,
    contract_id BYTEA NOT NULL REFERENCES contract_sign_requests(contract_id),
    event_type TEXT NOT NULL,
    units_delta DOUBLE PRECISION,
    heartbeat_at BIGINT,
    source TEXT,
    metadata TEXT,
    created_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000000000)::BIGINT
);

CREATE INDEX idx_contract_usage_events_contract ON contract_usage_events(contract_id);
CREATE INDEX idx_contract_usage_events_type ON contract_usage_events(event_type, created_at);

--------------------------------------------------------------------------------
-- AGENT/DELEGATION TABLES
--------------------------------------------------------------------------------

-- Provider agent delegations
CREATE TABLE provider_agent_delegations (
    id BIGSERIAL PRIMARY KEY,
    provider_pubkey BYTEA NOT NULL REFERENCES provider_registrations(pubkey),
    agent_pubkey BYTEA NOT NULL UNIQUE,
    permissions TEXT NOT NULL,
    expires_at_ns BIGINT,
    label TEXT,
    signature BYTEA NOT NULL,
    created_at_ns BIGINT NOT NULL,
    revoked_at_ns BIGINT,
    pool_id TEXT
);

CREATE INDEX idx_agent_delegations_agent ON provider_agent_delegations(agent_pubkey) WHERE revoked_at_ns IS NULL;
CREATE INDEX idx_agent_delegations_provider ON provider_agent_delegations(provider_pubkey);

-- Provider agent status
CREATE TABLE provider_agent_status (
    provider_pubkey BYTEA PRIMARY KEY,
    online BOOLEAN NOT NULL DEFAULT FALSE,
    last_heartbeat_ns BIGINT,
    version TEXT,
    provisioner_type TEXT,
    capabilities TEXT,
    active_contracts BIGINT DEFAULT 0,
    updated_at_ns BIGINT NOT NULL
);

-- Agent pools
CREATE TABLE agent_pools (
    pool_id TEXT PRIMARY KEY,
    provider_pubkey BYTEA NOT NULL REFERENCES provider_registrations(pubkey),
    name TEXT NOT NULL,
    location TEXT NOT NULL,
    provisioner_type TEXT NOT NULL,
    created_at_ns BIGINT NOT NULL
);

CREATE INDEX idx_agent_pools_provider ON agent_pools(provider_pubkey);

-- Agent setup tokens
CREATE TABLE agent_setup_tokens (
    token TEXT PRIMARY KEY,
    pool_id TEXT NOT NULL REFERENCES agent_pools(pool_id) ON DELETE CASCADE,
    label TEXT,
    created_at_ns BIGINT NOT NULL,
    expires_at_ns BIGINT NOT NULL,
    used_at_ns BIGINT,
    used_by_agent BYTEA
);

CREATE INDEX idx_setup_tokens_pool ON agent_setup_tokens(pool_id);
CREATE INDEX idx_setup_tokens_unused ON agent_setup_tokens(pool_id) WHERE used_at_ns IS NULL;

--------------------------------------------------------------------------------
-- REPUTATION TABLES
--------------------------------------------------------------------------------

-- Reputation changes
CREATE TABLE reputation_changes (
    id BIGSERIAL PRIMARY KEY,
    pubkey BYTEA NOT NULL,
    change_amount BIGINT NOT NULL,
    reason TEXT NOT NULL DEFAULT '',
    block_timestamp_ns BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Reputation aging
CREATE TABLE reputation_aging (
    id BIGSERIAL PRIMARY KEY,
    block_timestamp_ns BIGINT NOT NULL,
    aging_factor_ppm BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Reward distributions
CREATE TABLE reward_distributions (
    id BIGSERIAL PRIMARY KEY,
    block_timestamp_ns BIGINT NOT NULL,
    total_amount_e9s BIGINT NOT NULL,
    providers_count INTEGER NOT NULL,
    amount_per_provider_e9s BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

--------------------------------------------------------------------------------
-- SUBSCRIPTION TABLES
--------------------------------------------------------------------------------

-- Subscription plans
CREATE TABLE subscription_plans (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    stripe_price_id TEXT,
    monthly_price_cents INTEGER NOT NULL DEFAULT 0,
    trial_days INTEGER NOT NULL DEFAULT 0,
    features TEXT,
    created_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000000000)::BIGINT,
    updated_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000000000)::BIGINT
);

CREATE INDEX idx_subscription_plans_stripe_price ON subscription_plans(stripe_price_id);

-- Subscription events
CREATE TABLE subscription_events (
    id BIGSERIAL PRIMARY KEY,
    account_id BYTEA NOT NULL REFERENCES accounts(id),
    event_type TEXT NOT NULL,
    stripe_event_id TEXT UNIQUE,
    old_plan_id TEXT,
    new_plan_id TEXT,
    stripe_subscription_id TEXT,
    stripe_invoice_id TEXT,
    amount_cents BIGINT,
    metadata TEXT,
    created_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000000000)::BIGINT
);

CREATE INDEX idx_subscription_events_account ON subscription_events(account_id);
CREATE INDEX idx_subscription_events_stripe_event ON subscription_events(stripe_event_id);
CREATE INDEX idx_subscription_events_created ON subscription_events(created_at);

--------------------------------------------------------------------------------
-- RESELLER TABLES
--------------------------------------------------------------------------------

-- Reseller accounts
CREATE TABLE reseller_accounts (
    id BIGSERIAL PRIMARY KEY,
    account_id BYTEA NOT NULL UNIQUE REFERENCES accounts(id) ON DELETE CASCADE,
    company_name TEXT NOT NULL,
    contact_email TEXT NOT NULL,
    commission_rate_ppm INTEGER NOT NULL DEFAULT 100000,
    status TEXT NOT NULL DEFAULT 'pending',
    approved_at_ns BIGINT,
    approved_by BYTEA,
    created_at_ns BIGINT NOT NULL
);

CREATE INDEX idx_reseller_accounts_account ON reseller_accounts(account_id);
CREATE INDEX idx_reseller_accounts_status ON reseller_accounts(status);

-- Reseller commissions
CREATE TABLE reseller_commissions (
    id BIGSERIAL PRIMARY KEY,
    reseller_account_id BIGINT NOT NULL REFERENCES reseller_accounts(id),
    contract_id BYTEA NOT NULL,
    commission_amount_e9s BIGINT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    paid_at_ns BIGINT,
    created_at_ns BIGINT NOT NULL
);

CREATE INDEX idx_reseller_commissions_reseller ON reseller_commissions(reseller_account_id);
CREATE INDEX idx_reseller_commissions_contract ON reseller_commissions(contract_id);

-- Reseller commission mapping
CREATE TABLE reseller_commissions_mapping (
    id BIGSERIAL PRIMARY KEY,
    reseller_account_id BIGINT NOT NULL REFERENCES reseller_accounts(id),
    referred_account_id BYTEA NOT NULL REFERENCES accounts(id),
    created_at_ns BIGINT NOT NULL,
    UNIQUE(referred_account_id)
);

-- Reseller relationships (migration 036)
CREATE TABLE reseller_relationships (
    id BIGSERIAL PRIMARY KEY,
    reseller_pubkey BYTEA NOT NULL,
    external_provider_pubkey BYTEA NOT NULL,
    commission_percent BIGINT NOT NULL DEFAULT 10,
    status TEXT NOT NULL DEFAULT 'active',
    created_at_ns BIGINT NOT NULL,
    updated_at_ns BIGINT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(reseller_pubkey, external_provider_pubkey)
);

CREATE INDEX idx_reseller_relationships_reseller ON reseller_relationships(reseller_pubkey);
CREATE INDEX idx_reseller_relationships_status ON reseller_relationships(status);

-- Reseller orders (migration 036)
CREATE TABLE reseller_orders (
    id BIGSERIAL PRIMARY KEY,
    contract_id BYTEA NOT NULL UNIQUE,
    reseller_pubkey BYTEA NOT NULL,
    external_provider_pubkey BYTEA NOT NULL,
    offering_id BIGINT NOT NULL,
    base_price_e9s BIGINT NOT NULL,
    commission_e9s BIGINT NOT NULL,
    total_paid_e9s BIGINT NOT NULL,
    external_order_id TEXT,
    external_order_details TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at_ns BIGINT NOT NULL,
    fulfilled_at_ns BIGINT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_reseller_orders_reseller ON reseller_orders(reseller_pubkey);
CREATE INDEX idx_reseller_orders_status ON reseller_orders(status);

--------------------------------------------------------------------------------
-- BILLING/INVOICE TABLES
--------------------------------------------------------------------------------

-- Receipt tracking
CREATE TABLE receipt_tracking (
    id BIGSERIAL PRIMARY KEY,
    contract_id BYTEA NOT NULL,
    receipt_type TEXT NOT NULL,
    amount_e9s BIGINT NOT NULL,
    currency TEXT NOT NULL,
    recipient_email TEXT,
    sent_at_ns BIGINT,
    created_at_ns BIGINT NOT NULL
);

CREATE INDEX idx_receipt_tracking_contract ON receipt_tracking(contract_id);

-- Invoices (migration 039, recreated in 046)
CREATE TABLE invoices (
    id BYTEA PRIMARY KEY DEFAULT gen_random_bytes(16),
    contract_id BYTEA NOT NULL UNIQUE,
    invoice_number TEXT NOT NULL UNIQUE,
    invoice_date_ns BIGINT NOT NULL,
    seller_name TEXT NOT NULL,
    seller_address TEXT NOT NULL,
    seller_vat_id TEXT,
    buyer_name TEXT,
    buyer_address TEXT,
    buyer_vat_id TEXT,
    subtotal_e9s BIGINT NOT NULL,
    vat_rate_percent BIGINT NOT NULL DEFAULT 0,
    vat_amount_e9s BIGINT NOT NULL DEFAULT 0,
    total_e9s BIGINT NOT NULL,
    currency TEXT NOT NULL,
    pdf_generated_at_ns BIGINT,
    created_at_ns BIGINT NOT NULL,
    FOREIGN KEY (contract_id) REFERENCES contract_sign_requests(contract_id)
);

CREATE INDEX idx_invoices_contract_id ON invoices(contract_id);
CREATE INDEX idx_invoices_invoice_number ON invoices(invoice_number);
CREATE INDEX idx_invoices_created_at ON invoices(created_at_ns);

-- Invoice sequence for sequential numbering (migration 039)
CREATE TABLE invoice_sequence (
    id BIGINT PRIMARY KEY CHECK (id = 1),
    year BIGINT NOT NULL,
    next_number BIGINT NOT NULL DEFAULT 1
);

-- Initialize invoice sequence for current year
INSERT INTO invoice_sequence (id, year, next_number)
VALUES (1, EXTRACT(YEAR FROM NOW()), 1)
ON CONFLICT (id) DO NOTHING;

-- Tax tracking (migration 040)
CREATE TABLE tax_tracking (
    id BIGSERIAL PRIMARY KEY,
    invoice_id BYTEA NOT NULL REFERENCES invoices(id),
    tax_type TEXT NOT NULL,
    tax_rate_ppm INTEGER NOT NULL,
    tax_amount_e9s BIGINT NOT NULL,
    jurisdiction TEXT
);

CREATE INDEX idx_tax_tracking_invoice ON tax_tracking(invoice_id);

-- Billing settings (migration 042)
CREATE TABLE billing_settings (
    account_id BYTEA PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
    company_name TEXT,
    tax_id TEXT,
    billing_email TEXT,
    billing_address TEXT,
    updated_at_ns BIGINT NOT NULL
);

-- Pending Stripe receipts (migration 045)
CREATE TABLE pending_stripe_receipts (
    contract_id BYTEA PRIMARY KEY,
    created_at_ns BIGINT NOT NULL,
    next_attempt_at_ns BIGINT NOT NULL,
    attempts BIGINT NOT NULL DEFAULT 0
);

CREATE INDEX idx_pending_stripe_receipts_next_attempt ON pending_stripe_receipts(next_attempt_at_ns);

-- Receipt sequence (migration 038)
CREATE TABLE receipt_sequence (
    id BIGINT PRIMARY KEY CHECK (id = 1),
    next_number BIGINT NOT NULL DEFAULT 1
);

-- Initialize with first receipt number
INSERT INTO receipt_sequence (id, next_number) VALUES (1, 1)
ON CONFLICT (id) DO NOTHING;

-- Receipt number index on contract_sign_requests
CREATE INDEX idx_contract_sign_requests_receipt_number ON contract_sign_requests(receipt_number) WHERE receipt_number IS NOT NULL;

--------------------------------------------------------------------------------
-- EMAIL TABLES
--------------------------------------------------------------------------------

-- Email queue
CREATE TABLE email_queue (
    id BYTEA PRIMARY KEY NOT NULL,
    to_addr TEXT NOT NULL,
    from_addr TEXT NOT NULL,
    subject TEXT NOT NULL,
    body TEXT NOT NULL,
    is_html BOOLEAN NOT NULL DEFAULT FALSE,
    email_type TEXT NOT NULL DEFAULT 'general',
    status TEXT NOT NULL DEFAULT 'pending',
    attempts BIGINT NOT NULL DEFAULT 0,
    max_attempts BIGINT NOT NULL DEFAULT 6,
    last_error TEXT,
    created_at BIGINT NOT NULL,
    last_attempted_at BIGINT,
    sent_at BIGINT,
    related_account_id BYTEA,
    user_notified_retry BOOLEAN NOT NULL DEFAULT FALSE,
    user_notified_gave_up BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX idx_email_queue_status ON email_queue(status);
CREATE INDEX idx_email_queue_created_at ON email_queue(created_at);
CREATE INDEX idx_email_queue_related_account ON email_queue(related_account_id);

-- Email verification tokens (migration 020)
CREATE TABLE email_verification_tokens (
    token BYTEA PRIMARY KEY NOT NULL,
    account_id BYTEA NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    expires_at BIGINT NOT NULL,
    used_at BIGINT
);

CREATE INDEX idx_email_verification_tokens_account_id ON email_verification_tokens(account_id);
CREATE INDEX idx_email_verification_tokens_expires_at ON email_verification_tokens(expires_at);
CREATE INDEX idx_email_verification_tokens_email ON email_verification_tokens(email);

-- Account Recovery Tokens (migration 009)
CREATE TABLE recovery_tokens (
    token BYTEA PRIMARY KEY NOT NULL,
    account_id BYTEA NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    created_at BIGINT NOT NULL,
    expires_at BIGINT NOT NULL,
    used_at BIGINT
);

CREATE INDEX idx_recovery_tokens_account_id ON recovery_tokens(account_id);
CREATE INDEX idx_recovery_tokens_expires_at ON recovery_tokens(expires_at);

--------------------------------------------------------------------------------
-- MESSAGING/NOTIFICATION TABLES
--------------------------------------------------------------------------------

-- Telegram message tracking (migration 029)
CREATE TABLE telegram_message_tracking (
    telegram_message_id BIGINT PRIMARY KEY,
    conversation_id BIGINT NOT NULL,
    provider_chat_id TEXT NOT NULL,
    created_at BIGINT NOT NULL
);

CREATE INDEX idx_telegram_tracking_created_at ON telegram_message_tracking(created_at);

-- Notification usage (migration 031)
CREATE TABLE notification_usage (
    id BIGSERIAL PRIMARY KEY,
    provider_id TEXT NOT NULL,
    channel TEXT NOT NULL,
    date TEXT NOT NULL,
    count BIGINT NOT NULL DEFAULT 0,
    UNIQUE(provider_id, channel, date)
);

CREATE INDEX idx_notification_usage_provider_date ON notification_usage(provider_id, date);

-- Chatwoot message events (migration 024, 026)
CREATE TABLE chatwoot_message_events (
    id BIGSERIAL PRIMARY KEY,
    contract_id TEXT NOT NULL,
    chatwoot_conversation_id BIGINT NOT NULL,
    chatwoot_message_id BIGINT NOT NULL UNIQUE,
    sender_type TEXT NOT NULL CHECK (sender_type IN ('customer', 'provider')),
    created_at BIGINT NOT NULL,
    sla_breached BIGINT NOT NULL DEFAULT 0,
    sla_alert_sent BIGINT NOT NULL DEFAULT 0
);

CREATE INDEX idx_chatwoot_events_contract ON chatwoot_message_events(contract_id);
CREATE INDEX idx_chatwoot_events_conversation ON chatwoot_message_events(chatwoot_conversation_id);

-- Provider SLA configuration (migration 026)
CREATE TABLE provider_sla_config (
    provider_pubkey BYTEA PRIMARY KEY,
    response_time_seconds BIGINT NOT NULL DEFAULT 14400,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);

-- Chatwoot tracking
CREATE TABLE chatwoot_tracking (
    id BIGSERIAL PRIMARY KEY,
    entity_type TEXT NOT NULL,
    entity_id BYTEA NOT NULL,
    chatwoot_contact_id INTEGER,
    chatwoot_conversation_id INTEGER,
    created_at_ns BIGINT NOT NULL,
    updated_at_ns BIGINT NOT NULL,
    UNIQUE(entity_type, entity_id)
);

CREATE INDEX idx_chatwoot_tracking_entity ON chatwoot_tracking(entity_type, entity_id);

-- Chatwoot provider resources (migration 037)
CREATE TABLE chatwoot_provider_resources (
    provider_pubkey BYTEA PRIMARY KEY,
    portal_slug TEXT,
    category_id INTEGER,
    created_at_ns BIGINT NOT NULL
);

--------------------------------------------------------------------------------
-- PAYMENT RELEASES
--------------------------------------------------------------------------------

-- Payment releases for ICPay periodic payouts (migration 030)
CREATE TABLE payment_releases (
    id BIGSERIAL PRIMARY KEY,
    contract_id BYTEA NOT NULL REFERENCES contract_sign_requests(contract_id) ON DELETE CASCADE,
    release_type TEXT NOT NULL CHECK(release_type IN ('daily', 'hourly', 'final', 'cancellation')),
    period_start_ns BIGINT NOT NULL,
    period_end_ns BIGINT NOT NULL,
    amount_e9s BIGINT NOT NULL,
    provider_pubkey BYTEA NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'released', 'paid_out', 'refunded')),
    created_at_ns BIGINT NOT NULL,
    released_at_ns BIGINT,
    payout_id TEXT
);

CREATE INDEX idx_payment_releases_contract ON payment_releases(contract_id);
CREATE INDEX idx_payment_releases_provider ON payment_releases(provider_pubkey);
CREATE INDEX idx_payment_releases_status ON payment_releases(status);

--------------------------------------------------------------------------------
-- MISC TABLES
--------------------------------------------------------------------------------

-- Linked IC identities
CREATE TABLE linked_ic_ids (
    id BIGSERIAL PRIMARY KEY,
    pubkey BYTEA NOT NULL,
    ic_principal TEXT NOT NULL,
    operation TEXT NOT NULL,
    linked_at_ns BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Sync state
CREATE TABLE sync_state (
    id BIGINT PRIMARY KEY CHECK (id = 1),
    last_position BIGINT NOT NULL DEFAULT 0,
    last_sync_at TIMESTAMPTZ DEFAULT NOW()
);

-- Bandwidth history (migration 064)
CREATE TABLE bandwidth_history (
    id BIGSERIAL PRIMARY KEY,
    contract_id TEXT NOT NULL,
    gateway_slug TEXT NOT NULL,
    provider_pubkey TEXT NOT NULL,
    bytes_in BIGINT NOT NULL DEFAULT 0,
    bytes_out BIGINT NOT NULL DEFAULT 0,
    recorded_at_ns BIGINT NOT NULL
);

CREATE INDEX idx_bandwidth_history_contract ON bandwidth_history(contract_id, recorded_at_ns DESC);
CREATE INDEX idx_bandwidth_history_provider ON bandwidth_history(provider_pubkey, recorded_at_ns DESC);
CREATE INDEX idx_bandwidth_history_slug ON bandwidth_history(gateway_slug, recorded_at_ns DESC);

--------------------------------------------------------------------------------
-- INDEXES (remaining)
--------------------------------------------------------------------------------

-- Provider indexes
CREATE INDEX idx_provider_registrations_pubkey ON provider_registrations(pubkey);
CREATE INDEX idx_provider_check_ins_pubkey ON provider_check_ins(pubkey);
CREATE INDEX idx_provider_check_ins_timestamp ON provider_check_ins(block_timestamp_ns);
CREATE INDEX idx_provider_profiles_pubkey ON provider_profiles(pubkey);
CREATE INDEX idx_provider_profiles_account ON provider_profiles(account_id);
CREATE INDEX idx_provider_profiles_contacts_provider ON provider_profiles_contacts(provider_pubkey);
CREATE INDEX idx_provider_profiles_contacts_type ON provider_profiles_contacts(contact_type);
CREATE INDEX idx_provider_offerings_pubkey ON provider_offerings(pubkey);
CREATE INDEX idx_provider_offerings_offering_id ON provider_offerings(offering_id);
CREATE INDEX idx_provider_offerings_visibility ON provider_offerings(visibility);
CREATE INDEX idx_provider_offerings_country ON provider_offerings(datacenter_country);
CREATE INDEX idx_provider_offerings_product_type ON provider_offerings(product_type);
CREATE INDEX idx_provider_offerings_stock ON provider_offerings(stock_status);
CREATE INDEX idx_provider_offerings_account ON provider_offerings(account_id);
CREATE INDEX idx_provider_offerings_is_subscription ON provider_offerings(is_subscription);

-- Token indexes
CREATE INDEX idx_token_transfers_from_account ON token_transfers(from_account);
CREATE INDEX idx_token_transfers_to_account ON token_transfers(to_account);
CREATE INDEX idx_token_transfers_timestamp ON token_transfers(created_at_ns);
CREATE INDEX idx_token_transfers_block_hash ON token_transfers(block_hash);
CREATE INDEX idx_token_approvals_owner_account ON token_approvals(owner_account);
CREATE INDEX idx_token_approvals_spender_account ON token_approvals(spender_account);

-- Contract indexes
CREATE INDEX idx_contract_sign_requests_contract_id ON contract_sign_requests(contract_id);
CREATE INDEX idx_contract_sign_requests_requester_pubkey ON contract_sign_requests(requester_pubkey);
CREATE INDEX idx_contract_sign_requests_provider ON contract_sign_requests(provider_pubkey);
CREATE INDEX idx_contract_sign_requests_status ON contract_sign_requests(status);
CREATE INDEX idx_contract_sign_requests_offering ON contract_sign_requests(offering_id);
-- IMPORTANT: idx_contract_currency (migration 013, recreated in 017)
CREATE INDEX idx_contract_currency ON contract_sign_requests(currency);
CREATE INDEX idx_contracts_requester_account ON contract_sign_requests(requester_account_id);
CREATE INDEX idx_contracts_provider_account ON contract_sign_requests(provider_account_id);
CREATE INDEX idx_contracts_lock ON contract_sign_requests(provisioning_lock_expires_ns) WHERE provisioning_lock_agent IS NOT NULL;
CREATE INDEX idx_contract_subscriptions ON contract_sign_requests(stripe_subscription_id) WHERE stripe_subscription_id IS NOT NULL;
CREATE INDEX idx_contract_subscription_status ON contract_sign_requests(subscription_status) WHERE subscription_status IS NOT NULL;
CREATE UNIQUE INDEX idx_gateway_slug ON contract_sign_requests(gateway_slug) WHERE gateway_slug IS NOT NULL;
CREATE INDEX idx_contract_status_history_contract ON contract_status_history(contract_id);
CREATE INDEX idx_contract_status_history_changed_at ON contract_status_history(changed_at_ns);
CREATE INDEX idx_contract_payment_entries_contract_id ON contract_payment_entries(contract_id);
CREATE INDEX idx_contract_sign_replies_contract_id ON contract_sign_replies(contract_id);
CREATE INDEX idx_contract_extensions_contract_id ON contract_extensions(contract_id);
CREATE INDEX idx_contract_extensions_extended_by ON contract_extensions(extended_by_pubkey);
CREATE INDEX idx_contract_sign_requests_payment_method ON contract_sign_requests(payment_method);
CREATE INDEX idx_contract_sign_requests_stripe_payment_intent ON contract_sign_requests(stripe_payment_intent_id) WHERE stripe_payment_intent_id IS NOT NULL;
CREATE INDEX idx_contract_sign_requests_payment_status ON contract_sign_requests(payment_status);
CREATE INDEX idx_contract_sign_requests_payment_method_status ON contract_sign_requests(payment_method, payment_status);
CREATE INDEX idx_contract_sign_requests_icpay_transaction ON contract_sign_requests(icpay_transaction_id) WHERE icpay_transaction_id IS NOT NULL;
CREATE INDEX idx_contract_sign_requests_stripe_invoice ON contract_sign_requests(stripe_invoice_id) WHERE stripe_invoice_id IS NOT NULL;
CREATE INDEX idx_contract_refund_id ON contract_sign_requests(stripe_refund_id) WHERE stripe_refund_id IS NOT NULL;

-- Reputation indexes
CREATE INDEX idx_reputation_changes_pubkey ON reputation_changes(pubkey);
CREATE INDEX idx_reputation_changes_timestamp ON reputation_changes(block_timestamp_ns);

-- Linked IC indexes
CREATE INDEX idx_linked_ic_ids_pubkey ON linked_ic_ids(pubkey);
CREATE INDEX idx_linked_ic_ids_principal ON linked_ic_ids(ic_principal);
CREATE INDEX idx_linked_ic_ids_operation ON linked_ic_ids(operation);

-- User indexes
CREATE INDEX idx_user_profiles_pubkey ON user_profiles(pubkey);
CREATE INDEX idx_user_contacts_pubkey ON user_contacts(user_pubkey);
CREATE INDEX idx_user_contacts_type ON user_contacts(contact_type);
CREATE INDEX idx_user_socials_pubkey ON user_socials(user_pubkey);
CREATE INDEX idx_user_socials_platform ON user_socials(platform);
CREATE INDEX idx_user_public_keys_pubkey ON user_public_keys(user_pubkey);
CREATE INDEX idx_user_public_keys_type ON user_public_keys(key_type);
CREATE INDEX idx_user_public_keys_fingerprint ON user_public_keys(key_fingerprint);

--------------------------------------------------------------------------------
-- INITIAL DATA
--------------------------------------------------------------------------------

-- Initialize sync state
INSERT INTO sync_state (id, last_position) VALUES (1, 0)
ON CONFLICT (id) DO NOTHING;

-- Default subscription plans
INSERT INTO subscription_plans (id, name, description, monthly_price_cents, trial_days, features) VALUES
    ('free', 'Free', 'Basic marketplace access', 0, 0, '["marketplace_browse","one_rental"]'),
    ('pro', 'Pro', 'Full platform access', 2900, 14, '["marketplace_browse","unlimited_rentals","priority_support","api_access"]'),
    ('enterprise', 'Enterprise', 'Enterprise features', 9900, 14, '["marketplace_browse","unlimited_rentals","priority_support","api_access","dedicated_support","sla_guarantee"]')
ON CONFLICT (id) DO NOTHING;
