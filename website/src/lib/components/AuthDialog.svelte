<script lang="ts">
	import AuthFlow from './AuthFlow.svelte';
	import type { AccountInfo } from '$lib/stores/auth';
	import Icon from './Icons.svelte';

	let { open = $bindable(false) } = $props<{
		open: boolean;
	}>();

	function handleClose() {
		open = false;
	}

	function handleSuccess(account: AccountInfo) {
		console.log('Auth success:', account);
		open = false;
	}
</script>

{#if open}
	<!-- Backdrop -->
	<div
		class="fixed inset-0 bg-base/90 backdrop-blur-sm z-50 flex items-center justify-center p-4"
		onclick={(e) => {
			if (e.target === e.currentTarget) handleClose();
		}}
		onkeydown={(e) => e.key === 'Escape' && handleClose()}
		role="button"
		tabindex="0"
	>
		<!-- Dialog -->
		<div
			class="bg-surface border border-neutral-800 p-6 md:p-8 max-w-lg w-full max-h-[90vh] overflow-y-auto"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			role="dialog"
			tabindex="-1"
		>
			<!-- Close button -->
			<button
				onclick={handleClose}
				class="absolute top-4 right-4 text-neutral-500 hover:text-white transition-colors"
				aria-label="Close dialog"
			>
				<Icon name="x" size={20} />
			</button>

			<AuthFlow onSuccess={handleSuccess} />
		</div>
	</div>
{/if}
