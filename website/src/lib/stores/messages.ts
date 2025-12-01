import { writable, get } from 'svelte/store';
import type { Ed25519KeyIdentity } from '@dfinity/identity';
import type { Message } from '$lib/types/generated/Message';
import type { MessageThread } from '$lib/types/generated/MessageThread';
import { MessageApiClient, type MessagesResponse } from '../services/message-api';
import { authStore } from './auth';

function createMessagesStore() {
	const messages = writable<Message[]>([]);
	const currentThread = writable<MessageThread | null>(null);
	const inbox = writable<MessageThread[]>([]);
	const unreadCount = writable<number>(0);
	const isLoading = writable<boolean>(false);
	const error = writable<string | null>(null);

	async function getApiClient(): Promise<MessageApiClient | null> {
		const authResult = await authStore.getAuthenticatedIdentity();
		if (!authResult) {
			error.set('Not authenticated');
			return null;
		}
		return new MessageApiClient(authResult.identity as Ed25519KeyIdentity);
	}

	return {
		messages: { subscribe: messages.subscribe },
		currentThread: { subscribe: currentThread.subscribe },
		inbox: { subscribe: inbox.subscribe },
		unreadCount: { subscribe: unreadCount.subscribe },
		isLoading: { subscribe: isLoading.subscribe },
		error: { subscribe: error.subscribe },

		async loadContractMessages(contractId: string): Promise<void> {
			isLoading.set(true);
			error.set(null);

			try {
				const client = await getApiClient();
				if (!client) return;

				const response: MessagesResponse = await client.getContractMessages(contractId);
				messages.set(response.messages);
				currentThread.set(response.thread);
			} catch (err) {
				const errorMessage = err instanceof Error ? err.message : 'Failed to load messages';
				error.set(errorMessage);
				console.error('Error loading contract messages:', err);
			} finally {
				isLoading.set(false);
			}
		},

		async sendMessage(contractId: string, body: string): Promise<void> {
			error.set(null);

			try {
				const client = await getApiClient();
				if (!client) return;

				const newMessage = await client.sendMessage(contractId, body);

				// Add new message to messages list
				messages.update((msgs) => [...msgs, newMessage]);

				// Update thread's last_message_at_ns
				currentThread.update((thread) => {
					if (thread) {
						return {
							...thread,
							last_message_at_ns: newMessage.created_at_ns
						};
					}
					return thread;
				});
			} catch (err) {
				const errorMessage = err instanceof Error ? err.message : 'Failed to send message';
				error.set(errorMessage);
				console.error('Error sending message:', err);
				throw err;
			}
		},

		async markAsRead(messageId: string): Promise<void> {
			try {
				const client = await getApiClient();
				if (!client) return;

				await client.markMessageRead(messageId);

				// Update message in local state
				messages.update((msgs) =>
					msgs.map((msg) => (msg.message_id === messageId ? { ...msg, is_read: true } : msg))
				);

				// Decrement unread count
				unreadCount.update((count) => Math.max(0, count - 1));
			} catch (err) {
				const errorMessage = err instanceof Error ? err.message : 'Failed to mark message as read';
				error.set(errorMessage);
				console.error('Error marking message as read:', err);
			}
		},

		async loadInbox(): Promise<void> {
			isLoading.set(true);
			error.set(null);

			try {
				const client = await getApiClient();
				if (!client) return;

				const threads = await client.getInbox();
				inbox.set(threads);
			} catch (err) {
				const errorMessage = err instanceof Error ? err.message : 'Failed to load inbox';
				error.set(errorMessage);
				console.error('Error loading inbox:', err);
			} finally {
				isLoading.set(false);
			}
		},

		async loadUnreadCount(): Promise<void> {
			try {
				const client = await getApiClient();
				if (!client) return;

				const count = await client.getUnreadCount();
				unreadCount.set(count);
			} catch (err) {
				console.error('Error loading unread count:', err);
			}
		},

		clear(): void {
			messages.set([]);
			currentThread.set(null);
			inbox.set([]);
			unreadCount.set(0);
			error.set(null);
			isLoading.set(false);
		}
	};
}

export const messagesStore = createMessagesStore();
