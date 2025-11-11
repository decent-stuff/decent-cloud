<script lang="ts">
	import { authStore } from '$lib/stores/auth';
	import AuthDialog from './AuthDialog.svelte';
	import { onMount, onDestroy } from 'svelte';

	let isAuthenticated = $state(false);
	let showAuthDialog = $state(false);
	let unsubscribe: (() => void) | null = null;

	onMount(() => {
		unsubscribe = authStore.isAuthenticated.subscribe((value) => {
			isAuthenticated = value;
		});
	});

	onDestroy(() => {
		unsubscribe?.();
	});

	function handleConnect() {
		showAuthDialog = true;
	}
</script>

<header
	class="fixed top-0 left-0 right-0 z-50 bg-black/20 backdrop-blur-md border-b border-white/10"
>
	<div class="max-w-7xl mx-auto px-6 py-4 flex items-center justify-between">
		<!-- Logo -->
		<a href="/" class="text-2xl font-bold text-white hover:text-blue-400 transition-colors">
			Decent Cloud
		</a>

		<!-- Actions -->
		<div class="flex items-center gap-4">
			{#if isAuthenticated}
				<a
					href="/dashboard"
					class="px-6 py-2 bg-gradient-to-r from-blue-500 to-purple-600 rounded-full font-semibold hover:brightness-110 hover:scale-105 transition-all"
				>
					Dashboard →
				</a>
			{:else}
				<button
					type="button"
					onclick={handleConnect}
					class="px-6 py-2 bg-gradient-to-r from-blue-500 to-purple-600 rounded-full font-semibold hover:brightness-110 hover:scale-105 transition-all"
				>
					Connect Wallet
				</button>
			{/if}
		</div>
	</div>
</header>

<AuthDialog bind:open={showAuthDialog} />
