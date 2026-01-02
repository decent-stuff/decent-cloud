<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { page } from "$app/stores";
	import { authStore } from "$lib/stores/auth";
	import { navigateToLogin } from "$lib/utils/navigation";
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
		navigateToLogin($page.url.pathname);
	}

	onDestroy(() => {
		unsubscribe?.();
		unsubscribeAuth?.();
	});
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-2xl font-bold text-white tracking-tight">Security</h1>
		<p class="text-neutral-500">
			Manage your account credentials and device access
		</p>
	</div>

	{#if !isAuthenticated}
		<!-- Anonymous user view - login prompt -->
		<div class="card p-8 border border-neutral-800 text-center">
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">🔐</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-neutral-400">
					Create an account or login to manage your security settings, view active devices, and control access keys.
				</p>
				<button
					onclick={handleLogin}
					class="px-8 py-3 bg-gradient-to-r from-primary-500 to-primary-600  font-semibold text-white hover:brightness-110 hover:scale-105 transition-all"
				>
					Login / Create Account
				</button>
			</div>
		</div>
	{:else if currentIdentity?.account}
		<AccountOverview account={currentIdentity.account} />
	{:else}
		<p class="text-neutral-500">Loading...</p>
	{/if}
</div>
