<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { authStore } from '$lib/stores/auth';
	import { computePubkey } from '$lib/utils/contract-format';
	import { navigateToLogin } from '$lib/utils/navigation';

	let isAuthenticated = $state(false);
	let unsubscribe: (() => void) | null = null;

	onMount(async () => {
		unsubscribe = authStore.isAuthenticated.subscribe(async (isAuth) => {
			isAuthenticated = isAuth;

			// If authenticated, redirect to user's own reputation page
			if (isAuth) {
				const identity = await authStore.getSigningIdentity();
				if (identity?.publicKeyBytes) {
					const pubkey = computePubkey(identity.publicKeyBytes);
					goto(`/dashboard/reputation/${pubkey}`);
				}
			}
		});
	});

	onDestroy(() => {
		unsubscribe?.();
	});

	function handleLogin() {
		navigateToLogin($page.url.pathname);
	}
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">My Reputation</h1>
		<p class="text-white/60">
			View your reputation score, activity history, and token balance
		</p>
	</div>

	{#if !isAuthenticated}
		<!-- Anonymous user view - login prompt -->
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-8 border border-white/20 text-center">
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">⭐</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-white/70">
					Create an account or login to view your reputation score, activity history, token balance, and public profile.
				</p>
				<button
					onclick={handleLogin}
					class="px-8 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 hover:scale-105 transition-all"
				>
					Login / Create Account
				</button>
			</div>
		</div>
	{:else}
		<!-- Loading state while redirecting -->
		<div class="flex justify-center items-center p-8">
			<div
				class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-400"
			></div>
		</div>
	{/if}
</div>
