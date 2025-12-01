<script lang="ts">
	interface Props {
		onSend: (body: string) => Promise<void>;
		disabled?: boolean;
		placeholder?: string;
	}

	let { onSend, disabled = false, placeholder = 'Type a message...' }: Props = $props();

	let messageBody = $state('');
	let isSending = $state(false);
	let textarea = $state<HTMLTextAreaElement | null>(null);

	function adjustTextareaHeight() {
		if (!textarea) return;
		textarea.style.height = 'auto';
		textarea.style.height = `${Math.min(textarea.scrollHeight, 150)}px`;
	}

	async function handleSend() {
		if (!messageBody.trim() || isSending || disabled) return;

		const body = messageBody.trim();
		messageBody = '';
		adjustTextareaHeight();
		isSending = true;

		try {
			await onSend(body);
		} catch (error) {
			console.error('Failed to send message:', error);
			messageBody = body;
		} finally {
			isSending = false;
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault();
			handleSend();
		}
	}

	$effect(() => {
		adjustTextareaHeight();
	});
</script>

<div class="border-t border-white/10 p-4 bg-white/5 backdrop-blur-sm">
	<div class="flex items-end gap-2">
		<textarea
			bind:this={textarea}
			bind:value={messageBody}
			oninput={adjustTextareaHeight}
			onkeydown={handleKeydown}
			{disabled}
			{placeholder}
			rows="1"
			class="flex-1 px-4 py-2 bg-white/10 border border-white/20 rounded-xl text-white placeholder-white/40 resize-none focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:opacity-50 disabled:cursor-not-allowed"
		></textarea>
		<button
			onclick={handleSend}
			disabled={!messageBody.trim() || isSending || disabled}
			class="px-5 py-2 bg-gradient-to-r from-blue-500 to-purple-600 rounded-xl font-semibold text-white hover:brightness-110 transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:brightness-100 flex items-center gap-2"
		>
			{#if isSending}
				<span class="inline-block w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin"></span>
			{:else}
				<span>Send</span>
			{/if}
		</button>
	</div>
	<div class="mt-2 text-xs text-white/40">
		Press Enter to send, Shift+Enter for new line
	</div>
</div>
