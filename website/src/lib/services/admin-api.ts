import type { Ed25519KeyIdentity } from '@dfinity/identity';
import { signRequest } from './auth-api';
import { API_BASE_URL } from './api';

/**
 * Email queue entry from API
 */
export interface EmailQueueEntry {
	toAddr: string;
	fromAddr: string;
	subject: string;
	body: string;
	isHtml: number;
	emailType: string;
	status: string;
	attempts: number;
	maxAttempts: number;
	lastError: string | null;
	createdAt: number;
	lastAttemptedAt: number | null;
	sentAt: number | null;
}

/**
 * Email queue statistics
 */
export interface EmailStats {
	pending: number;
	sent: number;
	failed: number;
	total: number;
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
 * Authenticated fetch helper for admin endpoints
 */
async function authenticatedFetch<T>(
	identity: Ed25519KeyIdentity,
	method: string,
	path: string,
	bodyData?: unknown
): Promise<T> {
	const { headers, body } = await signRequest(identity, method, path, bodyData);

	const response = await fetch(`${API_BASE_URL}${path}`, {
		method,
		headers: headers as HeadersInit,
		body
	});

	if (!response.ok) {
		const text = await response.text().catch(() => '');
		let errorMessage = `Request failed (HTTP ${response.status} ${response.statusText})`;
		try {
			const errorData = JSON.parse(text);
			if (errorData.error) {
				errorMessage = `${errorData.error} (HTTP ${response.status})`;
			}
		} catch {
			if (text) {
				errorMessage = `Request failed (HTTP ${response.status} ${response.statusText}: ${text.substring(0, 200)})`;
			}
		}
		throw new Error(errorMessage);
	}

	const result: ApiResponse<T> = await response.json();

	if (!result.success) {
		throw new Error(result.error || 'Request failed');
	}

	if (!result.data) {
		throw new Error('No data in response');
	}

	return result.data;
}

/**
 * Get failed emails from the queue
 */
export async function getFailedEmails(
	identity: Ed25519KeyIdentity,
	limit?: number
): Promise<EmailQueueEntry[]> {
	const path = limit ? `/api/v1/admin/emails/failed?limit=${limit}` : '/api/v1/admin/emails/failed';
	return authenticatedFetch<EmailQueueEntry[]>(identity, 'GET', path);
}

/**
 * Get email queue statistics
 */
export async function getEmailStats(identity: Ed25519KeyIdentity): Promise<EmailStats> {
	return authenticatedFetch<EmailStats>(identity, 'GET', '/api/v1/admin/emails/stats');
}

/**
 * Reset a single email for retry
 */
export async function resetEmail(identity: Ed25519KeyIdentity, emailId: string): Promise<string> {
	return authenticatedFetch<string>(
		identity,
		'POST',
		`/api/v1/admin/emails/reset/${emailId}`
	);
}

/**
 * Retry all failed emails
 */
export async function retryAllFailed(identity: Ed25519KeyIdentity): Promise<string> {
	return authenticatedFetch<string>(identity, 'POST', '/api/v1/admin/emails/retry-all-failed');
}
