<script lang="ts">
	import type { Message } from '$lib/types/generated/Message';
	import MessageBubble from './MessageBubble.svelte';

	interface Props {
		messages: Message[];
		currentUserPubkey: string;
		onMarkRead?: (messageId: string) => void;
	}

	let { messages, currentUserPubkey, onMarkRead }: Props = $props();

	let scrollContainer = $state<HTMLDivElement | null>(null);
	let lastMessageCount = $state(0);

	function formatDateSeparator(timestampNs: number): string {
		const date = new Date(timestampNs / 1_000_000);
		const now = new Date();
		const isToday = date.toDateString() === now.toDateString();

		if (isToday) return 'Today';

		const yesterday = new Date(now);
		yesterday.setDate(yesterday.getDate() - 1);
		const isYesterday = date.toDateString() === yesterday.toDateString();

		if (isYesterday) return 'Yesterday';

		return date.toLocaleDateString('en-US', {
			month: 'long',
			day: 'numeric',
			year: date.getFullYear() !== now.getFullYear() ? 'numeric' : undefined
		});
	}

	function shouldShowDateSeparator(index: number): boolean {
		if (index === 0) return true;
		const currentDate = new Date(messages[index].created_at_ns / 1_000_000).toDateString();
		const prevDate = new Date(messages[index - 1].created_at_ns / 1_000_000).toDateString();
		return currentDate !== prevDate;
	}

	function scrollToBottom() {
		if (scrollContainer) {
			scrollContainer.scrollTop = scrollContainer.scrollHeight;
		}
	}

	function handleScroll() {
		if (!scrollContainer || !onMarkRead) return;

		const unreadMessages = messages.filter((msg) => !msg.is_read && msg.sender_pubkey !== currentUserPubkey);

		for (const msg of unreadMessages) {
			const element = scrollContainer.querySelector(`[data-message-id="${msg.message_id}"]`);
			if (element) {
				const rect = element.getBoundingClientRect();
				const containerRect = scrollContainer.getBoundingClientRect();

				if (rect.top >= containerRect.top && rect.bottom <= containerRect.bottom) {
					onMarkRead(msg.message_id);
				}
			}
		}
	}

	$effect(() => {
		if (messages.length > lastMessageCount) {
			scrollToBottom();
			lastMessageCount = messages.length;
		}
	});
</script>

<div
	bind:this={scrollContainer}
	onscroll={handleScroll}
	class="flex-1 overflow-y-auto px-4 py-4 space-y-1"
>
	{#if messages.length === 0}
		<div class="flex items-center justify-center h-full text-white/50 text-sm">
			No messages yet
		</div>
	{:else}
		{#each messages as message, i (message.message_id)}
			{#if shouldShowDateSeparator(i)}
				<div class="flex justify-center my-4">
					<div class="px-3 py-1 rounded-full bg-white/5 text-xs text-white/50 font-medium">
						{formatDateSeparator(message.created_at_ns)}
					</div>
				</div>
			{/if}
			<div data-message-id={message.message_id}>
				<MessageBubble
					{message}
					isOwnMessage={message.sender_pubkey === currentUserPubkey}
					senderName={message.sender_pubkey === currentUserPubkey ? undefined : 'Other Party'}
				/>
			</div>
		{/each}
	{/if}
</div>
