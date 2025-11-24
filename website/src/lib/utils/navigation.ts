import { goto } from '$app/navigation';

/**
 * Navigate to the login page with optional return URL
 * @param returnUrl - URL to return to after successful login (defaults to current page)
 */
export function navigateToLogin(returnUrl?: string): void {
	const params = new URLSearchParams();
	if (returnUrl) {
		params.set('returnUrl', returnUrl);
	}
	goto(`/login?${params.toString()}`);
}
