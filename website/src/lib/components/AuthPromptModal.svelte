<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';

	interface Props {
		isOpen: boolean;
		onClose: () => void;
		message?: string;
	}

	let { isOpen = false, onClose, message = "You need an account to perform this action" }: Props = $props();

	function handleAuth() {
		goto(`/login?returnUrl=${$page.url.pathname}`);
	}

	function handleBackdropClick(event: MouseEvent) {
		if (event.target === event.currentTarget) {
			onClose();
		}
	}

	function handleKeyDown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			onClose();
		}
	}
</script>

{#if isOpen}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
		onclick={handleBackdropClick}
		onkeydown={handleKeyDown}
		role="dialog"
		aria-modal="true"
		aria-labelledby="auth-modal-title"
		tabindex="0"
	>
		<div class="bg-gradient-to-br from-gray-900 to-blue-900 rounded-2xl p-8 max-w-md w-full mx-4 border border-white/20 shadow-2xl">
			<h2 id="auth-modal-title" class="text-2xl font-bold text-white mb-4">Authentication Required</h2>
			<p class="text-white/70 mb-6">{message}</p>

			<div class="space-y-3">
				<button
					onclick={handleAuth}
					class="w-full px-6 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 transition-all"
				>
					Login / Create Account
				</button>
				<button
					onclick={onClose}
					class="w-full px-6 py-3 text-white/60 hover:text-white transition-colors"
				>
					Continue Browsing
				</button>
			</div>
		</div>
	</div>
{/if}
