// API URLs for different environments
const DEV_API_BASE_URL = 'http://localhost:59001';
const PROD_API_BASE_URL = 'https://api.decent-cloud.org';

// Determine API URL based on build mode and environment variable
// Priority: VITE_DECENT_CLOUD_API_URL env var > production mode > development mode
const API_BASE_URL =
	import.meta.env.VITE_DECENT_CLOUD_API_URL?.replace(/\/+$/, '') ??
	(import.meta.env.PROD ? PROD_API_BASE_URL : DEV_API_BASE_URL);

interface ApiResponse<T> {
	success: boolean;
	data?: T | null;
	error?: string | null;
}

export interface PlatformStats {
	total_providers: number;
	active_providers: number;
	total_offerings: number;
	total_contracts: number;
	total_transfers: number;
	total_volume_e9s: number;
	validator_count_24h: number;
	latest_block_timestamp_ns: number | null;
	metadata: Record<string, unknown>;
}

export async function fetchPlatformStats(): Promise<PlatformStats> {
	const url = `${API_BASE_URL}/api/v1/stats`;
	const response = await fetch(url);

	if (!response.ok) {
		throw new Error(`Failed to fetch platform stats: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<PlatformStats>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Decent Cloud API stats response failed');
	}

	if (!payload.data) {
		throw new Error('Decent Cloud API stats response did not include data');
	}

	return payload.data;
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

function hexEncode(bytes: Uint8Array | number[]): string {
	return Array.from(bytes)
		.map((b) => b.toString(16).padStart(2, '0'))
		.join('');
}

export { hexEncode };

function normalizePubkeyHash(pubkeyHash: string | number[]): string {
	if (typeof pubkeyHash === 'string') {
		return pubkeyHash;
	}
	return hexEncode(new Uint8Array(pubkeyHash));
}

export interface Offering {
	id: number;
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
	price_per_hour_e9s?: number;
	price_per_day_e9s?: number;
}

export interface OfferingSearchParams {
	limit?: number;
	offset?: number;
	product_type?: string;
	country?: string;
	min_price_e9s?: number;
	max_price_e9s?: number;
	in_stock_only?: boolean;
}

export async function searchOfferings(params: OfferingSearchParams = {}): Promise<Offering[]> {
	const searchParams = new URLSearchParams();
	if (params.limit !== undefined) searchParams.set('limit', params.limit.toString());
	if (params.offset !== undefined) searchParams.set('offset', params.offset.toString());
	if (params.product_type) searchParams.set('product_type', params.product_type);
	if (params.country) searchParams.set('country', params.country);
	if (params.min_price_e9s !== undefined) searchParams.set('min_price_e9s', params.min_price_e9s.toString());
	if (params.max_price_e9s !== undefined) searchParams.set('max_price_e9s', params.max_price_e9s.toString());
	if (params.in_stock_only) searchParams.set('in_stock_only', 'true');

	const url = `${API_BASE_URL}/api/v1/offerings?${searchParams.toString()}`;
	const response = await fetch(url);

	if (!response.ok) {
		throw new Error(`Failed to fetch offerings: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<Offering[]>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Decent Cloud API offerings response failed');
	}

	const offerings = payload.data ?? [];
	// Normalize pubkey_hash to hex string
	return offerings.map((o) => ({
		...o,
		pubkey_hash: normalizePubkeyHash(o.pubkey_hash)
	}));
}

export async function getActiveProviders(days: number = 1): Promise<ProviderProfile[]> {
	const url = `${API_BASE_URL}/api/v1/providers/active/${days}`;
	const response = await fetch(url);

	if (!response.ok) {
		throw new Error(`Failed to fetch active providers: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<ProviderProfile[]>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Decent Cloud API providers response failed');
	}

	const providers = payload.data ?? [];
	// Normalize pubkey_hash to hex string
	return providers.map((p) => ({
		...p,
		pubkey_hash: normalizePubkeyHash(p.pubkey_hash)
	}));
}

export async function getProviderOfferings(pubkeyHash: string | Uint8Array): Promise<Offering[]> {
	const pubkeyHex = typeof pubkeyHash === 'string' ? pubkeyHash : hexEncode(pubkeyHash);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/offerings`;
	const response = await fetch(url);

	if (!response.ok) {
		throw new Error(`Failed to fetch provider offerings: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<Offering[]>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Decent Cloud API provider offerings response failed');
	}

	const offerings = payload.data ?? [];
	// Normalize pubkey_hash to hex string
	return offerings.map((o) => ({
		...o,
		pubkey_hash: normalizePubkeyHash(o.pubkey_hash)
	}));
}
