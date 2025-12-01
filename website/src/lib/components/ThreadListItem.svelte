<script lang="ts">
	import type { MessageThread } from '$lib/types/generated/MessageThread';
	import UnreadBadge from './UnreadBadge.svelte';

	interface Props {
		thread: MessageThread;
		unreadCount?: number;
		messageCount?: number;
		onClick?: () => void;
	}

	let { thread, unreadCount = 0, messageCount = 0, onClick }: Props = $props();

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
			return 'Yesterday';
		}

		return date.toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric'
		});
	}

	const hasUnread = $derived(unreadCount > 0);
	const containerClass = $derived(
		hasUnread
			? 'bg-white/15 border-blue-500/40 hover:bg-white/20'
			: 'bg-white/5 border-white/10 hover:bg-white/10'
	);
</script>

<button
	onclick={onClick}
	class="w-full px-4 py-3 rounded-xl border {containerClass} transition-all text-left"
>
	<div class="flex items-start justify-between gap-3">
		<div class="flex-1 min-w-0">
			<div class="flex items-center gap-2 mb-1">
				<h3 class="font-semibold text-white truncate" class:font-bold={hasUnread}>
					{thread.subject}
				</h3>
				{#if hasUnread}
					<UnreadBadge count={unreadCount} />
				{/if}
			</div>
			<p class="text-sm text-white/60 truncate">
				Contract: {thread.contract_id.slice(0, 8)}...{thread.contract_id.slice(-8)}
			</p>
		</div>
		<div class="flex-shrink-0">
			<span class="text-xs text-white/50">
				{formatTimestamp(thread.last_message_at_ns)}
			</span>
		</div>
	</div>
	<div class="mt-2 text-xs text-white/40">
		{thread.status} • {messageCount} {messageCount === 1 ? 'message' : 'messages'}
	</div>
</button>
