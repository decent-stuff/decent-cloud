<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';

	let contractId = $state<string | null>(null);

	onMount(() => {
		contractId = $page.url.searchParams.get('contract_id');
	});

	function navigateToMarketplace() {
		goto('/dashboard/marketplace');
	}

	function navigateToRentals() {
		goto('/dashboard/rentals');
	}
</script>

<div class="min-h-screen bg-gradient-to-br from-base via-surface to-surface flex items-center justify-center p-4">
	<div class="bg-gradient-to-br from-base to-gray-800 rounded-2xl max-w-2xl w-full border border-glass/15 shadow-2xl p-8">
		<div class="text-center">
			<div class="flex justify-center mb-6">
				<div class="w-16 h-16 bg-yellow-500/20 rounded-full flex items-center justify-center">
					<svg class="w-10 h-10 text-yellow-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
					</svg>
				</div>
			</div>
			<h1 class="text-3xl font-bold text-white mb-4">Payment Cancelled</h1>
			<p class="text-white/70 text-lg mb-6">
				Your payment was cancelled. No charges have been made to your account.
			</p>
			<p class="text-white/60 text-sm mb-8">
				Your rental request has been created but payment was not completed. You can try again or browse other offerings.
			</p>
			<div class="flex flex-col sm:flex-row gap-3 justify-center">
				<button
					onclick={navigateToMarketplace}
					class="px-6 py-3 bg-gradient-to-r from-primary-500 to-primary-600 rounded-lg font-semibold hover:brightness-110 transition-all text-white"
				>
					Browse Marketplace
				</button>
				{#if contractId}
					<button
						onclick={navigateToRentals}
						class="px-6 py-3 bg-glass/10 text-white rounded-lg font-semibold hover:bg-glass/15 transition-all"
					>
						View My Rentals
					</button>
				{/if}
			</div>
		</div>
	</div>
</div>
