// Import auto-generated types from Rust (these have pubkey_hash as Vec<u8> which is skipped in TS)
import type { Offering as OfferingRaw } from '$lib/types/generated/Offering';
import type { ProviderProfile as ProviderProfileRaw } from '$lib/types/generated/ProviderProfile';
import type { Validator as ValidatorRaw } from '$lib/types/generated/Validator';
import type { PlatformOverview } from '$lib/types/generated/PlatformOverview';
import type { UserProfile as UserProfileRaw } from '$lib/types/generated/UserProfile';
import type { UserContact } from '$lib/types/generated/UserContact';
import type { UserSocial } from '$lib/types/generated/UserSocial';
import type { UserPublicKey } from '$lib/types/generated/UserPublicKey';
import type { SignedRequestHeaders } from '$lib/types/generated/SignedRequestHeaders';

// Utility type to convert null to undefined (Rust Option -> TS optional)
type NullToUndefined<T> = T extends null ? undefined : T;
type ConvertNullToUndefined<T> = {
	[K in keyof T]: NullToUndefined<T[K]>;
};

// Frontend types: add pubkey_hash as string and convert null to undefined for convenience
export type Offering = ConvertNullToUndefined<OfferingRaw> & { pubkey_hash: string };
export type ProviderProfile = ConvertNullToUndefined<ProviderProfileRaw> & { pubkey_hash: string };
export type Validator = ConvertNullToUndefined<ValidatorRaw> & { pubkey_hash: string };
export type UserProfile = ConvertNullToUndefined<UserProfileRaw> & { pubkey_hash: string };
export type PlatformStats = ConvertNullToUndefined<PlatformOverview>;

// Generic API response wrapper
export interface ApiResponse<T> {
	success: boolean;
	data?: T | null;
	error?: string | null;
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

// CSV import types
export interface CsvImportError {
	row: number;
	message: string;
}

export interface CsvImportResult {
	success_count: number;
	errors: CsvImportError[];
}

// Create offering params (omit id and pubkey_hash for creation)
export type CreateOfferingParams = Omit<Offering, 'id' | 'pubkey_hash'>;

// API URLs for different environments
const DEV_API_BASE_URL = 'http://localhost:59001';
const PROD_API_BASE_URL = 'https://api.decent-cloud.org';

// Determine API URL based on build mode and environment variable
// Priority: VITE_DECENT_CLOUD_API_URL env var > production mode > development mode
export const API_BASE_URL =
	import.meta.env.VITE_DECENT_CLOUD_API_URL?.replace(/\/+$/, '') ??
	(import.meta.env.PROD ? PROD_API_BASE_URL : DEV_API_BASE_URL);

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

export async function searchOfferings(params: OfferingSearchParams = {}): Promise<Offering[]> {
	const searchParams = new URLSearchParams();
	if (params.limit !== undefined) searchParams.set('limit', params.limit.toString());
	if (params.offset !== undefined) searchParams.set('offset', params.offset.toString());
	if (params.product_type) searchParams.set('product_type', params.product_type);
	if (params.country) searchParams.set('country', params.country);
	if (params.min_price_monthly !== undefined) searchParams.set('min_price_monthly', params.min_price_monthly.toString());
	if (params.max_price_monthly !== undefined) searchParams.set('max_price_monthly', params.max_price_monthly.toString());
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

export async function getActiveValidators(days: number = 1): Promise<Validator[]> {
	const url = `${API_BASE_URL}/api/v1/validators/active/${days}`;
	const response = await fetch(url);

	if (!response.ok) {
		throw new Error(`Failed to fetch active validators: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<Validator[]>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Decent Cloud API validators response failed');
	}

	const validators = payload.data ?? [];
	// Normalize pubkey_hash to hex string
	return validators.map((v) => ({
		...v,
		pubkey_hash: normalizePubkeyHash(v.pubkey_hash)
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

export async function exportProviderOfferingsCSV(
	pubkeyHash: string | Uint8Array,
	headers: SignedRequestHeaders
): Promise<string> {
	const pubkeyHex = typeof pubkeyHash === 'string' ? pubkeyHash : hexEncode(pubkeyHash);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/offerings/export`;
	const response = await fetch(url, {
		method: 'GET',
		headers
	});

	if (!response.ok) {
		throw new Error(`Failed to export provider offerings: ${response.status} ${response.statusText}`);
	}

	return await response.text();
}

export async function importProviderOfferingsCSV(
	pubkeyHash: string | Uint8Array,
	csvContent: string,
	upsert: boolean,
	headers: SignedRequestHeaders
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

export async function createProviderOffering(
	pubkeyHash: string | Uint8Array,
	params: CreateOfferingParams | string,
	headers: SignedRequestHeaders
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
	headers: SignedRequestHeaders
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

// Generate CSV headers from the Offering type structure
function getOfferingsCSVHeader(): string[] {
	// Create a sample Offering object with all possible properties
	const sampleOffering: Offering = {
		id: undefined,
		offering_id: '',
		offer_name: '',
		description: null,
		product_page_url: null,
		currency: '',
		monthly_price: 0,
		setup_fee: 0,
		visibility: '',
		product_type: '',
		virtualization_type: null,
		billing_interval: '',
		stock_status: '',
		processor_brand: null,
		processor_amount: undefined,
		processor_cores: undefined,
		processor_speed: null,
		processor_name: null,
		memory_error_correction: null,
		memory_type: null,
		memory_amount: null,
		hdd_amount: undefined,
		total_hdd_capacity: null,
		ssd_amount: undefined,
		total_ssd_capacity: null,
		unmetered_bandwidth: false,
		uplink_speed: null,
		traffic: undefined,
		datacenter_country: '',
		datacenter_city: '',
		datacenter_latitude: null,
		datacenter_longitude: null,
		control_panel: null,
		gpu_name: null,
		min_contract_hours: undefined,
		max_contract_hours: undefined,
		payment_methods: null,
		features: null,
		operating_systems: null,
		pubkey_hash: ''
	};
	
	// Exclude id and pubkey_hash as they are not part of the CSV export
	return Object.keys(sampleOffering)
		.filter(key => key !== 'id' && key !== 'pubkey_hash')
		.sort(); // Sort to maintain consistent order
}

export function offeringToCSVRow(offering: Offering): string[] {
	const headers = getOfferingsCSVHeader();
	return headers.map(header => {
		const value = offering[header as keyof Offering];
		if (value === null || value === undefined) {
			return '';
		}
		if (typeof value === 'number' || typeof value === 'boolean') {
			return value.toString();
		}
		return value;
	});
}

// CSV header for offerings - generated dynamically
const OFFERINGS_CSV_HEADER = getOfferingsCSVHeader();

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

// ============ Rental Request Endpoints ============

export interface RentalRequestParams {
	offering_db_id: number;
	ssh_pubkey?: string;
	contact_method?: string;
	request_memo?: string;
	duration_hours?: number;
}

export interface RentalRequestResponse {
	contract_id: string;
	message: string;
}

export async function createRentalRequest(
	params: RentalRequestParams,
	headers: SignedRequestHeaders
): Promise<RentalRequestResponse> {
	const url = `${API_BASE_URL}/api/v1/contracts/rental-request`;

	const response = await fetch(url, {
		method: 'POST',
		headers,
		body: JSON.stringify(params)
	});

	if (!response.ok) {
		const errorText = await response.text();
		throw new Error(`Failed to create rental request: ${response.status} ${response.statusText}\n${errorText}`);
	}

	const payload = (await response.json()) as ApiResponse<RentalRequestResponse>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to create rental request');
	}

	if (!payload.data) {
		throw new Error('Rental request response did not include data');
	}

	return payload.data;
}
