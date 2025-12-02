import type { Ed25519KeyIdentity } from '@dfinity/identity';
import { signRequest } from './auth-api';
import { API_BASE_URL } from './api';

/**
 * Account information from API
 */
export interface AccountWithKeys {
	id: string;
	username: string;
	createdAt: number; // Timestamp in nanoseconds
	updatedAt: number; // Timestamp in nanoseconds
	isAdmin: boolean; // Admin flag from backend
	emailVerified: boolean;
	email?: string;
	publicKeys: PublicKeyInfo[];
}

/**
 * Public key information
 */
export interface PublicKeyInfo {
	id: string;
	publicKey: string;
	addedAt: number; // Timestamp in nanoseconds
	isActive: boolean;
	deviceName?: string;
	disabledAt?: number; // Timestamp in nanoseconds
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
	username: string,
	email: string
): Promise<AccountWithKeys> {
	const publicKeyBytes = new Uint8Array(identity.getPublicKey().rawKey);
	const publicKeyHex = bytesToHex(publicKeyBytes);

	const requestBody = {
		username,
		publicKey: publicKeyHex,
		email
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
		const text = await response.text().catch(() => '');
		let errorMessage = `Registration failed (HTTP ${response.status} ${response.statusText})`;
		try {
			const errorData = JSON.parse(text);
			if (errorData.error) {
				errorMessage = `${errorData.error} (HTTP ${response.status})`;
			}
		} catch {
			if (text) {
				errorMessage = `Registration failed (HTTP ${response.status} ${response.statusText}: ${text.substring(0, 200)})`;
			}
		}
		throw new Error(errorMessage);
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
 * Search for account by public key
 * Returns account if key is registered, null if not found
 */
export async function getAccountByPublicKey(publicKey: string): Promise<AccountWithKeys | null> {
	const response = await fetch(`${API_BASE_URL}/api/v1/accounts?publicKey=${publicKey}`, {
		method: 'GET'
	});

	if (response.status === 404) {
		return null; // Key not found in any account
	}

	if (!response.ok) {
		throw new Error(`Failed to search account by pubkey: ${response.statusText}`);
	}

	const result: ApiResponse<AccountWithKeys> = await response.json();

	if (!result.success) {
		return null; // Not found
	}

	return result.data || null;
}

/**
 * Add a new public key to an account
 */
export async function addAccountKey(
	identity: Ed25519KeyIdentity,
	username: string,
	newPublicKeyHex: string
): Promise<PublicKeyInfo> {
	const requestBody = {
		newPublicKey: newPublicKeyHex
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
		const text = await response.text().catch(() => '');
		let errorMessage = `Failed to add key (HTTP ${response.status} ${response.statusText})`;
		try {
			const errorData = JSON.parse(text);
			if (errorData.error) {
				errorMessage = `${errorData.error} (HTTP ${response.status})`;
			}
		} catch {
			if (text) {
				errorMessage = `Failed to add key (HTTP ${response.status} ${response.statusText}: ${text.substring(0, 200)})`;
			}
		}
		throw new Error(errorMessage);
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
	const { headers, body } = await signRequest(
		identity,
		'DELETE',
		`/api/v1/accounts/${username}/keys/${keyId}`
	);

	const response = await fetch(`${API_BASE_URL}/api/v1/accounts/${username}/keys/${keyId}`, {
		method: 'DELETE',
		headers: headers as HeadersInit,
		body
	});

	if (!response.ok) {
		const text = await response.text().catch(() => '');
		let errorMessage = `Failed to remove key (HTTP ${response.status} ${response.statusText})`;
		try {
			const errorData = JSON.parse(text);
			if (errorData.error) {
				errorMessage = `${errorData.error} (HTTP ${response.status})`;
			}
		} catch {
			if (text) {
				errorMessage = `Failed to remove key (HTTP ${response.status} ${response.statusText}: ${text.substring(0, 200)})`;
			}
		}
		throw new Error(errorMessage);
	}

	const result: ApiResponse<PublicKeyInfo> = await response.json();

	if (!result.success) {
		throw new Error(result.error || 'Failed to remove key');
	}
}

/**
 * Update device name for a public key
 * Requires signing with an active key from the same account
 */
export async function updateDeviceName(
	identity: Ed25519KeyIdentity,
	username: string,
	keyId: string,
	deviceName: string
): Promise<PublicKeyInfo> {
	const requestBody = {
		deviceName
	};

	const { headers, body } = await signRequest(
		identity,
		'PUT',
		`/api/v1/accounts/${username}/keys/${keyId}`,
		requestBody
	);

	const response = await fetch(`${API_BASE_URL}/api/v1/accounts/${username}/keys/${keyId}`, {
		method: 'PUT',
		headers: headers as HeadersInit,
		body
	});

	if (!response.ok) {
		const text = await response.text().catch(() => '');
		let errorMessage = `Failed to update device name (HTTP ${response.status} ${response.statusText})`;
		try {
			const errorData = JSON.parse(text);
			if (errorData.error) {
				errorMessage = `${errorData.error} (HTTP ${response.status})`;
			}
		} catch {
			if (text) {
				errorMessage = `Failed to update device name (HTTP ${response.status} ${response.statusText}: ${text.substring(0, 200)})`;
			}
		}
		throw new Error(errorMessage);
	}

	const result: ApiResponse<PublicKeyInfo> = await response.json();

	if (!result.success || !result.data) {
		throw new Error(result.error || 'Failed to update device name');
	}

	return result.data;
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
	const trimmed = username.trim();

	// Length check
	if (trimmed.length < 3) {
		return 'Username too short (minimum 3 characters)';
	}
	if (trimmed.length > 64) {
		return 'Username too long (maximum 64 characters)';
	}

	// Check first character
	if (!/^[a-zA-Z0-9]/.test(trimmed)) {
		return 'Username must start with a letter or number';
	}

	// Check last character
	if (!/[a-zA-Z0-9]$/.test(trimmed)) {
		return 'Username must end with a letter or number';
	}

	// Check for invalid characters
	const invalidChars = trimmed.match(/[^a-zA-Z0-9._@-]/g);
	if (invalidChars) {
		const uniqueInvalid = [...new Set(invalidChars)].join(', ');
		return `Invalid character(s): ${uniqueInvalid}. Only letters, numbers, and ._@- allowed`;
	}

	// Reserved usernames (case-insensitive check for safety)
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

	if (reserved.includes(trimmed.toLowerCase())) {
		return 'This username is reserved';
	}

	return null;
}

/**
 * Generate username suggestions based on a taken username
 */
export function generateUsernameSuggestions(username: string): string[] {
	const trimmed = username.trim();
	const suggestions: string[] = [];

	// Add numbers
	for (let i = 1; i <= 3; i++) {
		suggestions.push(`${trimmed}${i}`);
	}

	// Add underscore + numbers
	suggestions.push(`${trimmed}_99`);
	suggestions.push(`${trimmed}_01`);

	// Add random numbers
	const random = Math.floor(Math.random() * 1000);
	suggestions.push(`${trimmed}${random}`);

	return suggestions;
}

/**
 * Request account recovery via email
 * Sends recovery link to the email associated with the account
 */
export async function requestRecovery(email: string): Promise<string> {
	const response = await fetch(`${API_BASE_URL}/api/v1/accounts/recovery/request`, {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json'
		},
		body: JSON.stringify({ email })
	});

	if (!response.ok) {
		const text = await response.text().catch(() => '');
		let errorMessage = `Recovery request failed (HTTP ${response.status} ${response.statusText})`;
		try {
			const errorData = JSON.parse(text);
			if (errorData.error) {
				errorMessage = `${errorData.error} (HTTP ${response.status})`;
			}
		} catch {
			if (text) {
				errorMessage = `Recovery request failed (HTTP ${response.status} ${response.statusText}: ${text.substring(0, 200)})`;
			}
		}
		throw new Error(errorMessage);
	}

	const result: ApiResponse<string> = await response.json();

	if (!result.success || !result.data) {
		throw new Error(result.error || 'Recovery request failed');
	}

	return result.data;
}

/**
 * Complete account recovery with token and new public key
 * Adds the new key to the account, allowing access with the new identity
 */
export async function completeRecovery(token: string, publicKeyHex: string): Promise<string> {
	const response = await fetch(`${API_BASE_URL}/api/v1/accounts/recovery/complete`, {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json'
		},
		body: JSON.stringify({ token, publicKey: publicKeyHex })
	});

	if (!response.ok) {
		const text = await response.text().catch(() => '');
		let errorMessage = `Recovery completion failed (HTTP ${response.status} ${response.statusText})`;
		try {
			const errorData = JSON.parse(text);
			if (errorData.error) {
				errorMessage = `${errorData.error} (HTTP ${response.status})`;
			}
		} catch {
			if (text) {
				errorMessage = `Recovery completion failed (HTTP ${response.status} ${response.statusText}: ${text.substring(0, 200)})`;
			}
		}
		throw new Error(errorMessage);
	}

	const result: ApiResponse<string> = await response.json();

	if (!result.success || !result.data) {
		throw new Error(result.error || 'Recovery completion failed');
	}

	return result.data;
}

/**
 * Verify email address with token from verification email
 * Sets email_verified=true for the account
 */
export async function verifyEmail(token: string): Promise<string> {
	const response = await fetch(`${API_BASE_URL}/api/v1/accounts/verify-email`, {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json'
		},
		body: JSON.stringify({ token })
	});

	if (!response.ok) {
		const text = await response.text().catch(() => '');
		let errorMessage = `Email verification failed (HTTP ${response.status} ${response.statusText})`;
		try {
			const errorData = JSON.parse(text);
			if (errorData.error) {
				errorMessage = `${errorData.error} (HTTP ${response.status})`;
			}
		} catch {
			if (text) {
				errorMessage = `Email verification failed (HTTP ${response.status} ${response.statusText}: ${text.substring(0, 200)})`;
			}
		}
		throw new Error(errorMessage);
	}

	const result: ApiResponse<string> = await response.json();

	if (!result.success || !result.data) {
		throw new Error(result.error || 'Email verification failed');
	}

	return result.data;
}

/**
 * Resend verification email to the account's registered email address
 * Requires authentication. Rate limited to 1 request per minute.
 */
export async function resendVerificationEmail(identity: Ed25519KeyIdentity): Promise<string> {
	const { headers, body } = await signRequest(
		identity,
		'POST',
		'/api/v1/accounts/resend-verification'
	);

	const response = await fetch(`${API_BASE_URL}/api/v1/accounts/resend-verification`, {
		method: 'POST',
		headers: headers as HeadersInit,
		body
	});

	if (!response.ok) {
		const text = await response.text().catch(() => '');
		let errorMessage = `Failed to resend verification email (HTTP ${response.status} ${response.statusText})`;
		try {
			const errorData = JSON.parse(text);
			if (errorData.error) {
				errorMessage = `${errorData.error} (HTTP ${response.status})`;
			}
		} catch {
			if (text) {
				errorMessage = `Failed to resend verification email (HTTP ${response.status} ${response.statusText}: ${text.substring(0, 200)})`;
			}
		}
		throw new Error(errorMessage);
	}

	const result: ApiResponse<string> = await response.json();

	if (!result.success || !result.data) {
		throw new Error(result.error || 'Failed to resend verification email');
	}

	return result.data;
}

/**
 * Update account email address
 * Requires authentication. Resets email verification status and sends verification email.
 */
export async function updateAccountEmail(
	identity: Ed25519KeyIdentity,
	username: string,
	email: string
): Promise<AccountWithKeys> {
	const requestBody = { email };

	const { headers, body } = await signRequest(
		identity,
		'PUT',
		`/api/v1/accounts/${username}/email`,
		requestBody
	);

	const response = await fetch(`${API_BASE_URL}/api/v1/accounts/${username}/email`, {
		method: 'PUT',
		headers: headers as HeadersInit,
		body
	});

	if (!response.ok) {
		const text = await response.text().catch(() => '');
		let errorMessage = `Failed to update email (HTTP ${response.status} ${response.statusText})`;
		try {
			const errorData = JSON.parse(text);
			if (errorData.error) {
				errorMessage = `${errorData.error} (HTTP ${response.status})`;
			}
		} catch {
			if (text) {
				errorMessage = `Failed to update email (HTTP ${response.status} ${response.statusText}: ${text.substring(0, 200)})`;
			}
		}
		throw new Error(errorMessage);
	}

	const result: ApiResponse<AccountWithKeys> = await response.json();

	if (!result.success || !result.data) {
		throw new Error(result.error || 'Failed to update email');
	}

	return result.data;
}

function bytesToHex(bytes: Uint8Array): string {
	return Array.from(bytes)
		.map((b) => b.toString(16).padStart(2, '0'))
		.join('');
}
