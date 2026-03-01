<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { authStore } from "$lib/stores/auth";
	import SettingsTabs from "$lib/components/SettingsTabs.svelte";
	import AuthRequiredCard from "$lib/components/AuthRequiredCard.svelte";
	import UserProfileEditor from "$lib/components/UserProfileEditor.svelte";
	import AccountEmailEditor from "$lib/components/AccountEmailEditor.svelte";
	import type { IdentityInfo } from "$lib/stores/auth";
	import { computePubkey } from "$lib/utils/contract-format";

	let currentIdentity = $state<IdentityInfo | null>(null);
	let isAuthenticated = $state(false);
	let accountEmail = $state<string>('');
	let unsubscribe: (() => void) | null = null;
	let unsubscribeAuth: (() => void) | null = null;

	onMount(() => {
		unsubscribeAuth = authStore.isAuthenticated.subscribe((isAuth) => {
			isAuthenticated = isAuth;
		});

		unsubscribe = authStore.activeIdentity.subscribe((value) => {
			currentIdentity = value;
			accountEmail = value?.account?.email || '';
		});
	});

	function handleEmailUpdated(newEmail: string) {
		accountEmail = newEmail;
	}

	onDestroy(() => {
		unsubscribe?.();
		unsubscribeAuth?.();
	});
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-2xl font-bold text-white tracking-tight">Public Profile</h1>
		<p class="text-neutral-500">
			Information visible to other users
		</p>
	</div>

	<SettingsTabs />

	{#if !isAuthenticated}
		<AuthRequiredCard subtext="Create an account or login to view and edit your public profile information visible to other users." />
	{:else if currentIdentity}
		<div
			class="card p-6 border border-neutral-800"
		>
			<h3 class="text-xl font-semibold text-white mb-4">
				Current Identity
			</h3>
			<div class="space-y-2 text-sm">
				<p class="text-neutral-400">
					Type: <span class="text-white">Seed Phrase</span>
				</p>
				<p class="text-neutral-400 font-mono">
					Principal: <span class="text-white text-xs"
						>{currentIdentity.principal.toString()}</span
					>
				</p>
				{#if currentIdentity.publicKeyBytes}
					<p class="text-neutral-400 font-mono">
						Public key (hex): <span class="text-white text-xs"
							>{computePubkey(
								currentIdentity.publicKeyBytes,
							)}</span
						>
					</p>
				{/if}
			</div>
		</div>

		<!-- Account Email Section -->
		<div class="card p-6 border border-neutral-800">
			<h3 class="text-xl font-semibold text-white mb-4">Account Email</h3>
			<AccountEmailEditor
				email={accountEmail}
				username={currentIdentity.account?.username || ''}
				onEmailUpdated={handleEmailUpdated}
			/>
		</div>
	{/if}

	{#if currentIdentity}
		<UserProfileEditor identity={currentIdentity} signingIdentity={currentIdentity} />
	{:else}
		<p class="text-neutral-500">Loading...</p>
	{/if}
</div>
