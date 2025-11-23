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
			const username = 'testuser';
			const profile = {
				displayName: 'Test User',
				bio: 'Test bio',
				avatarUrl: 'https://example.com/avatar.png'
			};

			await client.updateProfile(username, profile);

			expect(authApi.signRequest).toHaveBeenCalledWith(
				mockIdentity,
				'PUT',
				`/api/v1/accounts/${username}/profile`,
				profile
			);

			expect(fetch).toHaveBeenCalledWith(
				expect.stringContaining(`/api/v1/accounts/${username}/profile`),
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
			const username = 'testuser';
			const contact = {
				contact_type: 'email',
				contact_value: 'test@example.com',
				verified: false
			};

			await client.upsertContact(username, contact);

			expect(authApi.signRequest).toHaveBeenCalledWith(
				mockIdentity,
				'POST',
				`/api/v1/accounts/${username}/contacts`,
				contact
			);
		});
	});

	describe('deleteContact', () => {
		it('calls DELETE with contact ID', async () => {
			const username = 'testuser';
			const contactId = 123;

			await client.deleteContact(username, contactId);

			expect(authApi.signRequest).toHaveBeenCalledWith(
				mockIdentity,
				'DELETE',
				`/api/v1/accounts/${username}/contacts/${contactId}`,
				undefined
			);
		});
	});

	describe('upsertSocial', () => {
		it('calls POST with social data', async () => {
			const username = 'testuser';
			const social = {
				platform: 'twitter',
				username: 'testuser',
				profile_url: 'https://twitter.com/testuser'
			};

			await client.upsertSocial(username, social);

			expect(authApi.signRequest).toHaveBeenCalledWith(
				mockIdentity,
				'POST',
				`/api/v1/accounts/${username}/socials`,
				social
			);
		});
	});

	describe('deleteSocial', () => {
		it('calls DELETE with social ID', async () => {
			const username = 'testuser';
			const socialId = 456;

			await client.deleteSocial(username, socialId);

			expect(authApi.signRequest).toHaveBeenCalledWith(
				mockIdentity,
				'DELETE',
				`/api/v1/accounts/${username}/socials/${socialId}`,
				undefined
			);
		});
	});

	describe('addExternalKey', () => {
		it('calls POST with external key data', async () => {
			const username = 'testuser';
			const key = {
				key_type: 'ssh-ed25519',
				key_data: 'ssh-ed25519 AAAAC3...',
				key_fingerprint: 'SHA256:abc123',
				label: 'My laptop'
			};

			await client.addExternalKey(username, key);

			expect(authApi.signRequest).toHaveBeenCalledWith(
				mockIdentity,
				'POST',
				`/api/v1/accounts/${username}/external-keys`,
				key
			);
		});
	});

	describe('deleteExternalKey', () => {
		it('calls DELETE with key ID', async () => {
			const username = 'testuser';
			const keyId = 789;

			await client.deleteExternalKey(username, keyId);

			expect(authApi.signRequest).toHaveBeenCalledWith(
				mockIdentity,
				'DELETE',
				`/api/v1/accounts/${username}/external-keys/${keyId}`,
				undefined
			);
		});
	});
});
