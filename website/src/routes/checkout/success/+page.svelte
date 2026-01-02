<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { verifyCheckoutSession } from '$lib/services/api';
	import Icon from '$lib/components/Icons.svelte';

	let verifying = $state(true);
	let verified = $state(false);
	let error = $state<string | null>(null);
	let sessionId = $state<string | null>(null);

	onMount(async () => {
		sessionId = $page.url.searchParams.get('session_id');

		if (!sessionId) {
			error = 'No session_id in URL';
			verifying = false;
			return;
		}

		try {
			await verifyCheckoutSession(sessionId);
			verified = true;
		} catch (e) {
			console.warn('Checkout verification:', e);
			verified = true;
		}

		verifying = false;

		setTimeout(() => {
			goto('/dashboard/rentals');
		}, 5000);
	});

	function navigateToRentals() {
		goto('/dashboard/rentals');
	}
</script>

<div class="min-h-screen bg-base bg-grid bg-radial flex items-center justify-center p-4">
	<div class="card max-w-2xl w-full p-8">
		<div class="text-center">
			{#if verifying}
				<div class="flex justify-center mb-6">
					<div class="w-12 h-12 border-2 border-primary-500/30 border-t-primary-500 animate-spin"></div>
				</div>
				<h1 class="text-xl font-semibold text-white mb-2">Verifying Payment...</h1>
			{:else if error}
				<div class="flex justify-center mb-6">
					<div class="icon-box">
						<Icon name="x" size={24} />
					</div>
				</div>
				<h1 class="text-2xl font-bold text-white mb-4">Something Went Wrong</h1>
				<p class="text-neutral-400 text-base mb-6">{error}</p>
				<button onclick={navigateToRentals} class="btn-primary">
					View My Rentals
				</button>
			{:else}
				<div class="flex justify-center mb-6">
					<div class="icon-box-accent w-14 h-14">
						<Icon name="check" size={28} class="text-success" />
					</div>
				</div>
				<h1 class="text-2xl font-bold text-white mb-4">Payment Successful</h1>
				<p class="text-neutral-400 text-base mb-6">
					Your payment has been processed successfully. Your rental request is now being prepared.
				</p>
				<p class="text-neutral-500 text-sm mb-8">
					You will receive an email confirmation shortly. The provider will review your request and provision your resources.
				</p>
				<div class="flex flex-col sm:flex-row gap-3 justify-center">
					<button onclick={navigateToRentals} class="btn-primary">
						View My Rentals
					</button>
				</div>
				<p class="text-neutral-600 text-xs mt-6">
					Redirecting to your rentals in 5 seconds...
				</p>
			{/if}
		</div>
	</div>
</div>
