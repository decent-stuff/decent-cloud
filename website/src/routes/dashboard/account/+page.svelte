<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { authStore } from "$lib/stores/auth";
	import AccountOverview from "$lib/components/AccountOverview.svelte";
	import type { IdentityInfo } from "$lib/stores/auth";

	let currentIdentity = $state<IdentityInfo | null>(null);
	let unsubscribe: (() => void) | null = null;

	onMount(() => {
		unsubscribe = authStore.currentIdentity.subscribe((value) => {
			currentIdentity = value;
		});
	});

	onDestroy(() => {
		unsubscribe?.();
	});
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">Account Settings</h1>
		<p class="text-white/60">
			Manage your account details and authentication keys
		</p>
	</div>

	{#if currentIdentity?.account}
		<AccountOverview account={currentIdentity.account} />
	{:else if currentIdentity}
		<div
			class="bg-yellow-500/20 border border-yellow-500/30 rounded-xl p-6 backdrop-blur-lg"
		>
			<p class="text-yellow-300 mb-2 font-medium">No Account Found</p>
			<p class="text-white/70 text-sm">
				You are signed in but don't have a registered account yet.
				<a href="/" class="text-blue-400 hover:text-blue-300 underline">
					Register an account
				</a>
				to access account settings.
			</p>
		</div>
	{:else}
		<p class="text-white/60">Loading...</p>
	{/if}
</div>
