<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { page } from "$app/stores";
	import { goto } from "$app/navigation";
	import { authStore } from "$lib/stores/auth";
	import AccountOverview from "$lib/components/AccountOverview.svelte";
	import type { IdentityInfo } from "$lib/stores/auth";

	let currentIdentity = $state<IdentityInfo | null>(null);
	let isAuthenticated = $state(false);
	let unsubscribe: (() => void) | null = null;
	let unsubscribeAuth: (() => void) | null = null;

	onMount(() => {
		unsubscribeAuth = authStore.isAuthenticated.subscribe((isAuth) => {
			isAuthenticated = isAuth;
		});

		unsubscribe = authStore.currentIdentity.subscribe((value) => {
			currentIdentity = value;
		});
	});

	function handleLogin() {
		goto(`/login?returnUrl=${$page.url.pathname}`);
	}

	onDestroy(() => {
		unsubscribe?.();
		unsubscribeAuth?.();
	});
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">Security</h1>
		<p class="text-white/60">
			Manage your account credentials and device access
		</p>
	</div>

	{#if !isAuthenticated}
		<!-- Anonymous user view - login prompt -->
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-8 border border-white/20 text-center">
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">🔐</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-white/70">
					Create an account or login to manage your security settings, view active devices, and control access keys.
				</p>
				<button
					onclick={handleLogin}
					class="px-8 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 hover:scale-105 transition-all"
				>
					Login / Create Account
				</button>
			</div>
		</div>
	{:else if currentIdentity?.account}
		<AccountOverview account={currentIdentity.account} />
	{:else}
		<p class="text-white/60">Loading...</p>
	{/if}
</div>
