<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';

	let loading = $state(true);
	let sessionId = $state<string | null>(null);

	onMount(() => {
		sessionId = $page.url.searchParams.get('session_id');

		if (!sessionId) {
			console.error('No session_id in URL');
		}

		loading = false;

		// Auto-redirect to contracts page after 5 seconds
		setTimeout(() => {
			goto('/dashboard/contracts');
		}, 5000);
	});

	function navigateToContracts() {
		goto('/dashboard/contracts');
	}
</script>

<div class="min-h-screen bg-gradient-to-br from-gray-900 via-blue-900 to-purple-900 flex items-center justify-center p-4">
	<div class="bg-gradient-to-br from-gray-900 to-gray-800 rounded-2xl max-w-2xl w-full border border-white/20 shadow-2xl p-8">
		<div class="text-center">
			{#if loading}
				<div class="flex justify-center mb-6">
					<div class="w-16 h-16 border-4 border-white border-t-transparent rounded-full animate-spin"></div>
				</div>
				<h1 class="text-2xl font-bold text-white mb-2">Processing Payment...</h1>
			{:else}
				<div class="flex justify-center mb-6">
					<div class="w-16 h-16 bg-green-500/20 rounded-full flex items-center justify-center">
						<svg class="w-10 h-10 text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
						</svg>
					</div>
				</div>
				<h1 class="text-3xl font-bold text-white mb-4">Payment Successful</h1>
				<p class="text-white/70 text-lg mb-6">
					Your payment has been processed successfully. Your rental request is now being prepared.
				</p>
				<p class="text-white/60 text-sm mb-8">
					You will receive an email confirmation shortly. The provider will review your request and provision your resources.
				</p>
				<div class="flex flex-col sm:flex-row gap-3 justify-center">
					<button
						onclick={navigateToContracts}
						class="px-6 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold hover:brightness-110 transition-all text-white"
					>
						View My Contracts
					</button>
				</div>
				<p class="text-white/50 text-xs mt-6">
					Redirecting to your contracts in 5 seconds...
				</p>
			{/if}
		</div>
	</div>
</div>
