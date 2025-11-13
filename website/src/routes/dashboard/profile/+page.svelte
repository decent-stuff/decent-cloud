<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { authStore } from "$lib/stores/auth";
	import UserProfileEditor from "$lib/components/UserProfileEditor.svelte";
	import type { IdentityInfo } from "$lib/stores/auth";
	import { computePubkeyHash } from "$lib/utils/contract-format";

	let currentIdentity = $state<IdentityInfo | null>(null);
	let signingIdentity = $state<IdentityInfo | null>(null);
	let unsubscribeCurrent: (() => void) | null = null;
	let unsubscribeSigning: (() => void) | null = null;

	onMount(() => {
		unsubscribeCurrent = authStore.currentIdentity.subscribe((value) => {
			currentIdentity = value;
		});
		unsubscribeSigning = authStore.signingIdentity.subscribe((value) => {
			signingIdentity = value;
		});
	});

	onDestroy(() => {
		unsubscribeCurrent?.();
		unsubscribeSigning?.();
	});

	async function createSigningKey() {
		try {
			await authStore.loginWithSeedPhrase(
				undefined,
				"/dashboard/profile",
			);
		} catch (err) {
			console.error("Failed to create signing key:", err);
		}
	}
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">Profile Settings</h1>
		<p class="text-white/60">
			Manage your account information and preferences
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
					Type: <span class="text-white"
						>{currentIdentity.type === "ii"
							? "Internet Identity"
							: "Seed Phrase"}</span
					>
				</p>
				<p class="text-white/70 font-mono">
					Principal: <span class="text-white text-xs"
						>{currentIdentity.principal.toString()}</span
					>
				</p>
				{#if currentIdentity.publicKeyBytes}
					<p class="text-white/70 font-mono">
						Public key (hex): <span class="text-white text-xs"
							>{computePubkeyHash(
								currentIdentity.publicKeyBytes,
							)}</span
						>
					</p>
				{/if}
			</div>
		</div>
	{/if}

	{#if !signingIdentity}
		<div
			class="bg-yellow-500/20 border border-yellow-500/30 rounded-xl p-6 backdrop-blur-lg"
		>
			<p class="text-yellow-300 mb-4">
				You need a signing key (seed phrase identity) to edit your
				profile.
			</p>
			<button
				onclick={createSigningKey}
				class="px-6 py-2 bg-yellow-600 text-white rounded-lg hover:bg-yellow-700 transition-colors"
			>
				Create Signing Key
			</button>
		</div>
	{:else if currentIdentity}
		<UserProfileEditor identity={currentIdentity} {signingIdentity} />
	{:else}
		<p class="text-white/60">Loading...</p>
	{/if}
</div>
