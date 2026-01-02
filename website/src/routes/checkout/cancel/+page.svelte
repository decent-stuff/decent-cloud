<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import Icon from '$lib/components/Icons.svelte';

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

<div class="min-h-screen bg-base bg-grid bg-radial flex items-center justify-center p-4">
	<div class="card max-w-2xl w-full p-8">
		<div class="text-center">
			<div class="flex justify-center mb-6">
				<div class="icon-box">
					<Icon name="alert" size={24} class="text-warning" />
				</div>
			</div>
			<h1 class="text-2xl font-bold text-white mb-4">Payment Cancelled</h1>
			<p class="text-neutral-400 text-base mb-6">
				Your payment was cancelled. No charges have been made to your account.
			</p>
			<p class="text-neutral-500 text-sm mb-8">
				Your rental request has been created but payment was not completed. You can try again or browse other offerings.
			</p>
			<div class="flex flex-col sm:flex-row gap-3 justify-center">
				<button onclick={navigateToMarketplace} class="btn-primary">
					Browse Marketplace
				</button>
				{#if contractId}
					<button onclick={navigateToRentals} class="btn-secondary">
						View My Rentals
					</button>
				{/if}
			</div>
		</div>
	</div>
</div>
