import type { Contract, Offering, ApiResponse } from './api';
import { API_BASE_URL } from './api';
import { normalizePubkey } from '$lib/utils/identity';

// ============ User Activity Endpoints ============

export interface UserActivity {
	offerings_provided: Offering[];
	rentals_as_requester: Contract[];
	rentals_as_provider: Contract[];
}

export async function getUserActivity(
	pubkeyHex: string,
	headers?: Record<string, string>
): Promise<UserActivity | null> {
	const url = `${API_BASE_URL}/api/v1/users/${pubkeyHex}/activity`;

	const response = await fetch(url, headers ? { headers } : undefined);

	if (!response.ok) {
		if (response.status === 401 || response.status === 403) {
			return null;
		}
		throw new Error(`Failed to fetch user activity: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<UserActivity>;

	if (!payload.success) {
		return null;
	}

	if (!payload.data) {
		return null;
	}

	// Normalize pubkey fields in all nested objects
	return {
		offerings_provided: payload.data.offerings_provided.map((o: any) => ({
			...o,
			pubkey: normalizePubkey(o.pubkey)
		})),
		rentals_as_requester: payload.data.rentals_as_requester.map((c: any) => ({
			...c,
			contractId: c.contract_id,
			requester_pubkey: normalizePubkey(c.requester_pubkey),
			provider_pubkey: normalizePubkey(c.provider_pubkey)
		})),
		rentals_as_provider: payload.data.rentals_as_provider.map((c: any) => ({
			...c,
			contractId: c.contract_id,
			requester_pubkey: normalizePubkey(c.requester_pubkey),
			provider_pubkey: normalizePubkey(c.provider_pubkey)
		}))
	};
}
