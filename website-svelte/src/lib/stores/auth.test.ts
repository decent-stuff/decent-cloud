import { describe, it, expect, beforeEach, vi } from 'vitest';
import { authStore } from './auth';

vi.mock('@dfinity/auth-client', () => ({
	AuthClient: {
		create: vi.fn().mockResolvedValue({
			isAuthenticated: vi.fn().mockResolvedValue(false),
			login: vi.fn(),
			logout: vi.fn()
		})
	}
}));

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
});
