import { signRequest } from './auth-api';
import type { Ed25519KeyIdentity } from '@dfinity/identity';

const API_BASE =
	typeof window !== 'undefined' && import.meta.env.VITE_DECENT_CLOUD_API_URL
		? import.meta.env.VITE_DECENT_CLOUD_API_URL
		: 'https://api.decent-cloud.org';

export class UserApiClient {
	constructor(private signingIdentity: Ed25519KeyIdentity) {}

	private async authenticatedFetch(method: string, path: string, body?: unknown): Promise<Response> {
		const { headers, body: signedBody } = await signRequest(
			this.signingIdentity,
			method,
			path,
			body
		);

		return fetch(`${API_BASE}${path}`, {
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

	async deleteContact(pubkey: string, contactType: string) {
		const path = `/api/v1/users/${pubkey}/contacts/${contactType}`;
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

	async deleteSocial(pubkey: string, platform: string) {
		const path = `/api/v1/users/${pubkey}/socials/${platform}`;
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

	async deletePublicKey(pubkey: string, fingerprint: string) {
		const path = `/api/v1/users/${pubkey}/keys/${fingerprint}`;
		return this.authenticatedFetch('DELETE', path);
	}
}
