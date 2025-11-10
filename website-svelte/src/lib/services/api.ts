const DEFAULT_API_BASE_URL = 'http://localhost:8080';

const API_BASE_URL =
	import.meta.env.VITE_DECENT_CLOUD_API_URL?.replace(/\/+$/, '') ?? DEFAULT_API_BASE_URL;

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
	current_block_validators: number;
	total_blocks: number;
	latest_block_timestamp_ns: number;
	blocks_until_next_halving: number;
	current_block_rewards_e9s: number;
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
