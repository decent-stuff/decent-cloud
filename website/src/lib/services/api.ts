// API URLs for different environments
const DEV_API_BASE_URL = 'http://localhost:59001';
const PROD_API_BASE_URL = 'https://api.decent-cloud.org';

// Determine API URL based on build mode and environment variable
// Priority: VITE_DECENT_CLOUD_API_URL env var > production mode > development mode
export const API_BASE_URL =
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
	min_contract_hours?: number;
	max_contract_hours?: number;
	payment_methods?: string;
	features?: string;
	operating_systems?: string;
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

export interface CsvImportError {
	row: number;
	message: string;
}

export interface CsvImportResult {
	success_count: number;
	errors: CsvImportError[];
}

export async function importProviderOfferingsCSV(
	pubkeyHash: string | Uint8Array,
	csvContent: string,
	upsert: boolean,
	headers: Record<string, string>
): Promise<CsvImportResult> {
	const pubkeyHex = typeof pubkeyHash === 'string' ? pubkeyHash : hexEncode(pubkeyHash);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/offerings/import${upsert ? '?upsert=true' : ''}`;

	const response = await fetch(url, {
		method: 'POST',
		headers, // Headers already include Content-Type from signRequest
		body: csvContent
	});

	if (!response.ok) {
		const errorText = await response.text();
		throw new Error(`Failed to import CSV: ${response.status} ${response.statusText}\n${errorText}`);
	}

	const payload = (await response.json()) as ApiResponse<CsvImportResult>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'CSV import failed');
	}

	if (!payload.data) {
		throw new Error('CSV import response did not include data');
	}

	return payload.data;
}

export interface CreateOfferingParams {
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

export async function createProviderOffering(
	pubkeyHash: string | Uint8Array,
	params: CreateOfferingParams | string,
	headers: Record<string, string>
): Promise<number> {
	const pubkeyHex = typeof pubkeyHash === 'string' ? pubkeyHash : hexEncode(pubkeyHash);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/offerings`;

	// Accept either params object or pre-signed JSON string
	const body = typeof params === 'string' ? params : JSON.stringify(params);

	const response = await fetch(url, {
		method: 'POST',
		headers,
		body
	});

	if (!response.ok) {
		const errorText = await response.text();
		throw new Error(`Failed to create offering: ${response.status} ${response.statusText}\n${errorText}`);
	}

	const payload = (await response.json()) as ApiResponse<number>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to create offering');
	}

	if (payload.data === null || payload.data === undefined) {
		throw new Error('Create offering response did not include offering ID');
	}

	return payload.data;
}

export async function updateProviderOffering(
	pubkeyHash: string | Uint8Array,
	offeringId: number,
	params: CreateOfferingParams | string,
	headers: Record<string, string>
): Promise<void> {
	const pubkeyHex = typeof pubkeyHash === 'string' ? pubkeyHash : hexEncode(pubkeyHash);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/offerings/${offeringId}`;

	// Accept either params object or pre-signed JSON string
	const body = typeof params === 'string' ? params : JSON.stringify(params);

	const response = await fetch(url, {
		method: 'PUT',
		headers,
		body
	});

	if (!response.ok) {
		const errorText = await response.text();
		throw new Error(`Failed to update offering: ${response.status} ${response.statusText}\n${errorText}`);
	}

	const payload = (await response.json()) as ApiResponse<void>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to update offering');
	}
}

export async function fetchCSVTemplate(): Promise<string> {
	const url = `${API_BASE_URL}/api/v1/offerings/template`;
	const response = await fetch(url);

	if (!response.ok) {
		throw new Error(`Failed to fetch CSV template: ${response.status} ${response.statusText}`);
	}

	return await response.text();
}

export async function downloadCSVTemplate(): Promise<void> {
	const csv = await fetchCSVTemplate();
	const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
	const link = document.createElement('a');
	const url = URL.createObjectURL(blob);

	link.setAttribute('href', url);
	link.setAttribute('download', 'offerings-template.csv');
	link.style.visibility = 'hidden';
	document.body.appendChild(link);
	link.click();
	document.body.removeChild(link);
	URL.revokeObjectURL(url);
}

export function offeringToCSVRow(offering: Offering): string[] {
	return [
		offering.offering_id,
		offering.offer_name,
		offering.description || '',
		offering.product_page_url || '',
		offering.currency,
		offering.monthly_price.toString(),
		offering.setup_fee.toString(),
		offering.visibility,
		offering.product_type,
		offering.virtualization_type || '',
		offering.billing_interval,
		offering.stock_status,
		offering.processor_brand || '',
		offering.processor_amount?.toString() || '',
		offering.processor_cores?.toString() || '',
		offering.processor_speed || '',
		offering.processor_name || '',
		offering.memory_error_correction || '',
		offering.memory_type || '',
		offering.memory_amount || '',
		offering.hdd_amount?.toString() || '',
		offering.total_hdd_capacity || '',
		offering.ssd_amount?.toString() || '',
		offering.total_ssd_capacity || '',
		offering.unmetered_bandwidth.toString(),
		offering.uplink_speed || '',
		offering.traffic?.toString() || '',
		offering.datacenter_country,
		offering.datacenter_city,
		offering.datacenter_latitude?.toString() || '',
		offering.datacenter_longitude?.toString() || '',
		offering.control_panel || '',
		offering.gpu_name || '',
		offering.min_contract_hours?.toString() || '',
		offering.max_contract_hours?.toString() || '',
		offering.payment_methods || '',
		offering.features || '',
		offering.operating_systems || ''
	];
}

// CSV header for offerings
const OFFERINGS_CSV_HEADER = [
	'offering_id',
	'offer_name',
	'description',
	'product_page_url',
	'currency',
	'monthly_price',
	'setup_fee',
	'visibility',
	'product_type',
	'virtualization_type',
	'billing_interval',
	'stock_status',
	'processor_brand',
	'processor_amount',
	'processor_cores',
	'processor_speed',
	'processor_name',
	'memory_error_correction',
	'memory_type',
	'memory_amount',
	'hdd_amount',
	'total_hdd_capacity',
	'ssd_amount',
	'total_ssd_capacity',
	'unmetered_bandwidth',
	'uplink_speed',
	'traffic',
	'datacenter_country',
	'datacenter_city',
	'datacenter_latitude',
	'datacenter_longitude',
	'control_panel',
	'gpu_name',
	'min_contract_hours',
	'max_contract_hours',
	'payment_methods',
	'features',
	'operating_systems'
];

export function offeringsToCSV(offerings: Offering[]): string {
	const rows = [OFFERINGS_CSV_HEADER, ...offerings.map(offeringToCSVRow)];
	return rows.map((row) => row.join(',')).join('\n');
}

export async function downloadOfferingsCSV(offerings: Offering[], filename: string = 'offerings.csv'): Promise<void> {
	const csv = offeringsToCSV(offerings);
	const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
	const link = document.createElement('a');
	const url = URL.createObjectURL(blob);

	link.setAttribute('href', url);
	link.setAttribute('download', filename);
	link.style.visibility = 'hidden';
	document.body.appendChild(link);
	link.click();
	document.body.removeChild(link);
	URL.revokeObjectURL(url);
}
