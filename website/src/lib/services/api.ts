// Import auto-generated types from Rust (these have pubkey as Vec<u8> which is skipped in TS)
import type { Offering as OfferingRaw } from '$lib/types/generated/Offering';
import type { ProviderProfile as ProviderProfileRaw } from '$lib/types/generated/ProviderProfile';
import type { Validator as ValidatorRaw } from '$lib/types/generated/Validator';
import type { PlatformOverview } from '$lib/types/generated/PlatformOverview';
import type { UserProfile as UserProfileRaw } from '$lib/types/generated/UserProfile';
import type { SignedRequestHeaders } from '$lib/types/generated/SignedRequestHeaders';
import type { ProviderTrustMetrics as ProviderTrustMetricsRaw } from '$lib/types/generated/ProviderTrustMetrics';
import type { ProviderOnboarding as ProviderOnboardingRaw } from '$lib/types/generated/ProviderOnboarding';
import type { ProviderHealthSummary as ProviderHealthSummaryRaw } from '$lib/types/generated/ProviderHealthSummary';
import type { ContractUsage } from '$lib/types/generated/ContractUsage';
import type { PoolCapabilities } from '$lib/types/generated/PoolCapabilities';
import type { OfferingSuggestion } from '$lib/types/generated/OfferingSuggestion';
import type { UnavailableTier } from '$lib/types/generated/UnavailableTier';
import { bytesToHex as hexEncode, normalizePubkey } from '$lib/utils/identity';

// Utility type to convert null to undefined (Rust Option -> TS optional)
type NullToUndefined<T> = T extends null ? undefined : T;
type ConvertNullToUndefined<T> = {
	[K in keyof T]: NullToUndefined<T[K]>;
};

// Frontend types: convert null to undefined for convenience
export type Offering = ConvertNullToUndefined<OfferingRaw> & { pubkey: string };
export type ProviderProfile = ConvertNullToUndefined<ProviderProfileRaw> & { pubkey: string };
export type Validator = ConvertNullToUndefined<ValidatorRaw> & { pubkey: string };
export type UserProfile = ConvertNullToUndefined<UserProfileRaw> & { pubkey: string };
export type PlatformStats = ConvertNullToUndefined<PlatformOverview>;
export type ProviderTrustMetrics = ConvertNullToUndefined<ProviderTrustMetricsRaw>;
export type ProviderOnboarding = ConvertNullToUndefined<ProviderOnboardingRaw>;
export type ProviderHealthSummary = ConvertNullToUndefined<ProviderHealthSummaryRaw>;

// Generic API response wrapper
export interface ApiResponse<T> {
	success: boolean;
	data?: T | null;
	error?: string | null;
}

// Offering search parameters
export interface OfferingSearchParams {
	limit?: number | null;
	offset?: number | null;
	product_type?: string;
	country?: string;
	min_price_monthly?: number | null;
	max_price_monthly?: number | null;
	in_stock_only?: boolean;
	q?: string; // DSL query
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

// Create offering params (omit id, pubkey, and computed fields)
export type CreateOfferingParams = Omit<Offering, 'id' | 'pubkey' | 'resolved_pool_id' | 'resolved_pool_name'>;

// API URLs for different environments
const DEV_API_BASE_URL = 'http://dev-api.decent-cloud.org';
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

// Re-export for backward compatibility
export { hexEncode };

/**
 * Helper to extract error message from response
 * Tries to read as JSON first, falls back to text
 */
async function getErrorMessage(response: Response, defaultMessage: string): Promise<string> {
	try {
		const contentType = response.headers.get('content-type');
		if (contentType?.includes('application/json')) {
			const data = await response.json();
			return data.error || defaultMessage;
		} else {
			const text = await response.text();
			return text || defaultMessage;
		}
	} catch {
		return defaultMessage;
	}
}

export async function searchOfferings(params: OfferingSearchParams = {}): Promise<Offering[]> {
	const searchParams = new URLSearchParams();
	if (params.limit !== undefined && params.limit !== null) searchParams.set('limit', params.limit.toString());
	if (params.offset !== undefined && params.offset !== null) searchParams.set('offset', params.offset.toString());
	if (params.product_type) searchParams.set('product_type', params.product_type);
	if (params.country) searchParams.set('country', params.country);
	if (params.min_price_monthly !== undefined && params.min_price_monthly !== null) searchParams.set('min_price_monthly', params.min_price_monthly.toString());
	if (params.max_price_monthly !== undefined && params.max_price_monthly !== null) searchParams.set('max_price_monthly', params.max_price_monthly.toString());
	if (params.in_stock_only) searchParams.set('in_stock_only', 'true');
	if (params.q) searchParams.set('q', params.q);

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
	// Normalize pubkey to hex string
	return offerings.map((o) => ({
		...o,
		pubkey: normalizePubkey(o.pubkey)
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
	// Normalize pubkey to hex string
	return providers.map((p) => ({
		...p,
		pubkey: normalizePubkey(p.pubkey)
	}));
}

export async function getProviderTrustMetrics(
	pubkey: string | Uint8Array
): Promise<ProviderTrustMetrics> {
	const pubkeyHex = typeof pubkey === 'string' ? pubkey : hexEncode(pubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/trust-metrics`;
	const response = await fetch(url);

	if (!response.ok) {
		throw new Error(`Failed to fetch trust metrics: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<ProviderTrustMetrics>;

	if (!payload.success || !payload.data) {
		throw new Error(payload.error ?? 'Failed to fetch provider trust metrics');
	}

	return payload.data;
}

export interface ResponseTimeDistribution {
	within1hPct: number;
	within4hPct: number;
	within12hPct: number;
	within24hPct: number;
	within72hPct: number;
	totalResponses: number;
}

export interface ProviderResponseMetrics {
	avgResponseSeconds: number | null;
	avgResponseHours: number | null;
	slaCompliancePercent: number;
	breachCount30d: number;
	totalInquiries30d: number;
	distribution: ResponseTimeDistribution;
}

export async function getProviderResponseMetrics(
	pubkey: string | Uint8Array
): Promise<ProviderResponseMetrics> {
	const pubkeyHex = typeof pubkey === 'string' ? pubkey : hexEncode(pubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/response-metrics`;
	const response = await fetch(url);

	if (!response.ok) {
		throw new Error(`Failed to fetch response metrics: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<ProviderResponseMetrics>;

	if (!payload.success || !payload.data) {
		throw new Error(payload.error ?? 'Failed to fetch provider response metrics');
	}

	return payload.data;
}

export async function getProviderHealthSummary(
	pubkey: string | Uint8Array,
	days?: number | null
): Promise<ProviderHealthSummary> {
	const pubkeyHex = typeof pubkey === 'string' ? pubkey : hexEncode(pubkey);
	const searchParams = days !== undefined && days !== null ? `?days=${days}` : '';
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/health-summary${searchParams}`;
	const response = await fetch(url);

	if (!response.ok) {
		throw new Error(`Failed to fetch provider health summary: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<ProviderHealthSummary>;

	if (!payload.success || !payload.data) {
		throw new Error(payload.error ?? 'Failed to fetch provider health summary');
	}

	return payload.data;
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
	// Normalize pubkey to hex string
	return validators.map((v) => ({
		...v,
		pubkey: normalizePubkey(v.pubkey)
	}));
}

export async function getProviderOfferings(pubkey: string | Uint8Array): Promise<Offering[]> {
	const pubkeyHex = typeof pubkey === 'string' ? pubkey : hexEncode(pubkey);
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
	// Normalize pubkey to hex string
	return offerings.map((o) => ({
		...o,
		pubkey: normalizePubkey(o.pubkey)
	}));
}

/**
 * Get the authenticated user's own offerings (all visibilities including private).
 * Requires authentication.
 * Used for "My Resources" UI section to enable self-rental.
 */
export async function getMyOfferings(headers: SignedRequestHeaders): Promise<Offering[]> {
	const url = `${API_BASE_URL}/api/v1/provider/my-offerings`;
	const response = await fetch(url, {
		method: 'GET',
		headers
	});

	if (!response.ok) {
		throw new Error(`Failed to fetch my offerings: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<Offering[]>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Decent Cloud API my offerings response failed');
	}

	const offerings = payload.data ?? [];
	return offerings.map((o) => ({
		...o,
		pubkey: normalizePubkey(o.pubkey)
	}));
}

export async function exportProviderOfferingsCSV(
	pubkey: string | Uint8Array,
	headers: SignedRequestHeaders
): Promise<string> {
	const pubkeyHex = typeof pubkey === 'string' ? pubkey : hexEncode(pubkey);
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
	pubkey: string | Uint8Array,
	csvContent: string,
	upsert: boolean,
	headers: SignedRequestHeaders
): Promise<CsvImportResult> {
	const pubkeyHex = typeof pubkey === 'string' ? pubkey : hexEncode(pubkey);
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
	pubkey: string | Uint8Array,
	params: CreateOfferingParams | string,
	headers: SignedRequestHeaders
): Promise<number> {
	const pubkeyHex = typeof pubkey === 'string' ? pubkey : hexEncode(pubkey);
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
	pubkey: string | Uint8Array,
	offeringId: number,
	params: CreateOfferingParams | string,
	headers: SignedRequestHeaders
): Promise<void> {
	const pubkeyHex = typeof pubkey === 'string' ? pubkey : hexEncode(pubkey);
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

// Visibility Allowlist API functions
export interface AllowlistEntry {
	id: number;
	offering_id: number;
	allowed_pubkey: string;
	created_at: number;
}

export async function getOfferingAllowlist(
	pubkey: string | Uint8Array,
	offeringId: number,
	headers: SignedRequestHeaders
): Promise<AllowlistEntry[]> {
	const pubkeyHex = typeof pubkey === 'string' ? pubkey : hexEncode(pubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/offerings/${offeringId}/allowlist`;

	const response = await fetch(url, {
		method: 'GET',
		headers
	});

	if (!response.ok) {
		const errorText = await response.text();
		throw new Error(`Failed to get allowlist: ${response.status} ${response.statusText}\n${errorText}`);
	}

	const payload = (await response.json()) as ApiResponse<AllowlistEntry[]>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to get allowlist');
	}

	return payload.data ?? [];
}

export async function addToAllowlist(
	pubkey: string | Uint8Array,
	offeringId: number,
	allowedPubkey: string,
	headers: SignedRequestHeaders,
	body: string
): Promise<number> {
	const pubkeyHex = typeof pubkey === 'string' ? pubkey : hexEncode(pubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/offerings/${offeringId}/allowlist`;

	const response = await fetch(url, {
		method: 'POST',
		headers,
		body
	});

	if (!response.ok) {
		const errorText = await response.text();
		throw new Error(`Failed to add to allowlist: ${response.status} ${response.statusText}\n${errorText}`);
	}

	const payload = (await response.json()) as ApiResponse<number>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to add to allowlist');
	}

	return payload.data ?? 0;
}

export async function removeFromAllowlist(
	pubkey: string | Uint8Array,
	offeringId: number,
	allowedPubkey: string,
	headers: SignedRequestHeaders
): Promise<boolean> {
	const pubkeyHex = typeof pubkey === 'string' ? pubkey : hexEncode(pubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/offerings/${offeringId}/allowlist/${allowedPubkey}`;

	const response = await fetch(url, {
		method: 'DELETE',
		headers
	});

	if (!response.ok) {
		const errorText = await response.text();
		throw new Error(`Failed to remove from allowlist: ${response.status} ${response.statusText}\n${errorText}`);
	}

	const payload = (await response.json()) as ApiResponse<boolean>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to remove from allowlist');
	}

	return payload.data ?? false;
}

// Encrypted credentials API
export interface EncryptedCredentials {
	version: number;
	ephemeral_pubkey: string;
	nonce: string;
	ciphertext: string;
}

export async function getContractCredentials(
	contractId: string,
	headers: SignedRequestHeaders
): Promise<string | null> {
	const url = `${API_BASE_URL}/api/v1/contracts/${contractId}/credentials`;

	const response = await fetch(url, {
		method: 'GET',
		headers
	});

	if (!response.ok) {
		if (response.status === 404) {
			return null; // No credentials available
		}
		const errorText = await response.text();
		throw new Error(`Failed to get credentials: ${response.status} ${response.statusText}\n${errorText}`);
	}

	const payload = (await response.json()) as ApiResponse<string>;

	if (!payload.success) {
		if (payload.error?.includes('No credentials available')) {
			return null;
		}
		throw new Error(payload.error ?? 'Failed to get credentials');
	}

	return payload.data ?? null;
}

export async function requestPasswordReset(
	contractId: string,
	headers: SignedRequestHeaders
): Promise<void> {
	const url = `${API_BASE_URL}/api/v1/contracts/${contractId}/reset-password`;

	const response = await fetch(url, {
		method: 'POST',
		headers
	});

	if (!response.ok) {
		const errorText = await response.text();
		throw new Error(`Failed to request password reset: ${response.status} ${response.statusText}\n${errorText}`);
	}

	const payload = (await response.json()) as ApiResponse<string>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to request password reset');
	}
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
		offering.gpu_count?.toString() || '',
		offering.gpu_memory_gb?.toString() || '',
		offering.min_contract_hours?.toString() || '',
		offering.max_contract_hours?.toString() || '',
		offering.payment_methods || '',
		offering.features || '',
		offering.operating_systems || '',
		offering.agent_pool_id || '',
		offering.template_name || '',
		offering.provisioner_type || '',
		offering.provisioner_config || ''
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
	'gpu_count',
	'gpu_memory_gb',
	'min_contract_hours',
	'max_contract_hours',
	'payment_methods',
	'features',
	'operating_systems',
	'agent_pool_id',
	'template_name',
	'provisioner_type',
	'provisioner_config'
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

// Product type information
export interface ProductType {
	key: string;
	label: string;
}

/**
 * Get available product types with their labels
 * @returns Array of product types
 */
export async function getProductTypes(): Promise<ProductType[]> {
	const response = await fetch(`${API_BASE_URL}/api/v1/offerings/product-types`, {
		method: 'GET',
		headers: {
			'Content-Type': 'application/json',
		},
	});

	if (!response.ok) {
		const error = await response.text();
		throw new Error(`Failed to fetch product types: ${error}`);
	}

	const result: ApiResponse<ProductType[]> = await response.json();
	if (!result.success || !result.data) {
		throw new Error(result.error || 'Failed to fetch product types');
	}

	return result.data;
}

/**
 * Get example offerings template CSV for a specific product type
 * @param productType - The product type key (e.g., 'compute', 'gpu', 'storage', 'network', 'dedicated')
 * @returns CSV string with example offerings
 */
export async function getExampleOfferingsCSV(productType: string): Promise<string> {
	const response = await fetch(`${API_BASE_URL}/api/v1/offerings/template/${productType}`, {
		method: 'GET',
	});

	if (!response.ok) {
		const error = await response.text();
		throw new Error(`Failed to fetch example offerings: ${error}`);
	}

	return await response.text();
}

// ============ Rental Request Endpoints ============

export interface Contract {
	contract_id: string;
	requester_pubkey: string;
	requester_ssh_pubkey: string;
	requester_contact: string;
	provider_pubkey: string;
	offering_id: string;
	region_name?: string;
	instance_config?: string;
	payment_amount_e9s: number;
	start_timestamp_ns?: number;
	end_timestamp_ns?: number;
	duration_hours?: number;
	original_duration_hours?: number;
	request_memo: string;
	created_at_ns: number;
	status: string;
	provisioning_instance_details?: string;
	provisioning_completed_at_ns?: number;
	payment_method: string;
	stripe_payment_intent_id?: string;
	stripe_customer_id?: string;
	payment_status: string;
	currency: string;
	refund_amount_e9s?: number;
	stripe_refund_id?: string;
	icpay_refund_id?: string;
	refund_created_at_ns?: number;
	status_updated_at_ns?: number;
	// Subscription fields
	stripe_subscription_id?: string;
	subscription_status?: string;
	current_period_end_ns?: number;
	cancel_at_period_end?: boolean;
	// Gateway fields
	gateway_slug?: string;
	gateway_subdomain?: string;
	gateway_ssh_port?: number;
	gateway_port_range_start?: number;
	gateway_port_range_end?: number;
}

export interface RentalRequestParams {
	offering_db_id: number;
	ssh_pubkey?: string;
	contact_method?: string;
	request_memo?: string;
	duration_hours?: number;
	payment_method?: string;
	/** Buyer address for B2B invoices (street, city, postal code, country) */
	buyer_address?: string;
}

export interface RentalRequestResponse {
	contractId: string;
	message: string;
	clientSecret?: string;
	checkoutUrl?: string;
}

export interface ProviderRentalResponseParams {
	accept: boolean;
	memo?: string;
}

export interface ProvisioningStatusUpdateParams {
	status: string;
	instanceDetails?: string;
}

export async function createRentalRequest(
	params: RentalRequestParams,
	headers: SignedRequestHeaders
): Promise<RentalRequestResponse> {
	const url = `${API_BASE_URL}/api/v1/contracts`;

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

/**
 * Update ICPay transaction ID for a contract after payment completes
 */
export async function updateIcpayTransactionId(
	contractId: string,
	transactionId: string,
	headers: SignedRequestHeaders
): Promise<void> {
	const url = `${API_BASE_URL}/api/v1/contracts/${contractId}/icpay-transaction`;

	const response = await fetch(url, {
		method: 'PUT',
		headers,
		body: JSON.stringify({ transaction_id: transactionId })
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, 'Failed to update ICPay transaction ID');
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<string>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to update ICPay transaction ID');
	}
}

export async function getUserContracts(headers: SignedRequestHeaders, pubkeyHex?: string): Promise<Contract[]> {
	if (!pubkeyHex) {
		const pubkey = headers['X-Public-Key'];
		if (!pubkey) {
			throw new Error('Public key is required to fetch user contracts');
		}
		pubkeyHex = pubkey;
	}

	const url = `${API_BASE_URL}/api/v1/users/${pubkeyHex}/contracts`;

	const response = await fetch(url, {
		method: 'GET',
		headers
	});

	if (!response.ok) {
		throw new Error(`Failed to fetch user contracts: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<Contract[]>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to fetch user contracts');
	}

	return payload.data ?? [];
}

export async function getProviderContracts(
	headers: SignedRequestHeaders,
	providerHex: string
): Promise<Contract[]> {
	const url = `${API_BASE_URL}/api/v1/providers/${providerHex}/contracts`;

	const response = await fetch(url, {
		method: 'GET',
		headers
	});

	if (!response.ok) {
		throw new Error(`Failed to fetch provider contracts: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<Contract[]>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to fetch provider contracts');
	}

	return payload.data ?? [];
}

export async function getPendingProviderRequests(headers: SignedRequestHeaders): Promise<Contract[]> {
	const url = `${API_BASE_URL}/api/v1/provider/rental-requests/pending`;

	const response = await fetch(url, {
		method: 'GET',
		headers
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to fetch pending rental requests: ${response.status} ${response.statusText}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<Contract[]>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to fetch pending rental requests');
	}

	return payload.data ?? [];
}

export async function respondToRentalRequest(
	contractIdHex: string,
	params: ProviderRentalResponseParams,
	headers: SignedRequestHeaders
): Promise<string> {
	const url = `${API_BASE_URL}/api/v1/provider/rental-requests/${contractIdHex}/respond`;

	const response = await fetch(url, {
		method: 'POST',
		headers,
		body: JSON.stringify(params)
	});

	if (!response.ok) {
		const errorText = await response.text();
		throw new Error(
			`Failed to respond to rental request: ${response.status} ${response.statusText}\n${errorText}`
		);
	}

	const payload = (await response.json()) as ApiResponse<string>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to respond to rental request');
	}

	if (!payload.data) {
		throw new Error('Rental response did not include confirmation message');
	}

	return payload.data;
}

export async function updateProvisioningStatus(
	contractIdHex: string,
	params: ProvisioningStatusUpdateParams,
	headers: SignedRequestHeaders
): Promise<string> {
	const url = `${API_BASE_URL}/api/v1/provider/rental-requests/${contractIdHex}/provisioning`;

	const response = await fetch(url, {
		method: 'PUT',
		headers,
		body: JSON.stringify(params)
	});

	if (!response.ok) {
		const errorText = await response.text();
		throw new Error(
			`Failed to update provisioning status: ${response.status} ${response.statusText}\n${errorText}`
		);
	}

	const payload = (await response.json()) as ApiResponse<string>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to update provisioning status');
	}

	if (!payload.data) {
		throw new Error('Provisioning status response did not include confirmation message');
	}

	return payload.data;
}

export async function cancelRentalRequest(
	contractIdHex: string,
	params: { memo?: string },
	headers: SignedRequestHeaders
): Promise<string> {
	const url = `${API_BASE_URL}/api/v1/contracts/${contractIdHex}/cancel`;
	const response = await fetch(url, {
		method: 'PUT',
		headers,
		body: JSON.stringify(params)
	});

	if (!response.ok) {
		const errorText = await response.text();
		throw new Error(`Failed to cancel rental request: ${response.status} ${response.statusText}\n${errorText}`);
	}

	const payload = (await response.json()) as ApiResponse<string>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to cancel rental request');
	}

	if (!payload.data) {
		throw new Error('Cancel rental request response did not include confirmation message');
	}

	return payload.data;
}

export interface VerifyCheckoutResponse {
	contractId: string;
	paymentStatus: string;
}

export async function verifyCheckoutSession(sessionId: string): Promise<VerifyCheckoutResponse> {
	const url = `${API_BASE_URL}/api/v1/contracts/verify-checkout`;
	const response = await fetch(url, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ sessionId })
	});

	if (!response.ok) {
		const errorText = await response.text();
		throw new Error(`Failed to verify checkout: ${response.status} ${response.statusText}\n${errorText}`);
	}

	const payload = (await response.json()) as ApiResponse<VerifyCheckoutResponse>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Payment not yet completed');
	}

	if (!payload.data) {
		throw new Error('Verify checkout response did not include data');
	}

	return payload.data;
}

/**
 * Get current usage for a contract
 */
export async function getContractUsage(
	contractId: string,
	headers: SignedRequestHeaders
): Promise<ContractUsage | null> {
	const url = `${API_BASE_URL}/api/v1/contracts/${contractId}/usage`;
	const response = await fetch(url, {
		method: 'GET',
		headers
	});

	if (!response.ok) {
		if (response.status === 404) {
			return null;
		}
		throw new Error(`Failed to fetch contract usage: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<ContractUsage>;

	if (!payload.success) {
		// No usage data is not an error for contracts without usage tracking
		if (payload.error?.includes('No active billing period')) {
			return null;
		}
		throw new Error(payload.error ?? 'Failed to fetch contract usage');
	}

	return payload.data ?? null;
}

export { type ContractUsage };

/**
 * Get recipe execution log for a contract
 */
export async function getContractRecipeLog(
	contractId: string,
	headers: SignedRequestHeaders
): Promise<string | null> {
	const url = `${API_BASE_URL}/api/v1/contracts/${contractId}/recipe-log`;
	const response = await fetch(url, {
		method: 'GET',
		headers
	});

	if (!response.ok) {
		if (response.status === 404) {
			return null;
		}
		throw new Error(`Failed to fetch recipe log: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<string | null>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to fetch recipe log');
	}

	return payload.data ?? null;
}

export async function getProviderOnboarding(pubkey: string): Promise<ProviderOnboarding | null> {
	const pubkeyHex = normalizePubkey(pubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/onboarding`;
	const response = await fetch(url);

	if (!response.ok) {
		if (response.status === 404) {
			return null;
		}
		throw new Error(`Failed to fetch provider onboarding: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<ProviderOnboarding>;

	// No onboarding data yet is not an error - return null to show empty form
	if (!payload.success && !payload.data) {
		return null;
	}

	return payload.data ?? null;
}

export async function updateProviderOnboarding(
	pubkey: string | number[],
	data: Partial<ProviderOnboarding>,
	headers: Record<string, string>
): Promise<{ onboarding_completed_at: number }> {
	const pubkeyHex = normalizePubkey(pubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/onboarding`;
	const response = await fetch(url, {
		method: 'PUT',
		headers: {
			'Content-Type': 'application/json',
			...headers
		},
		body: JSON.stringify(data)
	});

	if (!response.ok) {
		throw new Error(`Failed to update provider onboarding: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<{ onboarding_completed_at: number }>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to update provider onboarding');
	}

	if (!payload.data) {
		throw new Error('Update onboarding response did not include data');
	}

	return payload.data;
}

export async function syncProviderHelpcenter(
	pubkey: string | number[],
	headers: Record<string, string>
): Promise<{ articleUrl: string; action: string }> {
	const pubkeyHex = normalizePubkey(pubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/helpcenter/sync`;
	const response = await fetch(url, {
		method: 'POST',
		headers
	});

	if (!response.ok) {
		throw new Error(`Failed to sync help center: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<{ articleUrl: string; action: string }>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to sync help center');
	}

	if (!payload.data) {
		throw new Error('Sync help center response did not include data');
	}

	return payload.data;
}

// ============ Reseller Endpoints ============

export interface ExternalProvider {
	pubkey: string;
	name: string;
	domain: string;
	website_url: string;
	logo_url?: string;
	data_source: string;
	offerings_count: number;
}

export interface ResellerRelationship {
	id: number;
	reseller_pubkey: string;
	external_provider_pubkey: string;
	commission_percent: number;
	status: string;
	created_at_ns: number;
	updated_at_ns?: number;
}

export interface ResellerOrder {
	id: number;
	contract_id: string;
	reseller_pubkey: string;
	external_provider_pubkey: string;
	offering_id: number;
	base_price_e9s: number;
	commission_e9s: number;
	total_paid_e9s: number;
	external_order_id?: string;
	external_order_details?: string;
	status: string;
	created_at_ns: number;
	fulfilled_at_ns?: number;
}

export async function getExternalProviders(): Promise<ExternalProvider[]> {
	const url = `${API_BASE_URL}/api/v1/reseller/external-providers`;
	const response = await fetch(url);

	if (!response.ok) {
		throw new Error(`Failed to fetch external providers: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<ExternalProvider[]>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to fetch external providers');
	}

	return payload.data ?? [];
}

export async function getResellerRelationships(headers: SignedRequestHeaders): Promise<ResellerRelationship[]> {
	const url = `${API_BASE_URL}/api/v1/reseller/relationships`;
	const response = await fetch(url, {
		method: 'GET',
		headers
	});

	if (!response.ok) {
		throw new Error(`Failed to fetch reseller relationships: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<ResellerRelationship[]>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to fetch reseller relationships');
	}

	return payload.data ?? [];
}

export interface CreateResellerRelationshipParams {
	external_provider_pubkey: string;
	commission_percent: number;
}

export async function createResellerRelationship(
	params: CreateResellerRelationshipParams,
	headers: SignedRequestHeaders
): Promise<ResellerRelationship> {
	const url = `${API_BASE_URL}/api/v1/reseller/relationships`;
	const response = await fetch(url, {
		method: 'POST',
		headers,
		body: JSON.stringify(params)
	});

	if (!response.ok) {
		const errorText = await response.text();
		throw new Error(`Failed to create reseller relationship: ${response.status} ${response.statusText}\n${errorText}`);
	}

	const payload = (await response.json()) as ApiResponse<ResellerRelationship>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to create reseller relationship');
	}

	if (!payload.data) {
		throw new Error('Create reseller relationship response did not include data');
	}

	return payload.data;
}

export interface UpdateResellerRelationshipParams {
	commission_percent?: number;
	status?: string;
}

export async function updateResellerRelationship(
	external_provider_pubkey: string,
	params: UpdateResellerRelationshipParams,
	headers: SignedRequestHeaders
): Promise<void> {
	const url = `${API_BASE_URL}/api/v1/reseller/relationships/${external_provider_pubkey}`;
	const response = await fetch(url, {
		method: 'PUT',
		headers,
		body: JSON.stringify(params)
	});

	if (!response.ok) {
		const errorText = await response.text();
		throw new Error(`Failed to update reseller relationship: ${response.status} ${response.statusText}\n${errorText}`);
	}

	const payload = (await response.json()) as ApiResponse<void>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to update reseller relationship');
	}
}

export async function deleteResellerRelationship(
	external_provider_pubkey: string,
	headers: SignedRequestHeaders
): Promise<void> {
	const url = `${API_BASE_URL}/api/v1/reseller/relationships/${external_provider_pubkey}`;
	const response = await fetch(url, {
		method: 'DELETE',
		headers
	});

	if (!response.ok) {
		const errorText = await response.text();
		throw new Error(`Failed to delete reseller relationship: ${response.status} ${response.statusText}\n${errorText}`);
	}

	const payload = (await response.json()) as ApiResponse<void>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to delete reseller relationship');
	}
}

export async function getResellerOrders(
	headers: SignedRequestHeaders,
	status?: string
): Promise<ResellerOrder[]> {
	const url = status
		? `${API_BASE_URL}/api/v1/reseller/orders?status=${encodeURIComponent(status)}`
		: `${API_BASE_URL}/api/v1/reseller/orders`;
	const response = await fetch(url, {
		method: 'GET',
		headers
	});

	if (!response.ok) {
		throw new Error(`Failed to fetch reseller orders: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<ResellerOrder[]>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to fetch reseller orders');
	}

	return payload.data ?? [];
}

export interface FulfillResellerOrderParams {
	external_order_id: string;
	external_order_details?: string;
}

export async function fulfillResellerOrder(
	contract_id: string,
	params: FulfillResellerOrderParams,
	headers: SignedRequestHeaders
): Promise<void> {
	const url = `${API_BASE_URL}/api/v1/reseller/orders/${contract_id}/fulfill`;
	const response = await fetch(url, {
		method: 'POST',
		headers,
		body: JSON.stringify(params)
	});

	if (!response.ok) {
		const errorText = await response.text();
		throw new Error(`Failed to fulfill reseller order: ${response.status} ${response.statusText}\n${errorText}`);
	}

	const payload = (await response.json()) as ApiResponse<void>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to fulfill reseller order');
	}
}

// ============ Billing Settings Endpoints ============

export interface BillingSettings {
	billingAddress?: string;
	billingVatId?: string;
	billingCountryCode?: string;
}

export async function getBillingSettings(headers: SignedRequestHeaders): Promise<BillingSettings> {
	const url = `${API_BASE_URL}/api/v1/accounts/billing`;
	const response = await fetch(url, {
		method: 'GET',
		headers
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to fetch billing settings: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<BillingSettings>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to fetch billing settings');
	}

	return payload.data ?? {};
}

export async function updateBillingSettings(
	settings: BillingSettings,
	headers: SignedRequestHeaders
): Promise<BillingSettings> {
	const url = `${API_BASE_URL}/api/v1/accounts/billing`;
	const response = await fetch(url, {
		method: 'PUT',
		headers,
		body: JSON.stringify(settings)
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to update billing settings: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<BillingSettings>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to update billing settings');
	}

	return payload.data ?? {};
}

// ============ VAT Validation Endpoints ============

export interface VatValidationResult {
	valid: boolean;
	name?: string;
	address?: string;
	error?: string;
}

export async function validateVatId(countryCode: string, vatNumber: string): Promise<VatValidationResult> {
	const url = `${API_BASE_URL}/api/v1/vat/validate`;
	const response = await fetch(url, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ country_code: countryCode, vat_number: vatNumber })
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `VAT validation failed: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<VatValidationResult>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'VAT validation failed');
	}

	return payload.data ?? { valid: false };
}

// ============ Invoice Endpoints ============

export async function downloadContractInvoice(
	contractId: string,
	headers: SignedRequestHeaders
): Promise<void> {
	const url = `${API_BASE_URL}/api/v1/contracts/${contractId}/invoice`;
	const response = await fetch(url, {
		method: 'GET',
		headers: {
			...headers,
			'Accept': 'application/pdf'
		}
	});

	if (!response.ok) {
		const errorText = await response.text();
		throw new Error(`Failed to download invoice: ${response.status} ${response.statusText}\n${errorText}`);
	}

	const blob = await response.blob();
	const downloadUrl = URL.createObjectURL(blob);
	const link = document.createElement('a');
	link.setAttribute('href', downloadUrl);
	link.setAttribute('download', `invoice-${contractId}.pdf`);
	link.style.visibility = 'hidden';
	document.body.appendChild(link);
	link.click();
	document.body.removeChild(link);
	URL.revokeObjectURL(downloadUrl);
}

// ============ Agent Delegation and Status Endpoints ============

import type { AgentStatus as AgentStatusRaw } from '$lib/types/generated/AgentStatus';
import type { AgentDelegation as AgentDelegationRaw } from '$lib/types/generated/AgentDelegation';

export type AgentStatus = ConvertNullToUndefined<AgentStatusRaw>;
export type AgentDelegation = ConvertNullToUndefined<AgentDelegationRaw>;

export async function getProviderAgentStatus(pubkey: string | Uint8Array): Promise<AgentStatus | null> {
	const pubkeyHex = typeof pubkey === 'string' ? pubkey : hexEncode(pubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/agent-status`;
	const response = await fetch(url);

	if (!response.ok) {
		if (response.status === 404) {
			return null;
		}
		throw new Error(`Failed to fetch agent status: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<AgentStatus>;

	if (!payload.success) {
		// No agent status is not an error - return null
		return null;
	}

	return payload.data ?? null;
}

export async function getProviderAgentDelegations(
	pubkey: string | Uint8Array,
	headers: SignedRequestHeaders
): Promise<AgentDelegation[]> {
	const pubkeyHex = typeof pubkey === 'string' ? pubkey : hexEncode(pubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/agent-delegations`;

	const response = await fetch(url, {
		method: 'GET',
		headers
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to fetch agent delegations: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<AgentDelegation[]>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to fetch agent delegations');
	}

	return payload.data ?? [];
}

export async function revokeAgentDelegation(
	providerPubkey: string | Uint8Array,
	agentPubkey: string,
	headers: SignedRequestHeaders
): Promise<boolean> {
	const pubkeyHex = typeof providerPubkey === 'string' ? providerPubkey : hexEncode(providerPubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/agent-delegations/${agentPubkey}`;

	const response = await fetch(url, {
		method: 'DELETE',
		headers
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to revoke agent delegation: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<boolean>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to revoke agent delegation');
	}

	return payload.data ?? false;
}

export async function updateAgentDelegationLabel(
	providerPubkey: string | Uint8Array,
	agentPubkey: string,
	label: string,
	headers: SignedRequestHeaders
): Promise<boolean> {
	const pubkeyHex = typeof providerPubkey === 'string' ? providerPubkey : hexEncode(providerPubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/agent-delegations/${agentPubkey}/label`;

	const response = await fetch(url, {
		method: 'PUT',
		headers: {
			...headers,
			'Content-Type': 'application/json'
		},
		body: JSON.stringify({ label })
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to update agent label: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<boolean>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to update agent label');
	}

	return payload.data ?? false;
}

// ==================== Agent Pool APIs ====================

import type { AgentPool } from '$lib/types/generated/AgentPool';
import type { AgentPoolWithStats } from '$lib/types/generated/AgentPoolWithStats';
import type { SetupToken } from '$lib/types/generated/SetupToken';

export interface CreatePoolParams {
	name: string;
	location: string;
	provisionerType: string;
}

export interface UpdatePoolParams {
	name?: string;
	location?: string;
	provisionerType?: string;
}

export interface CreateSetupTokenParams {
	label?: string;
	expiresInHours?: number;
}

export async function listAgentPools(
	providerPubkey: string | Uint8Array,
	headers: SignedRequestHeaders
): Promise<AgentPoolWithStats[]> {
	const pubkeyHex = typeof providerPubkey === 'string' ? providerPubkey : hexEncode(providerPubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/pools`;

	const response = await fetch(url, { headers });

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to fetch agent pools: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<AgentPoolWithStats[]>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to fetch agent pools');
	}

	return payload.data ?? [];
}

export async function createAgentPool(
	providerPubkey: string | Uint8Array,
	params: CreatePoolParams,
	headers: SignedRequestHeaders
): Promise<AgentPool> {
	const pubkeyHex = typeof providerPubkey === 'string' ? providerPubkey : hexEncode(providerPubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/pools`;

	const body = JSON.stringify({
		name: params.name,
		location: params.location,
		provisionerType: params.provisionerType
	});

	const response = await fetch(url, {
		method: 'POST',
		headers: { ...headers, 'Content-Type': 'application/json' },
		body
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to create agent pool: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<AgentPool>;

	if (!payload.success || !payload.data) {
		throw new Error(payload.error ?? 'Failed to create agent pool');
	}

	return payload.data;
}

export async function updateAgentPool(
	providerPubkey: string | Uint8Array,
	poolId: string,
	params: UpdatePoolParams,
	headers: SignedRequestHeaders
): Promise<boolean> {
	const pubkeyHex = typeof providerPubkey === 'string' ? providerPubkey : hexEncode(providerPubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/pools/${poolId}`;

	const body = JSON.stringify({
		name: params.name,
		location: params.location,
		provisionerType: params.provisionerType
	});

	const response = await fetch(url, {
		method: 'PUT',
		headers: { ...headers, 'Content-Type': 'application/json' },
		body
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to update agent pool: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<boolean>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to update agent pool');
	}

	return payload.data ?? false;
}

export async function deleteAgentPool(
	providerPubkey: string | Uint8Array,
	poolId: string,
	headers: SignedRequestHeaders
): Promise<boolean> {
	const pubkeyHex = typeof providerPubkey === 'string' ? providerPubkey : hexEncode(providerPubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/pools/${poolId}`;

	const response = await fetch(url, {
		method: 'DELETE',
		headers
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to delete agent pool: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<boolean>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to delete agent pool');
	}

	return payload.data ?? false;
}

export async function getAgentPoolDetails(
	providerPubkey: string | Uint8Array,
	poolId: string,
	headers: SignedRequestHeaders
): Promise<AgentPoolWithStats> {
	const pubkeyHex = typeof providerPubkey === 'string' ? providerPubkey : hexEncode(providerPubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/pools/${poolId}`;

	const response = await fetch(url, { headers });

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to fetch agent pool details: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<AgentPoolWithStats>;

	if (!payload.success || !payload.data) {
		throw new Error(payload.error ?? 'Failed to fetch agent pool details');
	}

	return payload.data;
}

export async function listAgentsInPool(
	providerPubkey: string | Uint8Array,
	poolId: string,
	headers: SignedRequestHeaders
): Promise<AgentDelegation[]> {
	const pubkeyHex = typeof providerPubkey === 'string' ? providerPubkey : hexEncode(providerPubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/pools/${poolId}/agents`;

	const response = await fetch(url, { headers });

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to fetch agents in pool: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<AgentDelegation[]>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to fetch agents in pool');
	}

	return payload.data ?? [];
}

// ==================== Setup Token APIs ====================

export async function listSetupTokens(
	providerPubkey: string | Uint8Array,
	poolId: string,
	headers: SignedRequestHeaders
): Promise<SetupToken[]> {
	const pubkeyHex = typeof providerPubkey === 'string' ? providerPubkey : hexEncode(providerPubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/pools/${poolId}/setup-tokens`;

	const response = await fetch(url, { headers });

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to fetch setup tokens: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<SetupToken[]>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to fetch setup tokens');
	}

	return payload.data ?? [];
}

export async function createSetupToken(
	providerPubkey: string | Uint8Array,
	poolId: string,
	params: CreateSetupTokenParams,
	headers: SignedRequestHeaders
): Promise<SetupToken> {
	const pubkeyHex = typeof providerPubkey === 'string' ? providerPubkey : hexEncode(providerPubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/pools/${poolId}/setup-tokens`;

	const body = JSON.stringify({
		label: params.label,
		expiresInHours: params.expiresInHours ?? 24
	});

	const response = await fetch(url, {
		method: 'POST',
		headers: { ...headers, 'Content-Type': 'application/json' },
		body
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to create setup token: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<SetupToken>;

	if (!payload.success || !payload.data) {
		throw new Error(payload.error ?? 'Failed to create setup token');
	}

	return payload.data;
}

export async function deleteSetupToken(
	providerPubkey: string | Uint8Array,
	poolId: string,
	token: string,
	headers: SignedRequestHeaders
): Promise<boolean> {
	const pubkeyHex = typeof providerPubkey === 'string' ? providerPubkey : hexEncode(providerPubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/pools/${poolId}/setup-tokens/${token}`;

	const response = await fetch(url, {
		method: 'DELETE',
		headers
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to delete setup token: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<boolean>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to delete setup token');
	}

	return payload.data ?? false;
}

// ==================== Subscription API ====================

/** Subscription plan definition */
export interface SubscriptionPlan {
	id: string;
	name: string;
	description?: string;
	stripe_price_id?: string;
	monthlyPriceCents: number;
	trialDays: number;
	features?: string; // JSON array stored as string
}

/** Account subscription details */
export interface AccountSubscription {
	plan_id: string;
	plan_name: string;
	status: string;
	stripe_subscription_id?: string;
	current_period_end?: number;
	cancel_at_period_end: boolean;
	features: string[];
}

/** Checkout URL response */
export interface CheckoutUrlResponse {
	checkout_url: string;
}

/** Portal URL response */
export interface PortalUrlResponse {
	portal_url: string;
}

/**
 * List all available subscription plans
 * No authentication required
 */
export async function listSubscriptionPlans(): Promise<SubscriptionPlan[]> {
	const url = `${API_BASE_URL}/api/v1/subscriptions/plans`;
	const response = await fetch(url);

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to fetch plans: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<SubscriptionPlan[]>;

	if (!payload.success || !payload.data) {
		throw new Error(payload.error ?? 'Failed to fetch subscription plans');
	}

	return payload.data;
}

/**
 * Get current user's subscription details
 */
export async function getCurrentSubscription(
	headers: SignedRequestHeaders
): Promise<AccountSubscription> {
	const url = `${API_BASE_URL}/api/v1/subscriptions/current`;
	const response = await fetch(url, {
		method: 'GET',
		headers
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(
			response,
			`Failed to get subscription: ${response.status}`
		);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<AccountSubscription>;

	if (!payload.success || !payload.data) {
		throw new Error(payload.error ?? 'Failed to get subscription');
	}

	return payload.data;
}

/**
 * Create a Stripe Checkout session to subscribe to a plan
 * Returns checkout URL to redirect user to
 */
export async function createSubscriptionCheckout(
	planId: string,
	headers: SignedRequestHeaders
): Promise<string> {
	const url = `${API_BASE_URL}/api/v1/subscriptions/checkout`;
	const response = await fetch(url, {
		method: 'POST',
		headers: { ...headers, 'Content-Type': 'application/json' },
		body: JSON.stringify({ plan_id: planId })
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(
			response,
			`Failed to create checkout: ${response.status}`
		);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<CheckoutUrlResponse>;

	if (!payload.success || !payload.data) {
		throw new Error(payload.error ?? 'Failed to create checkout session');
	}

	return payload.data.checkout_url;
}

/**
 * Create a Stripe Billing Portal session for self-service subscription management
 * Returns portal URL to redirect user to
 */
export async function createBillingPortal(headers: SignedRequestHeaders): Promise<string> {
	const url = `${API_BASE_URL}/api/v1/subscriptions/portal`;
	const response = await fetch(url, {
		method: 'POST',
		headers
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(
			response,
			`Failed to create portal session: ${response.status}`
		);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<PortalUrlResponse>;

	if (!payload.success || !payload.data) {
		throw new Error(payload.error ?? 'Failed to create billing portal session');
	}

	return payload.data.portal_url;
}

/**
 * Cancel subscription
 * @param atPeriodEnd If true, cancel at end of billing period; if false, cancel immediately
 */
export async function cancelSubscription(
	atPeriodEnd: boolean,
	headers: SignedRequestHeaders
): Promise<AccountSubscription> {
	const url = `${API_BASE_URL}/api/v1/subscriptions/cancel`;
	const response = await fetch(url, {
		method: 'POST',
		headers: { ...headers, 'Content-Type': 'application/json' },
		body: JSON.stringify({ at_period_end: atPeriodEnd })
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(
			response,
			`Failed to cancel subscription: ${response.status}`
		);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<AccountSubscription>;

	if (!payload.success || !payload.data) {
		throw new Error(payload.error ?? 'Failed to cancel subscription');
	}

	return payload.data;
}

// ==================== Bandwidth Stats API ====================

import type { BandwidthStatsResponse } from '$lib/types/generated/BandwidthStatsResponse';
import type { BandwidthHistoryResponse } from '$lib/types/generated/BandwidthHistoryResponse';

export type { BandwidthStatsResponse, BandwidthHistoryResponse };

/**
 * Get bandwidth stats for all provider's contracts
 * Requires provider authentication
 */
export async function getProviderBandwidthStats(
	providerPubkey: string | Uint8Array,
	headers: SignedRequestHeaders
): Promise<BandwidthStatsResponse[]> {
	const pubkeyHex = typeof providerPubkey === 'string' ? providerPubkey : hexEncode(providerPubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/bandwidth`;

	const response = await fetch(url, {
		method: 'GET',
		headers
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(
			response,
			`Failed to fetch bandwidth stats: ${response.status}`
		);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<BandwidthStatsResponse[]>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to fetch bandwidth stats');
	}

	return payload.data ?? [];
}

/**
 * Get bandwidth history for a specific contract
 * Requires provider authentication
 */
export async function getContractBandwidthHistory(
	providerPubkey: string | Uint8Array,
	contractId: string,
	headers: SignedRequestHeaders
): Promise<BandwidthHistoryResponse[]> {
	const pubkeyHex = typeof providerPubkey === 'string' ? providerPubkey : hexEncode(providerPubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/contracts/${contractId}/bandwidth`;

	const response = await fetch(url, {
		method: 'GET',
		headers
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(
			response,
			`Failed to fetch bandwidth history: ${response.status}`
		);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<BandwidthHistoryResponse[]>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to fetch bandwidth history');
	}

	return payload.data ?? [];
}

// ==================== Offering Generation Types ====================

/** Response from offering suggestions endpoint */
export interface OfferingSuggestionsResponse {
	poolCapabilities: PoolCapabilities;
	suggestedOfferings: OfferingSuggestion[];
	unavailableTiers: UnavailableTier[];
}

/** Pricing configuration for a single tier */
export interface TierPricing {
	monthlyPrice: number;
	currency: string;
}

/** Request to generate offerings from pool capabilities */
export interface GenerateOfferingsRequest {
	tiers?: string[];
	pricing: Record<string, TierPricing>;
	visibility?: string;
	dryRun?: boolean;
}

/** Response from offering generation endpoint */
export interface GenerateOfferingsResponse {
	createdOfferings: Offering[];
	skippedTiers: UnavailableTier[];
}

// Re-export types for convenience
export type { PoolCapabilities, OfferingSuggestion, UnavailableTier };

// ==================== Offering Generation API Functions ====================

/**
 * Get offering suggestions for a pool based on its hardware capabilities
 * Requires provider authentication
 */
export async function getOfferingSuggestions(
	providerPubkey: string | Uint8Array,
	poolId: string,
	headers: SignedRequestHeaders
): Promise<OfferingSuggestionsResponse> {
	const pubkeyHex = typeof providerPubkey === 'string' ? providerPubkey : hexEncode(providerPubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/pools/${poolId}/offering-suggestions`;

	const response = await fetch(url, {
		method: 'GET',
		headers
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(
			response,
			`Failed to fetch offering suggestions: ${response.status}`
		);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<OfferingSuggestionsResponse>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to fetch offering suggestions');
	}

	if (!payload.data) {
		throw new Error('No data returned from offering suggestions');
	}

	return payload.data;
}

/**
 * Generate offerings for a pool with provided pricing
 * Requires provider authentication
 */
export async function generateOfferings(
	providerPubkey: string | Uint8Array,
	poolId: string,
	request: GenerateOfferingsRequest,
	headers: SignedRequestHeaders
): Promise<GenerateOfferingsResponse> {
	const pubkeyHex = typeof providerPubkey === 'string' ? providerPubkey : hexEncode(providerPubkey);
	const url = `${API_BASE_URL}/api/v1/providers/${pubkeyHex}/pools/${poolId}/generate-offerings`;

	const response = await fetch(url, {
		method: 'POST',
		headers: {
			...headers,
			'Content-Type': 'application/json'
		},
		body: JSON.stringify(request)
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(
			response,
			`Failed to generate offerings: ${response.status}`
		);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<GenerateOfferingsResponse>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to generate offerings');
	}

	if (!payload.data) {
		throw new Error('No data returned from offering generation');
	}

	return payload.data;
}

// ==================== Cloud Self-Provisioning API ====================

export interface CloudAccount {
	id: string;
	accountId: string;
	backendType: string;
	name: string;
	config?: string;
	isValid: boolean;
	lastValidatedAt?: string;
	validationError?: string;
	createdAt: string;
	updatedAt: string;
}

export interface CloudAccountListResponse {
	accounts: CloudAccount[];
}

export interface AddCloudAccountRequest {
	backendType: string;
	name: string;
	credentials: string;
	config?: string;
}

export interface BackendCatalog {
	serverTypes: ServerType[];
	locations: Location[];
	images: Image[];
}

export interface ServerType {
	id: string;
	name: string;
	cores: number;
	memoryGb: number;
	diskGb: number;
	priceMonthly?: number;
	priceHourly?: number;
}

export interface Location {
	id: string;
	name: string;
	city: string;
	country: string;
}

export interface Image {
	id: string;
	name: string;
	osType: string;
	osVersion?: string;
}

export interface CloudResource {
	id: string;
	cloudAccountId: string;
	externalId: string;
	name: string;
	serverType: string;
	location: string;
	image: string;
	sshPubkey: string;
	status: string;
	publicIp?: string;
	sshPort: number;
	sshUsername: string;
	externalSshKeyId?: string;
	gatewaySlug?: string;
	gatewaySubdomain?: string;
	gatewaySshPort?: number;
	gatewayPortRangeStart?: number;
	gatewayPortRangeEnd?: number;
	offeringId?: number;
	listingMode: string;
	errorMessage?: string;
	platformFeeE9s: number;
	createdAt: string;
	updatedAt: string;
	terminatedAt?: string;
}

export interface CloudResourceWithDetails {
	id: string;
	cloudAccountId: string;
	externalId: string;
	name: string;
	serverType: string;
	location: string;
	image: string;
	sshPubkey: string;
	status: string;
	publicIp?: string;
	sshPort: number;
	sshUsername: string;
	externalSshKeyId?: string;
	gatewaySlug?: string;
	gatewaySubdomain?: string;
	gatewaySshPort?: number;
	gatewayPortRangeStart?: number;
	gatewayPortRangeEnd?: number;
	offeringId?: number;
	listingMode: string;
	errorMessage?: string;
	platformFeeE9s: number;
	createdAt: string;
	updatedAt: string;
	terminatedAt?: string;
	cloudAccountName: string;
	cloudAccountBackend: string;
}

export interface CloudResourceListResponse {
	resources: CloudResourceWithDetails[];
}

export interface ProvisionResourceRequest {
	cloudAccountId: string;
	name: string;
	serverType: string;
	location: string;
	image: string;
	sshPubkey: string;
}

export async function listCloudAccounts(
	headers: SignedRequestHeaders
): Promise<CloudAccount[]> {
	const url = `${API_BASE_URL}/api/v1/cloud-accounts`;
	const response = await fetch(url, { headers });

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to fetch cloud accounts: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<CloudAccountListResponse>;
	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to fetch cloud accounts');
	}

	return payload.data?.accounts ?? [];
}

export async function addCloudAccount(
	request: AddCloudAccountRequest,
	headers: SignedRequestHeaders
): Promise<CloudAccount> {
	const url = `${API_BASE_URL}/api/v1/cloud-accounts`;
	const response = await fetch(url, {
		method: 'POST',
		headers: { ...headers, 'Content-Type': 'application/json' },
		body: JSON.stringify(request)
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to add cloud account: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<CloudAccount>;
	if (!payload.success || !payload.data) {
		throw new Error(payload.error ?? 'Failed to add cloud account');
	}

	return payload.data;
}

export async function deleteCloudAccount(
	accountId: string,
	headers: SignedRequestHeaders
): Promise<void> {
	const url = `${API_BASE_URL}/api/v1/cloud-accounts/${accountId}`;
	const response = await fetch(url, {
		method: 'DELETE',
		headers
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to delete cloud account: ${response.status}`);
		throw new Error(errorMsg);
	}
}

export async function getCloudAccountCatalog(
	accountId: string,
	headers: SignedRequestHeaders
): Promise<BackendCatalog> {
	const url = `${API_BASE_URL}/api/v1/cloud-accounts/${accountId}/catalog`;
	const response = await fetch(url, { headers });

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to fetch catalog: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<BackendCatalog>;
	if (!payload.success || !payload.data) {
		throw new Error(payload.error ?? 'Failed to fetch catalog');
	}

	return payload.data;
}

export async function listCloudResources(
	headers: SignedRequestHeaders
): Promise<CloudResourceWithDetails[]> {
	const url = `${API_BASE_URL}/api/v1/cloud-resources`;
	const response = await fetch(url, { headers });

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to fetch cloud resources: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<CloudResourceListResponse>;
	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to fetch cloud resources');
	}

	return payload.data?.resources ?? [];
}

export async function provisionCloudResource(
	request: ProvisionResourceRequest,
	headers: SignedRequestHeaders
): Promise<CloudResource> {
	const url = `${API_BASE_URL}/api/v1/cloud-resources`;
	const response = await fetch(url, {
		method: 'POST',
		headers: { ...headers, 'Content-Type': 'application/json' },
		body: JSON.stringify(request)
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to provision resource: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<CloudResource>;
	if (!payload.success || !payload.data) {
		throw new Error(payload.error ?? 'Failed to provision resource');
	}

	return payload.data;
}

export async function deleteCloudResource(
	resourceId: string,
	headers: SignedRequestHeaders
): Promise<void> {
	const url = `${API_BASE_URL}/api/v1/cloud-resources/${resourceId}`;
	const response = await fetch(url, {
		method: 'DELETE',
		headers
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to delete cloud resource: ${response.status}`);
		throw new Error(errorMsg);
	}
}

export async function startCloudResource(
	resourceId: string,
	headers: SignedRequestHeaders
): Promise<void> {
	const url = `${API_BASE_URL}/api/v1/cloud-resources/${resourceId}/start`;
	const response = await fetch(url, { method: 'POST', headers });

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to start resource: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<unknown>;
	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to start resource');
	}
}

export async function stopCloudResource(
	resourceId: string,
	headers: SignedRequestHeaders
): Promise<void> {
	const url = `${API_BASE_URL}/api/v1/cloud-resources/${resourceId}/stop`;
	const response = await fetch(url, { method: 'POST', headers });

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to stop resource: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<unknown>;
	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to stop resource');
	}
}

export async function validateCloudAccount(
	accountId: string,
	headers: SignedRequestHeaders
): Promise<CloudAccount> {
	const url = `${API_BASE_URL}/api/v1/cloud-accounts/${accountId}/validate`;
	const response = await fetch(url, { method: 'POST', headers });

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to validate account: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<CloudAccount>;
	if (!payload.success || !payload.data) {
		throw new Error(payload.error ?? 'Failed to validate account');
	}

	return payload.data;
}

export async function listOnMarketplace(
	resourceId: string,
	request: { offerName: string; monthlyPrice: number; description?: string },
	headers: SignedRequestHeaders
): Promise<unknown> {
	const url = `${API_BASE_URL}/api/v1/cloud-resources/${resourceId}/list-on-marketplace`;
	const response = await fetch(url, {
		method: 'POST',
		headers: { ...headers, 'Content-Type': 'application/json' },
		body: JSON.stringify(request)
	});

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to list on marketplace: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<unknown>;
	if (!payload.success || !payload.data) {
		throw new Error(payload.error ?? 'Failed to list on marketplace');
	}

	return payload.data;
}

export async function unlistFromMarketplace(
	resourceId: string,
	headers: SignedRequestHeaders
): Promise<void> {
	const url = `${API_BASE_URL}/api/v1/cloud-resources/${resourceId}/unlist-from-marketplace`;
	const response = await fetch(url, { method: 'POST', headers });

	if (!response.ok) {
		const errorMsg = await getErrorMessage(response, `Failed to unlist from marketplace: ${response.status}`);
		throw new Error(errorMsg);
	}

	const payload = (await response.json()) as ApiResponse<unknown>;
	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to unlist from marketplace');
	}
}
