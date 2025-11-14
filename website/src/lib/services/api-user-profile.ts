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
 * Get user profile (public, no auth required)
 */
export async function getUserProfile(pubkeyHash: string): Promise<UserProfile | null> {
	const response = await fetch(`${API_BASE_URL}/api/v1/users/${pubkeyHash}/profile`);

	if (!response.ok) {
		if (response.status === 404) {
			return null;
		}
		throw new Error(`Failed to fetch user profile: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<UserProfile>;

	if (!payload.success || !payload.data) {
		return null;
	}

	return payload.data;
}

/**
 * Get user contacts (public, no auth required)
 */
export async function getUserContacts(pubkeyHash: string): Promise<UserContact[]> {
	const response = await fetch(`${API_BASE_URL}/api/v1/users/${pubkeyHash}/contacts`);

	if (!response.ok) {
		if (response.status === 404) {
			return [];
		}
		throw new Error(`Failed to fetch user contacts: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<UserContact[]>;

	if (!payload.success || !payload.data) {
		return [];
	}

	return payload.data;
}

/**
 * Get user social links (public, no auth required)
 */
export async function getUserSocials(pubkeyHash: string): Promise<UserSocial[]> {
	const response = await fetch(`${API_BASE_URL}/api/v1/users/${pubkeyHash}/socials`);

	if (!response.ok) {
		if (response.status === 404) {
			return [];
		}
		throw new Error(`Failed to fetch user socials: ${response.status} ${response.statusText}`);
	}

	const payload = (await response.json()) as ApiResponse<UserSocial[]>;

	if (!payload.success || !payload.data) {
		return [];
	}

	return payload.data;
}
