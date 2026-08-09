import { API_BASE_URL, hexEncode, type ApiResponse } from './api';

// ============ Reputation & Account Info Types ============

export interface ReputationInfo {
	pubkey: string;
	total_reputation: number;
	change_count: number;
}

export interface AccountRegistration {
	pubkey: string;
	created_at_ns: number;
	account_type: 'user' | 'provider' | 'both';
}

export interface AccountSearchResult {
	username: string;
	display_name?: string;
	pubkey: string;
	reputation_score: number;
	contract_count: number;
	offering_count: number;
}

export interface ReputationLeaderboardEntry {
	pubkey: string;
	username?: string;
	display_name?: string;
	provider_name: string;
	trust_score?: number;
	completed_contracts: number;
	total_contracts: number;
	completion_rate_pct: number;
	volume_e9s: number;
}

// ============ Helper Functions ============

function normalizePubkey(pubkey: string | number[]): string {
	if (typeof pubkey === 'string') {
		return pubkey;
	}
	return hexEncode(new Uint8Array(pubkey));
}

// ============ API Functions ============

/**
 * Get reputation information for an account
 */
export async function getReputation(pubkeyHex: string): Promise<ReputationInfo | null> {
	const url = `${API_BASE_URL}/api/v1/reputation/${pubkeyHex}`;

	const response = await fetch(url);

	if (!response.ok) {
		if (response.status === 404) {
			return null;
		}
		throw new Error(`Failed to fetch reputation: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<ReputationInfo>;

	if (!payload.success || !payload.data) {
		return null;
	}

	return {
		...payload.data,
		pubkey: normalizePubkey(payload.data.pubkey)
	};
}

/**
 * Search accounts by username, display name, or public key
 */
export async function searchReputation(query: string, limit: number = 50): Promise<AccountSearchResult[]> {
	if (!query || query.trim().length === 0) {
		return [];
	}

	const url = `${API_BASE_URL}/api/v1/reputation/search?q=${encodeURIComponent(query)}&limit=${limit}`;

	const response = await fetch(url);

	if (!response.ok) {
		throw new Error(`Failed to search accounts: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<AccountSearchResult[]>;

	if (!payload.success || !payload.data) {
		return [];
	}

	return payload.data;
}

/**
 * Get the reputation leaderboard: top providers by trust score and completed
 * contracts. The backend honesty gate excludes providers with no contracts.
 */
export async function getReputationLeaderboard(
	limit: number = 20
): Promise<ReputationLeaderboardEntry[]> {
	const url = `${API_BASE_URL}/api/v1/reputation/leaderboard?limit=${limit}`;

	const response = await fetch(url);

	if (!response.ok) {
		throw new Error(`Failed to fetch leaderboard: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<ReputationLeaderboardEntry[]>;

	if (!payload.success || !payload.data) {
		return [];
	}

	return payload.data;
}
