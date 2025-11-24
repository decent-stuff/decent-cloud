<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { page } from "$app/stores";
	import { goto } from "$app/navigation";
	import { authStore } from "$lib/stores/auth";
	import UserProfileEditor from "$lib/components/UserProfileEditor.svelte";
	import type { IdentityInfo } from "$lib/stores/auth";
	import { computePubkey } from "$lib/utils/contract-format";

	let currentIdentity = $state<IdentityInfo | null>(null);
	let isAuthenticated = $state(false);
	let unsubscribe: (() => void) | null = null;
	let unsubscribeAuth: (() => void) | null = null;

	onMount(() => {
		unsubscribeAuth = authStore.isAuthenticated.subscribe((isAuth) => {
			isAuthenticated = isAuth;
		});

		unsubscribe = authStore.activeIdentity.subscribe((value) => {
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
		<h1 class="text-4xl font-bold text-white mb-2">Public Profile</h1>
		<p class="text-white/60">
			Information visible to other users
		</p>
	</div>

	{#if !isAuthenticated}
		<!-- Anonymous user view - login prompt -->
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-8 border border-white/20 text-center">
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">👤</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-white/70">
					Create an account or login to view and edit your public profile information visible to other users.
				</p>
				<button
					onclick={handleLogin}
					class="px-8 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 hover:scale-105 transition-all"
				>
					Login / Create Account
				</button>
			</div>
		</div>
	{:else if currentIdentity}
		<div
			class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20"
		>
			<h3 class="text-xl font-semibold text-white mb-4">
				Current Identity
			</h3>
			<div class="space-y-2 text-sm">
				<p class="text-white/70">
					Type: <span class="text-white">Seed Phrase</span>
				</p>
				<p class="text-white/70 font-mono">
					Principal: <span class="text-white text-xs"
						>{currentIdentity.principal.toString()}</span
					>
				</p>
				{#if currentIdentity.publicKeyBytes}
					<p class="text-white/70 font-mono">
						Public key (hex): <span class="text-white text-xs"
							>{computePubkey(
								currentIdentity.publicKeyBytes,
							)}</span
						>
					</p>
				{/if}
			</div>
		</div>
	{/if}

	{#if currentIdentity}
		<UserProfileEditor identity={currentIdentity} signingIdentity={currentIdentity} />
	{:else}
		<p class="text-white/60">Loading...</p>
	{/if}
</div>
