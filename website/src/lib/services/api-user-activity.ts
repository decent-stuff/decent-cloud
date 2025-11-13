import type { Contract, Offering, ApiResponse, API_BASE_URL as BASE_URL } from './api';
import { API_BASE_URL, hexEncode } from './api';

// ============ User Activity Endpoints ============

export interface UserActivity {
	offerings_provided: Offering[];
	rentals_as_requester: Contract[];
	rentals_as_provider: Contract[];
}

function normalizePubkeyHash(pubkeyHash: string | number[]): string {
	if (typeof pubkeyHash === 'string') {
		return pubkeyHash;
	}
	return hexEncode(new Uint8Array(pubkeyHash));
}

export async function getUserActivity(pubkeyHex: string): Promise<UserActivity> {
	const url = `${API_BASE_URL}/api/v1/users/${pubkeyHex}/activity`;

	const response = await fetch(url);

	if (!response.ok) {
		throw new Error(`Failed to fetch user activity: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<UserActivity>;

	if (!payload.success) {
		throw new Error(payload.error ?? 'Failed to fetch user activity');
	}

	if (!payload.data) {
		throw new Error('User activity response did not include data');
	}

	// Normalize pubkey_hash fields in all nested objects
	return {
		offerings_provided: payload.data.offerings_provided.map((o: any) => ({
			...o,
			pubkey_hash: normalizePubkeyHash(o.pubkey_hash)
		})),
		rentals_as_requester: payload.data.rentals_as_requester.map((c: any) => ({
			...c,
			contract_id: normalizePubkeyHash(c.contract_id),
			requester_pubkey_hash: normalizePubkeyHash(c.requester_pubkey_hash),
			provider_pubkey_hash: normalizePubkeyHash(c.provider_pubkey_hash)
		})),
		rentals_as_provider: payload.data.rentals_as_provider.map((c: any) => ({
			...c,
			contract_id: normalizePubkeyHash(c.contract_id),
			requester_pubkey_hash: normalizePubkeyHash(c.requester_pubkey_hash),
			provider_pubkey_hash: normalizePubkeyHash(c.provider_pubkey_hash)
		}))
	};
}
