<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { verifyCheckoutSession } from '$lib/services/api';

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

		// Verify the checkout session with our backend
		// This syncs payment status even if webhook was delayed/failed
		try {
			await verifyCheckoutSession(sessionId);
			verified = true;
		} catch (e) {
			// Verification failed - payment might still be processing
			// Log but don't show error - webhook may still update it
			console.warn('Checkout verification:', e);
			verified = true; // Show success anyway - Stripe redirected here
		}

		verifying = false;

		// Auto-redirect to rentals page after 5 seconds
		setTimeout(() => {
			goto('/dashboard/rentals');
		}, 5000);
	});

	function navigateToRentals() {
		goto('/dashboard/rentals');
	}
</script>

<div class="min-h-screen bg-gradient-to-br from-gray-900 via-blue-900 to-purple-900 flex items-center justify-center p-4">
	<div class="bg-gradient-to-br from-gray-900 to-gray-800 rounded-2xl max-w-2xl w-full border border-white/20 shadow-2xl p-8">
		<div class="text-center">
			{#if verifying}
				<div class="flex justify-center mb-6">
					<div class="w-16 h-16 border-4 border-white border-t-transparent rounded-full animate-spin"></div>
				</div>
				<h1 class="text-2xl font-bold text-white mb-2">Verifying Payment...</h1>
			{:else if error}
				<div class="flex justify-center mb-6">
					<div class="w-16 h-16 bg-red-500/20 rounded-full flex items-center justify-center">
						<svg class="w-10 h-10 text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
						</svg>
					</div>
				</div>
				<h1 class="text-3xl font-bold text-white mb-4">Something Went Wrong</h1>
				<p class="text-white/70 text-lg mb-6">{error}</p>
				<button
					onclick={navigateToRentals}
					class="px-6 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold hover:brightness-110 transition-all text-white"
				>
					View My Rentals
				</button>
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
						onclick={navigateToRentals}
						class="px-6 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold hover:brightness-110 transition-all text-white"
					>
						View My Rentals
					</button>
				</div>
				<p class="text-white/50 text-xs mt-6">
					Redirecting to your rentals in 5 seconds...
				</p>
			{/if}
		</div>
	</div>
</div>
