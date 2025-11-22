import { API_BASE_URL } from './api';
import type { UserProfile } from '$lib/types/generated/UserProfile';
import type { UserContact } from '$lib/types/generated/UserContact';
import type { UserSocial } from '$lib/types/generated/UserSocial';

interface ApiResponse<T> {
	success: boolean;
	data?: T;
	error?: string;
}

/**
 * Get account profile (public, no auth required)
 */
export async function getUserProfile(username: string): Promise<UserProfile | null> {
	const response = await fetch(`${API_BASE_URL}/api/v1/accounts/${username}/profile`);

	if (!response.ok) {
		if (response.status === 404) {
			return null;
		}
		throw new Error(`Failed to fetch account profile: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<UserProfile>;

	if (!payload.success || !payload.data) {
		return null;
	}

	return payload.data;
}

/**
 * Get account contacts (requires authentication in backend)
 * Note: This function is deprecated - contacts are private
 */
export async function getUserContacts(username: string): Promise<UserContact[]> {
	const response = await fetch(`${API_BASE_URL}/api/v1/accounts/${username}/contacts`);

	if (!response.ok) {
		if (response.status === 404 || response.status === 401) {
			return [];
		}
		throw new Error(`Failed to fetch account contacts: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<UserContact[]>;

	if (!payload.success || !payload.data) {
		return [];
	}

	return payload.data;
}

/**
 * Get account social links (public, no auth required)
 */
export async function getUserSocials(username: string): Promise<UserSocial[]> {
	const response = await fetch(`${API_BASE_URL}/api/v1/accounts/${username}/socials`);

	if (!response.ok) {
		if (response.status === 404) {
			return [];
		}
		throw new Error(`Failed to fetch account socials: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<UserSocial[]>;

	if (!payload.success || !payload.data) {
		return [];
	}

	return payload.data;
}
