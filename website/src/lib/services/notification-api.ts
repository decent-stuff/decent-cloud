import type { Ed25519KeyIdentity } from '@dfinity/identity';
import { signRequest } from './auth-api';
import { API_BASE_URL } from './api';

/**
 * Notification configuration from API
 */
export interface NotificationConfig {
	chatwootPortalSlug?: string;
	notifyVia: 'telegram' | 'email' | 'sms';
	telegramChatId?: string;
	notifyPhone?: string;
}

/**
 * API response wrapper
 */
interface ApiResponse<T> {
	success: boolean;
	data?: T;
	error?: string;
}

/**
 * Get provider notification configuration
 */
export async function getNotificationConfig(
	identity: Ed25519KeyIdentity
): Promise<NotificationConfig | null> {
	const { headers } = await signRequest(identity, 'GET', '/api/v1/providers/me/notification-config');

	const response = await fetch(`${API_BASE_URL}/api/v1/providers/me/notification-config`, {
		method: 'GET',
		headers: headers as HeadersInit
	});

	if (response.status === 404) {
		return null;
	}

	if (!response.ok) {
		const text = await response.text().catch(() => '');
		let errorMessage = `Failed to get notification config (HTTP ${response.status})`;
		try {
			const errorData = JSON.parse(text);
			if (errorData.error) {
				errorMessage = errorData.error;
			}
		} catch {
			// ignore parse errors
		}
		throw new Error(errorMessage);
	}

	const result: ApiResponse<NotificationConfig> = await response.json();

	if (!result.success) {
		// No config found is not an error
		if (result.error?.includes('not found')) {
			return null;
		}
		throw new Error(result.error || 'Failed to get notification config');
	}

	return result.data || null;
}

/**
 * Notification usage stats from API
 */
export interface NotificationUsage {
	telegramCount: number;
	smsCount: number;
	emailCount: number;
	telegramLimit: number;
	smsLimit: number;
}

/**
 * Get provider notification usage for today
 */
export async function getNotificationUsage(
	identity: Ed25519KeyIdentity
): Promise<NotificationUsage | null> {
	const { headers } = await signRequest(identity, 'GET', '/api/v1/providers/me/notification-usage');

	const response = await fetch(`${API_BASE_URL}/api/v1/providers/me/notification-usage`, {
		method: 'GET',
		headers: headers as HeadersInit
	});

	if (!response.ok) {
		return null;
	}

	const result: ApiResponse<NotificationUsage> = await response.json();
	return result.success ? result.data || null : null;
}

export async function updateNotificationConfig(
	identity: Ed25519KeyIdentity,
	config: {
		chatwootPortalSlug?: string;
		notifyVia: string;
		telegramChatId?: string;
		notifyPhone?: string;
	}
): Promise<void> {
	const { headers, body } = await signRequest(
		identity,
		'PUT',
		'/api/v1/providers/me/notification-config',
		config
	);

	const response = await fetch(`${API_BASE_URL}/api/v1/providers/me/notification-config`, {
		method: 'PUT',
		headers: headers as HeadersInit,
		body
	});

	if (!response.ok) {
		const text = await response.text().catch(() => '');
		let errorMessage = `Failed to update notification config (HTTP ${response.status})`;
		try {
			const errorData = JSON.parse(text);
			if (errorData.error) {
				errorMessage = errorData.error;
			}
		} catch {
			// ignore parse errors
		}
		throw new Error(errorMessage);
	}

	const result: ApiResponse<string> = await response.json();

	if (!result.success) {
		throw new Error(result.error || 'Failed to update notification config');
	}
}
