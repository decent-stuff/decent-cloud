import { API_BASE_URL, hexEncode, type ApiResponse } from './api';

// ============ Reputation & Account Info Types ============

export interface ReputationInfo {
	pubkey: string;
	total_reputation: number;
	change_count: number;
}

export interface AccountBalance {
	balance: number; // in e9s (smallest unit)
}

export interface TokenTransfer {
	from_account: string;
	to_account: string;
	amount_e9s: number;
	fee_e9s: number;
	memo?: string;
	created_at_ns: number;
}

export interface AccountRegistration {
	pubkey: string;
	created_at_ns: number;
	account_type: 'user' | 'provider' | 'both';
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
 * Get account balance
 */
export async function getAccountBalance(account: string): Promise<number> {
	const url = `${API_BASE_URL}/api/v1/accounts/${account}/balance`;

	const response = await fetch(url);

	if (!response.ok) {
		if (response.status === 404) {
			return 0;
		}
		throw new Error(`Failed to fetch balance: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<number>;

	if (!payload.success) {
		return 0;
	}

	return payload.data ?? 0;
}

/**
 * Get token transfers for an account
 */
export async function getAccountTransfers(
	account: string,
	limit: number = 50
): Promise<TokenTransfer[]> {
	const url = `${API_BASE_URL}/api/v1/accounts/${account}/transfers?limit=${limit}`;

	const response = await fetch(url);

	if (!response.ok) {
		if (response.status === 404) {
			return [];
		}
		throw new Error(`Failed to fetch transfers: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<TokenTransfer[]>;

	if (!payload.success || !payload.data) {
		return [];
	}

	return payload.data;
}

/**
 * Get recent transfers across the platform
 */
export async function getRecentTransfers(limit: number = 50): Promise<TokenTransfer[]> {
	const url = `${API_BASE_URL}/api/v1/transfers/recent?limit=${limit}`;

	const response = await fetch(url);

	if (!response.ok) {
		throw new Error(`Failed to fetch recent transfers: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<TokenTransfer[]>;

	if (!payload.success || !payload.data) {
		return [];
	}

	return payload.data;
}
