/**
 * Unit tests for auth-guard utility
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { requireAuth } from './auth-guard';
import * as navigation from '$app/navigation';

// Mock $app/navigation
vi.mock('$app/navigation', () => ({
	goto: vi.fn()
}));

describe('auth-guard', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	describe('requireAuth', () => {
		it('should return true when user is authenticated', () => {
			const result = requireAuth(true, '/dashboard/account');

			expect(result).toBe(true);
			expect(navigation.goto).not.toHaveBeenCalled();
		});

		it('should return false and redirect when user is not authenticated', () => {
			const result = requireAuth(false, '/dashboard/account');

			expect(result).toBe(false);
			expect(navigation.goto).toHaveBeenCalledWith('/login?returnUrl=%2Fdashboard%2Faccount');
		});

		it('should redirect with returnUrl parameter', () => {
			requireAuth(false, '/dashboard/rentals');

			expect(navigation.goto).toHaveBeenCalledWith('/login?returnUrl=%2Fdashboard%2Frentals');
		});

		it('should handle undefined returnUrl', () => {
			const result = requireAuth(false);

			expect(result).toBe(false);
			expect(navigation.goto).toHaveBeenCalledWith('/login?');
		});

		it('should encode returnUrl parameter correctly', () => {
			requireAuth(false, '/dashboard/user/test@example');

			// Check that the URL was encoded properly
			const callArg = (navigation.goto as any).mock.calls[0][0];
			expect(callArg).toContain('returnUrl=');
			expect(callArg).toBe('/login?returnUrl=%2Fdashboard%2Fuser%2Ftest%40example');
		});

		it('should not redirect when already authenticated', () => {
			requireAuth(true);
			requireAuth(true, '/dashboard');
			requireAuth(true, '/dashboard/account');

			expect(navigation.goto).not.toHaveBeenCalled();
		});
	});
});
