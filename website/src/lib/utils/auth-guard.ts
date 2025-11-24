/**
 * Auth Guard Utility - Protected Route Navigation
 *
 * Provides utilities for protecting routes that require authentication.
 * When a non-authenticated user attempts to access a protected route,
 * they are redirected to the login page with a returnUrl parameter.
 */

import { goto } from '$app/navigation';

/**
 * Checks if user is authenticated and redirects to login page if not.
 * Preserves the current URL as a returnUrl parameter for post-login navigation.
 *
 * @param isAuthenticated - Current authentication status
 * @param returnUrl - Optional URL to return to after authentication (defaults to current path)
 * @returns true if authenticated, false if redirected
 *
 * @example
 * ```typescript
 * onMount(() => {
 *   const unsubAuth = authStore.isAuthenticated.subscribe((isAuth) => {
 *     requireAuth(isAuth, $page.url.pathname);
 *   });
 * });
 * ```
 */
export function requireAuth(isAuthenticated: boolean, returnUrl?: string): boolean {
	if (!isAuthenticated) {
		const params = new URLSearchParams();
		if (returnUrl) {
			params.set('returnUrl', returnUrl);
		}
		goto(`/login?${params.toString()}`);
		return false;
	}
	return true;
}
