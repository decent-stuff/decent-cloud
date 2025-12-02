import { API_BASE_URL, type ApiResponse } from './api';
import { signRequest } from './auth-api';
import type { Ed25519KeyIdentity } from '@dfinity/identity';

export interface ChatwootIdentity {
	identifier: string;
	identifierHash: string;
}

/**
 * Get Chatwoot identity hash for authenticated user.
 * Used to authenticate users in the Chatwoot widget.
 */
export async function getChatwootIdentity(
	identity: Ed25519KeyIdentity
): Promise<ChatwootIdentity | null> {
	const path = '/api/v1/chatwoot/identity';
	const { headers } = await signRequest(identity, 'GET', path);

	const response = await fetch(`${API_BASE_URL}${path}`, {
		method: 'GET',
		headers: headers as unknown as HeadersInit
	});

	if (!response.ok) {
		console.error('Failed to get Chatwoot identity:', response.status);
		return null;
	}

	const payload = (await response.json()) as ApiResponse<ChatwootIdentity>;

	if (!payload.success || !payload.data) {
		console.error('Chatwoot identity API error:', payload.error);
		return null;
	}

	return payload.data;
}
