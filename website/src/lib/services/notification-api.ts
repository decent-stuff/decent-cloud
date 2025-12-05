import type { Ed25519KeyIdentity } from '@dfinity/identity';
import { signRequest } from './auth-api';
import { API_BASE_URL } from './api';

/**
 * Notification configuration from API
 */
export interface NotificationConfig {
	chatwootPortalSlug?: string;
	notifyTelegram: boolean;
	notifyEmail: boolean;
	notifySms: boolean;
	telegramChatId?: string;
	notifyPhone?: string;
	notifyEmailAddress?: string;
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
 * Get user notification configuration
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
 * Get user notification usage for today
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
		notifyTelegram: boolean;
		notifyEmail: boolean;
		notifySms: boolean;
		telegramChatId?: string;
		notifyPhone?: string;
		notifyEmailAddress?: string;
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

/**
 * Result from testing a notification channel
 */
export interface TestNotificationResult {
	sent: boolean;
	message: string;
}

/**
 * Send a test notification to a specific channel
 */
export async function testNotificationChannel(
	identity: Ed25519KeyIdentity,
	channel: 'telegram' | 'email' | 'sms'
): Promise<TestNotificationResult> {
	const { headers, body } = await signRequest(
		identity,
		'POST',
		'/api/v1/providers/me/notification-test',
		{ channel }
	);

	const response = await fetch(`${API_BASE_URL}/api/v1/providers/me/notification-test`, {
		method: 'POST',
		headers: headers as HeadersInit,
		body
	});

	if (!response.ok) {
		throw new Error(`Failed to test notification (HTTP ${response.status})`);
	}

	const result: ApiResponse<TestNotificationResult> = await response.json();
	if (!result.success || !result.data) {
		throw new Error(result.error || 'Failed to test notification');
	}
	return result.data;
}

/**
 * Send a test escalation notification to all enabled channels
 */
export async function testEscalationNotification(
	identity: Ed25519KeyIdentity
): Promise<TestNotificationResult> {
	const { headers } = await signRequest(
		identity,
		'POST',
		'/api/v1/providers/me/notification-test/escalation',
		{}
	);

	const response = await fetch(`${API_BASE_URL}/api/v1/providers/me/notification-test/escalation`, {
		method: 'POST',
		headers: headers as HeadersInit,
		body: JSON.stringify({})
	});

	if (!response.ok) {
		throw new Error(`Failed to test escalation (HTTP ${response.status})`);
	}

	const result: ApiResponse<TestNotificationResult> = await response.json();
	if (!result.success || !result.data) {
		throw new Error(result.error || 'Failed to test escalation');
	}
	return result.data;
}
