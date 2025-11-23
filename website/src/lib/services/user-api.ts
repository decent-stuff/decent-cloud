import { signRequest } from './auth-api';
import { API_BASE_URL } from './api';
import type { Ed25519KeyIdentity } from '@dfinity/identity';

/**
 * Helper function to handle API response errors consistently
 */
export async function handleApiResponse(res: Response): Promise<void> {
	if (!res.ok) {
		let errorMsg = `HTTP ${res.status}: ${res.statusText}`;
		try {
			const contentType = res.headers.get('content-type');
			if (contentType?.includes('application/json')) {
				const data = await res.json();
				errorMsg = data.error || errorMsg;
			} else {
				const text = await res.text();
				errorMsg = text || errorMsg;
			}
		} catch {
			// If parsing fails, use the default error message
		}
		throw new Error(errorMsg);
	}
}

export class UserApiClient {
	constructor(private signingIdentity: Ed25519KeyIdentity) { }

	private async authenticatedFetch(method: string, path: string, body?: unknown): Promise<Response> {
		const { headers, body: signedBody } = await signRequest(
			this.signingIdentity,
			method,
			path,
			body
		);

		return fetch(`${API_BASE_URL}${path}`, {
			method,
			headers,
			body: signedBody || undefined
		});
	}

	// Profile
	async updateProfile(
		username: string,
		profile: {
			displayName?: string;
			bio?: string;
			avatarUrl?: string;
		}
	) {
		const path = `/api/v1/accounts/${username}/profile`;
		return this.authenticatedFetch('PUT', path, profile);
	}

	// Contacts
	async upsertContact(
		username: string,
		contact: {
			contact_type: string;
			contact_value: string;
			verified?: boolean;
		}
	) {
		const path = `/api/v1/accounts/${username}/contacts`;
		return this.authenticatedFetch('POST', path, contact);
	}

	async deleteContact(username: string, contactId: number) {
		const path = `/api/v1/accounts/${username}/contacts/${contactId}`;
		return this.authenticatedFetch('DELETE', path);
	}

	// Socials
	async upsertSocial(
		username: string,
		social: {
			platform: string;
			username: string;
			profile_url?: string;
		}
	) {
		const path = `/api/v1/accounts/${username}/socials`;
		return this.authenticatedFetch('POST', path, social);
	}

	async deleteSocial(username: string, socialId: number) {
		const path = `/api/v1/accounts/${username}/socials/${socialId}`;
		return this.authenticatedFetch('DELETE', path);
	}

	// External Keys (SSH/GPG)
	async addExternalKey(
		username: string,
		key: {
			key_type: string;
			key_data: string;
			key_fingerprint?: string;
			label?: string;
		}
	) {
		const path = `/api/v1/accounts/${username}/external-keys`;
		return this.authenticatedFetch('POST', path, key);
	}

	async deleteExternalKey(username: string, keyId: number) {
		const path = `/api/v1/accounts/${username}/external-keys/${keyId}`;
		return this.authenticatedFetch('DELETE', path);
	}
}
