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
	constructor(private signingIdentity: Ed25519KeyIdentity) {}

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
		pubkey: string,
		profile: {
			display_name?: string;
			bio?: string;
			avatar_url?: string;
		}
	) {
		const path = `/api/v1/users/${pubkey}/profile`;
		return this.authenticatedFetch('PUT', path, profile);
	}

	// Contacts
	async upsertContact(
		pubkey: string,
		contact: {
			contact_type: string;
			contact_value: string;
			verified?: boolean;
		}
	) {
		const path = `/api/v1/users/${pubkey}/contacts`;
		return this.authenticatedFetch('POST', path, contact);
	}

	async deleteContact(pubkey: string, contactId: number) {
		const path = `/api/v1/users/${pubkey}/contacts/${contactId}`;
		return this.authenticatedFetch('DELETE', path);
	}

	// Socials
	async upsertSocial(
		pubkey: string,
		social: {
			platform: string;
			username: string;
			profile_url?: string;
		}
	) {
		const path = `/api/v1/users/${pubkey}/socials`;
		return this.authenticatedFetch('POST', path, social);
	}

	async deleteSocial(pubkey: string, socialId: number) {
		const path = `/api/v1/users/${pubkey}/socials/${socialId}`;
		return this.authenticatedFetch('DELETE', path);
	}

	// Public Keys
	async addPublicKey(
		pubkey: string,
		key: {
			key_type: string;
			key_data: string;
			key_fingerprint?: string;
			label?: string;
		}
	) {
		const path = `/api/v1/users/${pubkey}/keys`;
		return this.authenticatedFetch('POST', path, key);
	}

	async deletePublicKey(pubkey: string, keyId: number) {
		const path = `/api/v1/users/${pubkey}/keys/${keyId}`;
		return this.authenticatedFetch('DELETE', path);
	}
}
