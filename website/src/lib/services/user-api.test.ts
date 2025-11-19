import { describe, it, expect, vi, beforeEach } from 'vitest';
import { UserApiClient } from './user-api';
import { Ed25519KeyIdentity } from '@dfinity/identity';
import * as authApi from './auth-api';

// Mock the signRequest function
vi.mock('./auth-api', () => ({
	signRequest: vi.fn()
}));

// Mock fetch
globalThis.fetch = vi.fn() as typeof fetch;

describe('UserApiClient', () => {
	let client: UserApiClient;
	let mockIdentity: Ed25519KeyIdentity;

	beforeEach(() => {
		vi.clearAllMocks();

		// Create a test identity
		const seed = new Uint8Array(32).fill(1);
		mockIdentity = Ed25519KeyIdentity.fromSecretKey(seed);

		// Mock signRequest to return test headers
		vi.mocked(authApi.signRequest).mockResolvedValue({
			headers: {
				'X-Public-Key': 'test-pubkey-hex',
				'X-Signature': 'test-signature-hex',
				'X-Timestamp': '1234567890000000000',
				'X-Nonce': 'test-nonce-uuid',
				'Content-Type': 'application/json'
			},
			body: JSON.stringify({ test: 'data' })
		});

		// Mock successful fetch response
		vi.mocked(fetch).mockResolvedValue({
			ok: true,
			json: async () => ({ success: true, data: {} })
		} as Response);

		client = new UserApiClient(mockIdentity);
	});

	describe('updateProfile', () => {
		it('calls authenticated fetch with correct parameters', async () => {
			const pubkey = 'test-pubkey';
			const profile = {
				display_name: 'Test User',
				bio: 'Test bio',
				avatar_url: 'https://example.com/avatar.png'
			};

			await client.updateProfile(pubkey, profile);

			expect(authApi.signRequest).toHaveBeenCalledWith(
				mockIdentity,
				'PUT',
				`/api/v1/users/${pubkey}/profile`,
				profile
			);

			expect(fetch).toHaveBeenCalledWith(
				expect.stringContaining(`/api/v1/users/${pubkey}/profile`),
				expect.objectContaining({
					method: 'PUT',
					headers: expect.objectContaining({
						'X-Public-Key': 'test-pubkey-hex',
						'X-Signature': 'test-signature-hex'
					})
				})
			);
		});
	});

	describe('upsertContact', () => {
		it('calls POST with contact data', async () => {
			const pubkey = 'test-pubkey';
			const contact = {
				contact_type: 'email',
				contact_value: 'test@example.com',
				verified: false
			};

			await client.upsertContact(pubkey, contact);

			expect(authApi.signRequest).toHaveBeenCalledWith(
				mockIdentity,
				'POST',
				`/api/v1/users/${pubkey}/contacts`,
				contact
			);
		});
	});

	describe('deleteContact', () => {
		it('calls DELETE with contact type', async () => {
			const pubkey = 'test-pubkey';
			const contactId = 123;

			await client.deleteContact(pubkey, contactId);

			expect(authApi.signRequest).toHaveBeenCalledWith(
				mockIdentity,
				'DELETE',
				`/api/v1/users/${pubkey}/contacts/${contactId}`,
				undefined
			);
		});
	});

	describe('upsertSocial', () => {
		it('calls POST with social data', async () => {
			const pubkey = 'test-pubkey';
			const social = {
				platform: 'twitter',
				username: 'testuser',
				profile_url: 'https://twitter.com/testuser'
			};

			await client.upsertSocial(pubkey, social);

			expect(authApi.signRequest).toHaveBeenCalledWith(
				mockIdentity,
				'POST',
				`/api/v1/users/${pubkey}/socials`,
				social
			);
		});
	});

	describe('deleteSocial', () => {
		it('calls DELETE with platform', async () => {
			const pubkey = 'test-pubkey';
			const socialId = 456;

			await client.deleteSocial(pubkey, socialId);

			expect(authApi.signRequest).toHaveBeenCalledWith(
				mockIdentity,
				'DELETE',
				`/api/v1/users/${pubkey}/socials/${socialId}`,
				undefined
			);
		});
	});

	describe('addPublicKey', () => {
		it('calls POST with key data', async () => {
			const pubkey = 'test-pubkey';
			const key = {
				key_type: 'ssh-ed25519',
				key_data: 'ssh-ed25519 AAAAC3...',
				key_fingerprint: 'SHA256:abc123',
				label: 'My laptop'
			};

			await client.addPublicKey(pubkey, key);

			expect(authApi.signRequest).toHaveBeenCalledWith(
				mockIdentity,
				'POST',
				`/api/v1/users/${pubkey}/keys`,
				key
			);
		});
	});

	describe('deletePublicKey', () => {
		it('calls DELETE with fingerprint', async () => {
			const pubkey = 'test-pubkey';
			const keyId = 789;

			await client.deletePublicKey(pubkey, keyId);

			expect(authApi.signRequest).toHaveBeenCalledWith(
				mockIdentity,
				'DELETE',
				`/api/v1/users/${pubkey}/keys/${keyId}`,
				undefined
			);
		});
	});
});
