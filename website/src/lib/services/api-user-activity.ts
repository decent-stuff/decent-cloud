import type { Contract, Offering, ApiResponse } from './api';
import { API_BASE_URL } from './api';
import { normalizePubkey } from '$lib/utils/identity';
import type { PublicUserActivity } from '$lib/types/generated/PublicUserActivity';
import type { PublicContractSummary } from '$lib/types/generated/PublicContractSummary';

// ============ User Activity Endpoints ============

export interface UserActivity {
	offerings_provided: Offering[];
	rentals_as_requester: Contract[];
	rentals_as_provider: Contract[];
}

export type { PublicUserActivity, PublicContractSummary };

/**
 * Normalize pubkey fields and add `contractId` aliases on a raw activity
 * payload (the shape returned verbatim by the API). Shared by the standalone
 * /users/:pubkey/activity call and the combined /provider/dashboard call so the
 * normalization lives in exactly one place.
 */
export function normalizeUserActivity(data: UserActivity): UserActivity {
	return {
		offerings_provided: data.offerings_provided.map((o: any) => ({
			...o,
			pubkey: normalizePubkey(o.pubkey)
		})),
		rentals_as_requester: data.rentals_as_requester.map((c: any) => ({
			...c,
			contractId: c.contract_id,
			requester_pubkey: normalizePubkey(c.requester_pubkey),
			provider_pubkey: normalizePubkey(c.provider_pubkey)
		})),
		rentals_as_provider: data.rentals_as_provider.map((c: any) => ({
			...c,
			contractId: c.contract_id,
			requester_pubkey: normalizePubkey(c.requester_pubkey),
			provider_pubkey: normalizePubkey(c.provider_pubkey)
		}))
	};
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
	return normalizeUserActivity(payload.data);
}

/**
 * Normalize pubkey + add contractId alias on a PUBLIC activity payload (the
 * shape returned by /users/:pubkey/public-profile). Mirrors normalizeUserActivity
 * but for the reduced PublicContractSummary contracts.
 */
export function normalizePublicUserActivity(data: PublicUserActivity): PublicUserActivity {
	const normalizeSummary = (c: PublicContractSummary): PublicContractSummary => ({
		...c,
		provider_pubkey: normalizePubkey(c.provider_pubkey),
		requester_pubkey: normalizePubkey(c.requester_pubkey)
	});
	return {
		offerings_provided: data.offerings_provided.map((o: any) => ({
			...o,
			pubkey: normalizePubkey(o.pubkey)
		})),
		rentals_as_requester: data.rentals_as_requester.map(normalizeSummary),
		rentals_as_provider: data.rentals_as_provider.map(normalizeSummary)
	};
}

/**
 * Fetch the PUBLIC (non-sensitive) activity profile for any user. No auth
 * required — used by the reputation and user-profile pages that view OTHER
 * users. Payment amounts, SSH keys, and gateway info are not included.
 */
export async function getPublicUserActivity(pubkeyHex: string): Promise<PublicUserActivity | null> {
	const url = `${API_BASE_URL}/api/v1/users/${pubkeyHex}/public-profile`;

	const response = await fetch(url);

	if (!response.ok) {
		if (response.status === 404) {
			return null;
		}
		throw new Error(`Failed to fetch public user profile: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<PublicUserActivity>;

	if (!payload.success || !payload.data) {
		return null;
	}

	return normalizePublicUserActivity(payload.data);
}
