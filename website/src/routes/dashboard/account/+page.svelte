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
		<h1 class="text-4xl font-bold text-white mb-2">Account & Security</h1>
		<p class="text-white/60">
			Manage your account credentials and device access
		</p>
	</div>

	{#if currentIdentity?.account}
		<AccountOverview account={currentIdentity.account} />
	{:else}
		<p class="text-white/60">Loading...</p>
	{/if}
</div>
