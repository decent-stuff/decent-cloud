<script lang="ts">
	import type { Message } from '$lib/types/generated/Message';

	interface Props {
		message: Message;
		isOwnMessage: boolean;
		senderName?: string;
	}

	let { message, isOwnMessage, senderName }: Props = $props();

	function formatTimestamp(timestampNs: number): string {
		const date = new Date(timestampNs / 1_000_000);
		const now = new Date();
		const isToday = date.toDateString() === now.toDateString();

		if (isToday) {
			return date.toLocaleTimeString('en-US', {
				hour: 'numeric',
				minute: '2-digit'
			});
		}

		const yesterday = new Date(now);
		yesterday.setDate(yesterday.getDate() - 1);
		const isYesterday = date.toDateString() === yesterday.toDateString();

		if (isYesterday) {
			return `Yesterday ${date.toLocaleTimeString('en-US', {
				hour: 'numeric',
				minute: '2-digit'
			})}`;
		}

		return date.toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			hour: 'numeric',
			minute: '2-digit'
		});
	}

	const isAiMessage = $derived(
		message.sender_role === 'assistant' || message.sender_role === 'system'
	);

	const alignmentClass = $derived(isOwnMessage ? 'justify-end' : 'justify-start');
	const bubbleClass = $derived(
		isOwnMessage
			? 'bg-gradient-to-r from-blue-500/30 to-purple-500/30 border-blue-500/40'
			: 'bg-white/10 border-white/20'
	);
</script>

<div class="flex {alignmentClass} mb-3">
	<div
		class="max-w-[70%] rounded-2xl px-4 py-2 border {bubbleClass} backdrop-blur-sm"
	>
		{#if !isOwnMessage && senderName}
			<div class="flex items-center gap-2 mb-1">
				<span class="text-xs font-semibold text-white/80">{senderName}</span>
				{#if isAiMessage}
					<span
						class="text-xs px-2 py-0.5 rounded-full bg-purple-500/20 border border-purple-500/40 text-purple-300"
					>
						AI
					</span>
				{/if}
			</div>
		{/if}

		<p class="text-white whitespace-pre-wrap break-words">{message.body}</p>

		<div class="flex items-center gap-2 mt-1 text-xs text-white/50">
			<span>{formatTimestamp(message.created_at_ns)}</span>
			{#if isOwnMessage}
				<span>{message.is_read ? '✓✓' : '✓'}</span>
			{/if}
		</div>
	</div>
</div>
