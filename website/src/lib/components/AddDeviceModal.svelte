<script lang="ts">
	import { Ed25519KeyIdentity } from '@dfinity/identity';
	import { addAccountKey, updateDeviceName } from '$lib/services/account-api';
	import type { IdentityInfo } from '$lib/stores/auth';
	import { authStore } from '$lib/stores/auth';
	import { identityFromSeed, bytesToHex } from '$lib/utils/identity';
	import SeedPhraseStep from './SeedPhraseStep.svelte';

	let { open = $bindable(false), account, currentIdentity } = $props<{
		open: boolean;
		account: { username: string };
		currentIdentity: IdentityInfo;
	}>();

	type Step = 'seed' | 'adding' | 'success' | 'error';

	let step = $state<Step>('seed');
	let seedPhrase = $state('');
	let deviceName = $state<string | undefined>(undefined);
	let error = $state('');

	function resetState() {
		step = 'seed';
		seedPhrase = '';
		deviceName = undefined;
		error = '';
	}

	function handleSeedComplete(seed: string, name?: string) {
		seedPhrase = seed;
		deviceName = name;
		handleAddDevice();
	}

	async function handleAddDevice() {
		step = 'adding';
		error = '';

		try {
			const newIdentity = identityFromSeed(seedPhrase);
			const newPublicKeyBytes = new Uint8Array(newIdentity.getPublicKey().rawKey);
			const newPublicKeyHex = bytesToHex(newPublicKeyBytes);

			const addedKey = await addAccountKey(
				currentIdentity.identity as Ed25519KeyIdentity,
				account.username,
				newPublicKeyHex
			);

			// Set device name if provided
			if (deviceName) {
				await updateDeviceName(
					currentIdentity.identity as Ed25519KeyIdentity,
					account.username,
					addedKey.id,
					deviceName
				);
			}

			// Reload account to get updated keys
			await authStore.loadAccountByUsername(account.username);
			step = 'success';
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to add device';
			step = 'error';
		}
	}

	function handleClose() {
		open = false;
		resetState();
	}
</script>

{#if open}
	<div
		class="fixed inset-0 bg-base/80 backdrop-blur-sm z-50 flex items-center justify-center p-4"
		onclick={handleClose}
		onkeydown={(e) => e.key === 'Escape' && handleClose()}
		role="button"
		tabindex="0"
	>
		<div
			class="bg-surface  p-8 max-w-2xl w-full border border-neutral-800 shadow-2xl"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			role="dialog"
			tabindex="-1"
		>
			{#if step === 'seed'}
				<SeedPhraseStep
					initialMode="choose"
					showModeChoice={true}
					showDeviceName={true}
					onComplete={handleSeedComplete}
					onBack={handleClose}
				/>

			{:else if step === 'adding'}
				<div class="text-center py-8">
					<div class="text-5xl animate-pulse mb-4">🔗</div>
					<h2 class="text-2xl font-bold text-white mb-2">Linking Device</h2>
					<p class="text-neutral-500">Adding new device to your account...</p>
				</div>

			{:else if step === 'success'}
				<div class="text-center py-8">
					<div class="text-5xl mb-4">✅</div>
					<h2 class="text-2xl font-bold text-white mb-2">Device Added!</h2>
					<p class="text-neutral-500 mb-6">
						You can now use the seed phrase to sign in on your new device.
					</p>
					<button
						type="button"
						onclick={handleClose}
						class="px-8 py-3 bg-gradient-to-r from-primary-500 to-primary-600  text-white font-medium"
					>
						Done
					</button>
				</div>

			{:else if step === 'error'}
				<div class="text-center py-8">
					<div class="text-5xl mb-4">❌</div>
					<h2 class="text-2xl font-bold text-white mb-2">Failed to Add Device</h2>
					<p class="text-red-400 mb-6">{error}</p>
					<div class="flex gap-3 justify-center">
						<button
							type="button"
							onclick={handleClose}
							class="px-6 py-3 bg-surface-elevated hover:bg-surface-elevated  text-white transition-colors"
						>
							Cancel
						</button>
						<button
							type="button"
							onclick={() => (step = 'seed')}
							class="px-6 py-3 bg-gradient-to-r from-primary-500 to-primary-600  text-white font-medium"
						>
							Try Again
						</button>
					</div>
				</div>
			{/if}
		</div>
	</div>
{/if}
