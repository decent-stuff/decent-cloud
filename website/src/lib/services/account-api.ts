import type { Ed25519KeyIdentity } from '@dfinity/identity';
import { signRequest } from './auth-api';
import { API_BASE_URL } from './api';

/**
 * Account information from API
 */
export interface AccountWithKeys {
	id: string;
	username: string;
	createdAt: number;
	updatedAt: number;
	publicKeys: PublicKeyInfo[];
}

/**
 * Public key information
 */
export interface PublicKeyInfo {
	id: string;
	publicKey: string;
	addedAt: number;
	isActive: boolean;
	disabledAt?: number;
	disabledByKeyId?: string;
}

/**
 * API response wrapper
 */
interface ApiResponse<T> {
	success: boolean;
	data?: T;
	error?: string;
}

/**
 * Register a new account with username and initial public key
 */
export async function registerAccount(
	identity: Ed25519KeyIdentity,
	username: string
): Promise<AccountWithKeys> {
	const publicKeyBytes = new Uint8Array(identity.getPublicKey().rawKey);
	const publicKeyHex = bytesToHex(publicKeyBytes);

	const requestBody = {
		username,
		publicKey: publicKeyHex
	};

	const { headers, body } = await signRequest(
		identity,
		'POST',
		'/api/v1/accounts',
		requestBody
	);

	const response = await fetch(`${API_BASE_URL}/api/v1/accounts`, {
		method: 'POST',
		headers: headers as HeadersInit,
		body
	});

	if (!response.ok) {
		const errorData = await response.json().catch(() => ({ error: 'Unknown error' }));
		throw new Error(errorData.error || `Registration failed: ${response.statusText}`);
	}

	const result: ApiResponse<AccountWithKeys> = await response.json();

	if (!result.success || !result.data) {
		throw new Error(result.error || 'Registration failed');
	}

	return result.data;
}

/**
 * Get account by username
 */
export async function getAccount(username: string): Promise<AccountWithKeys | null> {
	const response = await fetch(`${API_BASE_URL}/api/v1/accounts/${username}`, {
		method: 'GET'
	});

	if (response.status === 404) {
		return null;
	}

	if (!response.ok) {
		throw new Error(`Failed to fetch account: ${response.statusText}`);
	}

	const result: ApiResponse<AccountWithKeys> = await response.json();

	// "Account not found" is a valid response, not an error
	if (!result.success && result.error === 'Account not found') {
		return null;
	}

	if (!result.success) {
		throw new Error(result.error || 'Failed to fetch account');
	}

	return result.data || null;
}

/**
 * Check if an account exists with the given public key
 * Returns the account if found, null otherwise
 */
export async function getAccountByPublicKey(publicKey: string): Promise<AccountWithKeys | null> {
	// We need to search through accounts or use a dedicated endpoint
	// For now, we'll need to add this endpoint to the API
	// Temporary: return null (will implement when API endpoint is added)

	// TODO: Implement GET /api/v1/accounts?publicKey={hex}
	// For now, users need to know their username to sign in
	return null;
}

/**
 * Add a new public key to an account
 */
export async function addAccountKey(
	identity: Ed25519KeyIdentity,
	username: string,
	newPublicKeyHex: string
): Promise<PublicKeyInfo> {
	const publicKeyBytes = new Uint8Array(identity.getPublicKey().rawKey);
	const signingPublicKeyHex = bytesToHex(publicKeyBytes);

	const requestBody = {
		newPublicKey: newPublicKeyHex,
		signingPublicKey: signingPublicKeyHex
	};

	const { headers, body } = await signRequest(
		identity,
		'POST',
		`/api/v1/accounts/${username}/keys`,
		requestBody
	);

	const response = await fetch(`${API_BASE_URL}/api/v1/accounts/${username}/keys`, {
		method: 'POST',
		headers: headers as HeadersInit,
		body
	});

	if (!response.ok) {
		const errorData = await response.json().catch(() => ({ error: 'Unknown error' }));
		throw new Error(errorData.error || `Failed to add key: ${response.statusText}`);
	}

	const result: ApiResponse<PublicKeyInfo> = await response.json();

	if (!result.success || !result.data) {
		throw new Error(result.error || 'Failed to add key');
	}

	return result.data;
}

/**
 * Remove a public key from an account
 */
export async function removeAccountKey(
	identity: Ed25519KeyIdentity,
	username: string,
	keyId: string
): Promise<void> {
	const publicKeyBytes = new Uint8Array(identity.getPublicKey().rawKey);
	const signingPublicKeyHex = bytesToHex(publicKeyBytes);

	const requestBody = {
		signingPublicKey: signingPublicKeyHex
	};

	const { headers, body } = await signRequest(
		identity,
		'DELETE',
		`/api/v1/accounts/${username}/keys/${keyId}`,
		requestBody
	);

	const response = await fetch(`${API_BASE_URL}/api/v1/accounts/${username}/keys/${keyId}`, {
		method: 'DELETE',
		headers: headers as HeadersInit,
		body
	});

	if (!response.ok) {
		const errorData = await response.json().catch(() => ({ error: 'Unknown error' }));
		throw new Error(errorData.error || `Failed to remove key: ${response.statusText}`);
	}

	const result: ApiResponse<PublicKeyInfo> = await response.json();

	if (!result.success) {
		throw new Error(result.error || 'Failed to remove key');
	}
}

/**
 * Check if a username is available
 * Returns true if available, false if taken
 * Throws error if there's a problem checking (FAIL FAST)
 */
export async function checkUsernameAvailable(username: string): Promise<boolean> {
	const account = await getAccount(username);
	return account === null;
}

/**
 * Validate username format (client-side)
 * Returns error message or null if valid
 */
export function validateUsernameFormat(username: string): string | null {
	// Normalize
	const normalized = username.trim().toLowerCase();

	// Length check
	if (normalized.length < 3) {
		return 'Username must be at least 3 characters';
	}
	if (normalized.length > 64) {
		return 'Username must be at most 64 characters';
	}

	// Format check: [a-z0-9][a-z0-9._@-]*[a-z0-9]
	const regex = /^[a-z0-9][a-z0-9._@-]*[a-z0-9]$/;
	if (!regex.test(normalized)) {
		return 'Username must start and end with a letter or number, and contain only lowercase letters, numbers, dots, underscores, hyphens, or @';
	}

	// Reserved usernames
	const reserved = [
		'admin',
		'api',
		'system',
		'root',
		'support',
		'moderator',
		'administrator',
		'test',
		'null',
		'undefined',
		'decent',
		'cloud'
	];

	if (reserved.includes(normalized)) {
		return 'This username is reserved';
	}

	return null;
}

/**
 * Generate username suggestions based on a taken username
 */
export function generateUsernameSuggestions(username: string): string[] {
	const normalized = username.trim().toLowerCase();
	const suggestions: string[] = [];

	// Add numbers
	for (let i = 1; i <= 3; i++) {
		suggestions.push(`${normalized}${i}`);
	}

	// Add underscore + numbers
	suggestions.push(`${normalized}_99`);
	suggestions.push(`${normalized}_01`);

	// Add random numbers
	const random = Math.floor(Math.random() * 1000);
	suggestions.push(`${normalized}${random}`);

	return suggestions;
}

function bytesToHex(bytes: Uint8Array): string {
	return Array.from(bytes)
		.map((b) => b.toString(16).padStart(2, '0'))
		.join('');
}
