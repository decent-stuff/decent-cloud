<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { authStore } from "$lib/stores/auth";
	import UserProfileEditor from "$lib/components/UserProfileEditor.svelte";
	import type { IdentityInfo } from "$lib/stores/auth";
	import { computePubkey } from "$lib/utils/contract-format";

	let currentIdentity = $state<IdentityInfo | null>(null);
	let unsubscribe: (() => void) | null = null;

	onMount(() => {
		unsubscribe = authStore.activeIdentity.subscribe((value) => {
			currentIdentity = value;
		});
	});

	onDestroy(() => {
		unsubscribe?.();
	});
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">Public Profile</h1>
		<p class="text-white/60">
			Information visible to other users
		</p>
	</div>

	{#if currentIdentity}
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
