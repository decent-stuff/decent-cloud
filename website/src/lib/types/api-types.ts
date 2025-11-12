// Type conversion utilities for API responses
// Converts generated Rust types (with bigint, null) to frontend-friendly types (number, undefined)

// Frontend-friendly API types (manually converted from generated types)
// Generated types use: bigint for integers, null for optionals
// Frontend types use: number for integers, undefined for optionals

export interface Offering {
	id?: number;
	pubkey_hash: string | number[];
	offering_id: string;
	offer_name: string;
	description?: string;
	product_page_url?: string;
	currency: string;
	monthly_price: number;
	setup_fee: number;
	visibility: string;
	product_type: string;
	virtualization_type?: string;
	billing_interval: string;
	stock_status: string;
	processor_brand?: string;
	processor_amount?: number;
	processor_cores?: number;
	processor_speed?: string;
	processor_name?: string;
	memory_error_correction?: string;
	memory_type?: string;
	memory_amount?: string;
	hdd_amount?: number;
	total_hdd_capacity?: string;
	ssd_amount?: number;
	total_ssd_capacity?: string;
	unmetered_bandwidth: boolean;
	uplink_speed?: string;
	traffic?: number;
	datacenter_country: string;
	datacenter_city: string;
	datacenter_latitude?: number;
	datacenter_longitude?: number;
	control_panel?: string;
	gpu_name?: string;
	min_contract_hours?: number;
	max_contract_hours?: number;
	payment_methods?: string;
	features?: string;
	operating_systems?: string;
}

export interface ProviderProfile {
	pubkey_hash: string | number[];
	name: string;
	description?: string;
	website_url?: string;
	logo_url?: string;
	why_choose_us?: string;
	api_version: string;
	profile_version: string;
	updated_at_ns: number;
}

export interface PlatformStats {
	total_providers: number;
	active_providers: number;
	total_offerings: number;
	total_contracts: number;
	total_transfers: number;
	total_volume_e9s: number;
	validator_count_24h: number;
	latest_block_timestamp_ns?: number;
	metadata: Record<string, unknown>;
}

export interface UserProfile {
	pubkey_hash: string | number[];
	display_name?: string;
	bio?: string;
	avatar_url?: string;
}

export interface UserContact {
	contact_type: string;
	contact_value: string;
	is_verified: boolean;
}

export interface UserSocial {
	platform: string;
	username: string;
	profile_url?: string;
}

export interface UserPublicKey {
	key_type: string;
	public_key: string;
	label?: string;
	added_at_ns: number;
}

// Create offering params (omit id and pubkey_hash for creation)
export type CreateOfferingParams = Omit<Offering, 'id' | 'pubkey_hash'>;

// Generic API response wrapper
export interface ApiResponse<T> {
	success: boolean;
	data?: T | null;
	error?: string | null;
}

// CSV import types
export interface CsvImportError {
	row: number;
	message: string;
}

export interface CsvImportResult {
	success_count: number;
	errors: CsvImportError[];
}

// Offering search parameters
export interface OfferingSearchParams {
	limit?: number;
	offset?: number;
	product_type?: string;
	country?: string;
	min_price_monthly?: number;
	max_price_monthly?: number;
	in_stock_only?: boolean;
}
