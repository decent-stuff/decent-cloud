<script lang="ts">
	import AuthFlow from './AuthFlow.svelte';
	import type { AccountInfo } from '$lib/stores/auth';

	let { open = $bindable(false) } = $props<{
		open: boolean;
	}>();

	function handleClose() {
		open = false;
	}

	function handleSuccess(account: AccountInfo) {
		console.log('Auth success:', account);
		open = false;
		// Navigate to dashboard (already handled by auth store)
	}
</script>

{#if open}
	<!-- Backdrop -->
	<div
		class="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center p-4"
		onclick={(e) => {
			if (e.target === e.currentTarget) handleClose();
		}}
		onkeydown={(e) => e.key === 'Escape' && handleClose()}
		role="button"
		tabindex="0"
	>
		<!-- Dialog -->
		<div
			class="bg-gray-900/95 rounded-2xl p-6 md:p-8 max-w-lg w-full border border-white/20 shadow-2xl max-h-[90vh] overflow-y-auto"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			role="dialog"
			tabindex="-1"
		>
			<AuthFlow onSuccess={handleSuccess} />
		</div>
	</div>
{/if}
