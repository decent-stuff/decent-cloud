import { API_BASE_URL, type ApiResponse } from './api';
import { signRequest } from './auth-api';
import type { Ed25519KeyIdentity } from '@dfinity/identity';

export interface ChatwootIdentity {
	identifier: string;
	identifierHash: string;
}

export interface SupportPortalStatus {
	hasAccount: boolean;
	userId: number | null;
	email: string | null;
	loginUrl: string;
	/** Help Center portal slug for this provider (if set up) */
	portalSlug: string | null;
	/** Provider's inbox ID for filtering conversations */
	inboxId: number | null;
}

export interface PasswordResetResponse {
	password: string;
	loginUrl: string;
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

/**
 * Get support portal account status for authenticated user.
 * Throws an error with the API error message on failure.
 */
export async function getSupportPortalStatus(
	identity: Ed25519KeyIdentity
): Promise<SupportPortalStatus> {
	const path = '/api/v1/chatwoot/support-access';
	const { headers } = await signRequest(identity, 'GET', path);

	const response = await fetch(`${API_BASE_URL}${path}`, {
		method: 'GET',
		headers: headers as unknown as HeadersInit
	});

	const payload = (await response.json()) as ApiResponse<SupportPortalStatus>;

	if (!payload.success || !payload.data) {
		throw new Error(payload.error || 'Failed to get support portal status');
	}

	return payload.data;
}

/**
 * Create support portal account. Returns the initial password directly.
 * Throws an error with the API error message on failure.
 */
export async function createSupportPortalAccount(
	identity: Ed25519KeyIdentity
): Promise<PasswordResetResponse> {
	const path = '/api/v1/chatwoot/support-access';
	const { headers } = await signRequest(identity, 'POST', path);

	const response = await fetch(`${API_BASE_URL}${path}`, {
		method: 'POST',
		headers: headers as unknown as HeadersInit
	});

	const payload = (await response.json()) as ApiResponse<PasswordResetResponse>;

	if (!payload.success || !payload.data) {
		throw new Error(payload.error || 'Failed to create account');
	}

	return payload.data;
}

/**
 * Reset support portal password. Returns the new password directly.
 * Throws an error with the API error message on failure.
 */
export async function resetSupportPortalPassword(
	identity: Ed25519KeyIdentity
): Promise<PasswordResetResponse> {
	const path = '/api/v1/chatwoot/support-access/reset';
	const { headers } = await signRequest(identity, 'POST', path);

	const response = await fetch(`${API_BASE_URL}${path}`, {
		method: 'POST',
		headers: headers as unknown as HeadersInit
	});

	const payload = (await response.json()) as ApiResponse<PasswordResetResponse>;

	if (!payload.success || !payload.data) {
		throw new Error(payload.error || 'Failed to reset password');
	}

	return payload.data;
}
