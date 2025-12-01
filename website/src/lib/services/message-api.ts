import { signRequest } from './auth-api';
import { API_BASE_URL } from './api';
import type { Ed25519KeyIdentity } from '@dfinity/identity';
import type { Message } from '$lib/types/generated/Message';
import type { MessageThread } from '$lib/types/generated/MessageThread';

export interface MessagesResponse {
	messages: Message[];
	thread: MessageThread;
}

export interface ProviderResponseMetrics {
	avgResponseTimeHours: number | null;
	responseRatePct: number;
	totalThreads: number;
	respondedThreads: number;
}

export class MessageApiClient {
	constructor(private signingIdentity: Ed25519KeyIdentity) {}

	private async authenticatedFetch(method: string, path: string, body?: unknown): Promise<Response> {
		const { headers, body: signedBody } = await signRequest(
			this.signingIdentity,
			method,
			path,
			body
		);

		return fetch(`${API_BASE_URL}${path}`, {
			method,
			headers,
			body: signedBody || undefined
		});
	}

	async getContractMessages(contractId: string): Promise<MessagesResponse> {
		const path = `/api/v1/contracts/${contractId}/messages`;
		const res = await this.authenticatedFetch('GET', path);

		if (!res.ok) {
			const errorText = await res.text();
			throw new Error(`Failed to get contract messages: ${res.status} ${res.statusText}\n${errorText}`);
		}

		const payload = await res.json();
		if (!payload.success || !payload.data) {
			throw new Error(payload.error || 'Failed to get contract messages');
		}

		return payload.data;
	}

	async sendMessage(contractId: string, body: string): Promise<Message> {
		const path = `/api/v1/contracts/${contractId}/messages`;
		const res = await this.authenticatedFetch('POST', path, { body });

		if (!res.ok) {
			const errorText = await res.text();
			throw new Error(`Failed to send message: ${res.status} ${res.statusText}\n${errorText}`);
		}

		const payload = await res.json();
		if (!payload.success || !payload.data) {
			throw new Error(payload.error || 'Failed to send message');
		}

		return payload.data;
	}

	async getContractThread(contractId: string): Promise<MessageThread> {
		const path = `/api/v1/contracts/${contractId}/thread`;
		const res = await this.authenticatedFetch('GET', path);

		if (!res.ok) {
			const errorText = await res.text();
			throw new Error(`Failed to get contract thread: ${res.status} ${res.statusText}\n${errorText}`);
		}

		const payload = await res.json();
		if (!payload.success || !payload.data) {
			throw new Error(payload.error || 'Failed to get contract thread');
		}

		return payload.data;
	}

	async markMessageRead(messageId: string): Promise<void> {
		const path = `/api/v1/messages/${messageId}/read`;
		const res = await this.authenticatedFetch('PUT', path);

		if (!res.ok) {
			const errorText = await res.text();
			throw new Error(`Failed to mark message as read: ${res.status} ${res.statusText}\n${errorText}`);
		}

		const payload = await res.json();
		if (!payload.success) {
			throw new Error(payload.error || 'Failed to mark message as read');
		}
	}

	async getUnreadCount(): Promise<number> {
		const path = `/api/v1/messages/unread-count`;
		const res = await this.authenticatedFetch('GET', path);

		if (!res.ok) {
			const errorText = await res.text();
			throw new Error(`Failed to get unread count: ${res.status} ${res.statusText}\n${errorText}`);
		}

		const payload = await res.json();
		if (!payload.success || !payload.data) {
			throw new Error(payload.error || 'Failed to get unread count');
		}

		return payload.data.count;
	}

	async getInbox(): Promise<MessageThread[]> {
		const path = `/api/v1/messages/inbox`;
		const res = await this.authenticatedFetch('GET', path);

		if (!res.ok) {
			const errorText = await res.text();
			throw new Error(`Failed to get inbox: ${res.status} ${res.statusText}\n${errorText}`);
		}

		const payload = await res.json();
		if (!payload.success || !payload.data) {
			throw new Error(payload.error || 'Failed to get inbox');
		}

		return payload.data.threads;
	}
}

/**
 * Get provider response metrics (public endpoint, no auth required)
 */
export async function getProviderResponseMetrics(pubkey: string): Promise<ProviderResponseMetrics> {
	const path = `/api/v1/providers/${pubkey}/response-metrics`;
	const res = await fetch(`${API_BASE_URL}${path}`);

	if (!res.ok) {
		const errorText = await res.text();
		throw new Error(`Failed to get provider response metrics: ${res.status} ${res.statusText}\n${errorText}`);
	}

	const payload = await res.json();
	if (!payload.success || !payload.data) {
		throw new Error(payload.error || 'Failed to get provider response metrics');
	}

	return {
		avgResponseTimeHours: payload.data.avg_response_time_hours,
		responseRatePct: payload.data.response_rate_pct,
		totalThreads: payload.data.total_threads,
		respondedThreads: payload.data.responded_threads
	};
}
