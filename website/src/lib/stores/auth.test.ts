import { describe, it, expect, beforeEach, vi } from 'vitest';
import { get } from 'svelte/store';
import { authStore } from './auth';
import type { IdentityInfo } from './auth';

vi.mock('@dfinity/auth-client', () => ({
	AuthClient: {
		create: vi.fn().mockResolvedValue({
			isAuthenticated: vi.fn().mockResolvedValue(false),
			login: vi.fn(),
			logout: vi.fn()
		})
	}
}));

vi.stubGlobal('fetch', vi.fn());

describe('authStore', () => {
	beforeEach(async () => {
		vi.clearAllMocks();
		await authStore.logout();
	});

	it('should initialize with logged out state', () => {
		let isAuth = false;
		authStore.isAuthenticated.subscribe((value) => (isAuth = value))();

		expect(isAuth).toBe(false);
	});

	it('should throw error on login with invalid seed phrase', async () => {
		await expect(authStore.loginWithSeedPhrase('invalid seed', '/test')).rejects.toThrow();
	});

	it('should return null signing identity when not logged in', async () => {
		const signingIdentity = await authStore.getSigningIdentity();

		expect(signingIdentity).toBeNull();
	});

	it('should return null authenticated identity when not logged in', async () => {
		const authenticatedIdentity = await authStore.getAuthenticatedIdentity();

		expect(authenticatedIdentity).toBeNull();
	});

	it('should not update display name when no current identity', async () => {
		await authStore.updateDisplayName();

		let currentIdentity: IdentityInfo | null = null;
		authStore.currentIdentity.subscribe((value) => (currentIdentity = value))();

		expect(currentIdentity).toBeNull();
	});

	it('should not update display name when identity has no publicKeyBytes', async () => {
		const mockFetch = vi.fn();
		vi.stubGlobal('fetch', mockFetch);

		await authStore.updateDisplayName();

		expect(mockFetch).not.toHaveBeenCalled();
	});

	it('should fetch and update display name when profile exists', async () => {
		const mockDisplayName = 'Test User';
		const mockFetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({
				success: true,
				data: { displayName: mockDisplayName }
			})
		});
		vi.stubGlobal('fetch', mockFetch);

		const seedPhrase =
			'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about';
		await authStore.loginWithSeedPhrase(seedPhrase, '/test');

		await authStore.updateDisplayName();

		const currentIdentity = get(authStore.currentIdentity);

		expect(currentIdentity?.displayName).toBe(mockDisplayName);
		expect(mockFetch).toHaveBeenCalled();
	});

	it('should not update display name when API returns no displayName', async () => {
		const mockFetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({
				success: true,
				data: {}
			})
		});
		vi.stubGlobal('fetch', mockFetch);

		const seedPhrase =
			'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about';
		await authStore.loginWithSeedPhrase(seedPhrase, '/test');

		await authStore.updateDisplayName();

		const currentIdentity = get(authStore.currentIdentity);

		expect(currentIdentity?.displayName).toBeUndefined();
	});

	it('should handle fetch errors gracefully', async () => {
		const mockFetch = vi.fn().mockRejectedValue(new Error('Network error'));
		vi.stubGlobal('fetch', mockFetch);

		const seedPhrase =
			'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about';
		await authStore.loginWithSeedPhrase(seedPhrase, '/test');

		await expect(authStore.updateDisplayName()).resolves.not.toThrow();

		const currentIdentity = get(authStore.currentIdentity);

		expect(currentIdentity?.displayName).toBeUndefined();
	});
});
