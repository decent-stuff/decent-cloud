<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { authStore } from '$lib/stores/auth';
	import UserProfileEditor from '$lib/components/UserProfileEditor.svelte';
	import type { IdentityInfo } from '$lib/stores/auth';

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
			await authStore.loginWithSeedPhrase(undefined, '/dashboard/profile');
		} catch (err) {
			console.error('Failed to create signing key:', err);
		}
	}
</script>

<div class="p-8 max-w-4xl mx-auto">
	<h1 class="text-2xl font-bold mb-6">Profile Settings</h1>

	{#if !signingIdentity}
		<div class="bg-yellow-50 border border-yellow-200 rounded-lg p-6">
			<p class="text-yellow-800 mb-4">
				You need a signing key (seed phrase identity) to edit your profile.
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
		<p class="text-gray-500">Loading...</p>
	{/if}
</div>
