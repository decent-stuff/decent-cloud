<script lang="ts">
	import { page } from '$app/stores';
	import { navigateToLogin } from '$lib/utils/navigation';
	import Icon from './Icons.svelte';

	interface Props {
		isOpen: boolean;
		onClose: () => void;
		message?: string;
	}

	let { isOpen = false, onClose, message = 'You need an account to perform this action' }: Props = $props();

	function handleAuth() {
		navigateToLogin($page.url.pathname);
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
		class="fixed inset-0 z-50 flex items-center justify-center bg-base/90 backdrop-blur-sm"
		onclick={handleBackdropClick}
		onkeydown={handleKeyDown}
		role="dialog"
		aria-modal="true"
		aria-labelledby="auth-modal-title"
		tabindex="0"
	>
		<div class="bg-surface border border-neutral-800 p-8 max-w-md w-full mx-4">
			<div class="flex items-center gap-4 mb-6">
				<div class="w-12 h-12 bg-surface-elevated border border-neutral-700 flex items-center justify-center">
					<Icon name="lock" size={22} class="text-primary-400" />
				</div>
				<h2 id="auth-modal-title" class="text-xl font-bold text-white">Authentication Required</h2>
			</div>
			<p class="text-neutral-400 mb-8">{message}</p>

			<div class="space-y-3">
				<button
					onclick={handleAuth}
					class="w-full px-6 py-3 bg-primary-500 hover:bg-primary-400 font-semibold text-base transition-colors flex items-center justify-center gap-2"
				>
					<Icon name="login" size={18} />
					<span>Sign In</span>
				</button>
				<button
					onclick={onClose}
					class="w-full px-6 py-3 text-neutral-500 hover:text-white transition-colors"
				>
					Continue Browsing
				</button>
			</div>
		</div>
	</div>
{/if}
