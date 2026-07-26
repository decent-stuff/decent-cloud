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
		body: method === 'GET' || method === 'HEAD' ? undefined : body
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
 * Get sent emails from the queue
 */
export async function getSentEmails(
	identity: Ed25519KeyIdentity,
	limit?: number
): Promise<EmailQueueEntry[]> {
	const path = limit ? `/api/v1/admin/emails/sent?limit=${limit}` : '/api/v1/admin/emails/sent';
	return authenticatedFetch<EmailQueueEntry[]>(identity, 'GET', path);
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

/**
 * Admin account info from API
 */
export interface AdminAccountInfo {
	id: string;
	username: string;
	email: string | null;
	emailVerified: boolean;
	createdAt: number;
	lastLoginAt: number | null;
	isAdmin: boolean;
	activeKeys: number;
	totalKeys: number;
}

/**
 * Send test email to verify configuration
 */
export async function sendTestEmail(
	identity: Ed25519KeyIdentity,
	toEmail: string
): Promise<string> {
	return authenticatedFetch<string>(identity, 'POST', '/api/v1/admin/emails/test', { toEmail });
}

/**
 * Lookup account by username
 */
export async function getAccount(
	identity: Ed25519KeyIdentity,
	username: string
): Promise<AdminAccountInfo> {
	return authenticatedFetch<AdminAccountInfo>(
		identity,
		'GET',
		`/api/v1/admin/accounts/${encodeURIComponent(username)}`
	);
}

/**
 * Set email verification status for an account
 */
export async function setEmailVerified(
	identity: Ed25519KeyIdentity,
	username: string,
	verified: boolean
): Promise<string> {
	return authenticatedFetch<string>(
		identity,
		'POST',
		`/api/v1/admin/accounts/${encodeURIComponent(username)}/email-verified`,
		{ verified }
	);
}

/**
 * Set or clear email for an account
 */
export async function setAccountEmail(
	identity: Ed25519KeyIdentity,
	username: string,
	email: string | null
): Promise<string> {
	return authenticatedFetch<string>(
		identity,
		'POST',
		`/api/v1/admin/accounts/${encodeURIComponent(username)}/email`,
		{ email }
	);
}

/**
 * Summary of resources deleted when deleting an account
 */
export interface AccountDeletionSummary {
	offeringsDeleted: number;
	contractsAsRequester: number;
	contractsAsProvider: number;
	publicKeysDeleted: number;
	providerProfileDeleted: boolean;
}

/**
 * Delete an account and all associated resources
 */
export async function deleteAccount(
	identity: Ed25519KeyIdentity,
	username: string
): Promise<AccountDeletionSummary> {
	return authenticatedFetch<AccountDeletionSummary>(
		identity,
		'DELETE',
		`/api/v1/admin/accounts/${encodeURIComponent(username)}`
	);
}

/**
 * Paginated list of accounts
 */
export interface AdminAccountListResponse {
	accounts: AdminAccountInfo[];
	total: number;
	limit: number;
	offset: number;
}

/**
 * List all accounts with pagination
 */
export async function listAccounts(
	identity: Ed25519KeyIdentity,
	limit?: number,
	offset?: number
): Promise<AdminAccountListResponse> {
	const params = new URLSearchParams();
	if (limit !== undefined) params.set('limit', limit.toString());
	if (offset !== undefined) params.set('offset', offset.toString());
	const query = params.toString();
	const path = query ? `/api/v1/admin/accounts?${query}` : '/api/v1/admin/accounts';
	return authenticatedFetch<AdminAccountListResponse>(identity, 'GET', path);
}

/**
 * Set admin status for an account
 */
export async function setAdminStatus(
	identity: Ed25519KeyIdentity,
	username: string,
	isAdmin: boolean
): Promise<string> {
	return authenticatedFetch<string>(
		identity,
		'POST',
		`/api/v1/admin/accounts/${encodeURIComponent(username)}/admin-status`,
		{ isAdmin }
	);
}

/**
 * A single refund request for admin review (hex-encoded byte fields).
 */
export interface AdminRefundRequestInfo {
	id: number;
	contractId: string;
	requesterPubkey: string;
	refundAmountE9s: number;
	reason: string;
	status: string;
	userLatestPaymentE9s: number;
	capExceeded: boolean;
	paymentIntentId: string;
	currency: string;
	stripeDisputeId: string | null;
	stripeRefundId: string | null;
	createdAtNs: number;
	reviewedAtNs: number | null;
	reviewedBy: string | null;
	reviewNote: string | null;
}

/**
 * Paginated list of refund requests for admin listing.
 */
export interface AdminRefundRequestListResponse {
	requests: AdminRefundRequestInfo[];
	total: number;
	limit: number;
	offset: number;
}

/**
 * List refund requests, optionally filtered by status (default: pending).
 * Pass status `"all"` to return every request regardless of state.
 */
export async function listRefundRequests(
	identity: Ed25519KeyIdentity,
	status?: string,
	limit?: number,
	offset?: number
): Promise<AdminRefundRequestListResponse> {
	const params = new URLSearchParams();
	if (status !== undefined) params.set('status', status);
	if (limit !== undefined) params.set('limit', limit.toString());
	if (offset !== undefined) params.set('offset', offset.toString());
	const query = params.toString();
	const path = query ? `/api/v1/admin/refund-requests?${query}` : '/api/v1/admin/refund-requests';
	return authenticatedFetch<AdminRefundRequestListResponse>(identity, 'GET', path);
}

/**
 * Approve a pending refund request. Issues a REAL Stripe refund.
 */
export async function approveRefundRequest(
	identity: Ed25519KeyIdentity,
	id: number,
	note?: string
): Promise<AdminRefundRequestInfo> {
	return authenticatedFetch<AdminRefundRequestInfo>(
		identity,
		'POST',
		`/api/v1/admin/refund-requests/${id}/approve`,
		{ note }
	);
}

/**
 * Decline a pending refund request. No Stripe refund is issued.
 */
export async function declineRefundRequest(
	identity: Ed25519KeyIdentity,
	id: number,
	note?: string
): Promise<AdminRefundRequestInfo> {
	return authenticatedFetch<AdminRefundRequestInfo>(
		identity,
		'POST',
		`/api/v1/admin/refund-requests/${id}/decline`,
		{ note }
	);
}
