<script lang="ts">
	import { generateMnemonic, mnemonicToSeedSync } from 'bip39';
	import { hmac } from '@noble/hashes/hmac';
	import { sha512 } from '@noble/hashes/sha2';
	import { Ed25519KeyIdentity } from '@dfinity/identity';
	import { addAccountKey } from '$lib/services/account-api';
	import type { IdentityInfo } from '$lib/stores/auth';
	import { authStore } from '$lib/stores/auth';

	let { open = $bindable(false), account, currentIdentity } = $props<{
		open: boolean;
		account: { username: string };
		currentIdentity: IdentityInfo;
	}>();

	type Step = 'generate' | 'confirm' | 'adding' | 'success' | 'error';

	let step = $state<Step>('generate');
	let seedPhrase = $state('');
	let copySuccess = $state(false);
	let confirmed = $state(false);
	let error = $state('');
	let deviceName = $state('');

	function resetState() {
		step = 'generate';
		seedPhrase = '';
		copySuccess = false;
		confirmed = false;
		error = '';
		deviceName = '';
	}

	function handleOpen() {
		seedPhrase = generateMnemonic();
		step = 'generate';
	}

	$effect(() => {
		if (open && !seedPhrase) {
			handleOpen();
		}
	});

	async function copyToClipboard() {
		try {
			await navigator.clipboard.writeText(seedPhrase);
			copySuccess = true;
			setTimeout(() => (copySuccess = false), 2000);
		} catch (err) {
			console.error('Failed to copy:', err);
		}
	}

	function identityFromSeed(seed: string): Ed25519KeyIdentity {
		const seedBuffer = mnemonicToSeedSync(seed, '');
		const seedBytes = new Uint8Array(seedBuffer);
		const keyMaterial = hmac(sha512, 'ed25519 seed', seedBytes);
		const derivedSeed = keyMaterial.slice(0, 32);
		return Ed25519KeyIdentity.fromSecretKey(derivedSeed);
	}

	function bytesToHex(bytes: Uint8Array): string {
		return Array.from(bytes)
			.map((b) => b.toString(16).padStart(2, '0'))
			.join('');
	}

	async function handleAddDevice() {
		if (!confirmed) {
			error = 'Please confirm you have saved the seed phrase';
			return;
		}

		step = 'adding';
		error = '';

		try {
			const newIdentity = identityFromSeed(seedPhrase);
			const newPublicKeyBytes = new Uint8Array(newIdentity.getPublicKey().rawKey);
			const newPublicKeyHex = bytesToHex(newPublicKeyBytes);

			await addAccountKey(
				currentIdentity.identity as Ed25519KeyIdentity,
				account.username,
				newPublicKeyHex
			);

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
		class="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center p-4"
		onclick={handleClose}
		onkeydown={(e) => e.key === 'Escape' && handleClose()}
		role="button"
		tabindex="0"
	>
		<div
			class="bg-gray-900 rounded-2xl p-8 max-w-2xl w-full border border-white/20 shadow-2xl"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			role="dialog"
			tabindex="-1"
		>
			{#if step === 'generate'}
				<h2 class="text-2xl font-bold text-white mb-2">Add New Device</h2>
				<p class="text-white/60 mb-6">
					Generate a new seed phrase for your other device. Use this phrase to sign in on that device.
				</p>

				<div class="bg-yellow-500/10 border border-yellow-500/30 rounded-lg p-4 mb-6">
					<p class="text-yellow-400 text-sm">
						<strong>Write down these words</strong> and store them safely. You'll need them to sign in on your new device.
					</p>
				</div>

				<div class="bg-gray-800 border border-white/20 rounded-xl p-6 mb-4">
					<div class="grid grid-cols-3 gap-3 mb-4">
						{#each seedPhrase.split(' ') as word, i}
							<div class="flex items-center gap-2 text-white">
								<span class="text-white/40 text-sm w-5">{i + 1}.</span>
								<span class="font-mono font-semibold">{word}</span>
							</div>
						{/each}
					</div>
				</div>

				<button
					type="button"
					onclick={copyToClipboard}
					class="w-full mb-6 px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white hover:bg-white/20 transition-colors"
				>
					{#if copySuccess}
						<span class="text-green-400">Copied!</span>
					{:else}
						Copy to Clipboard
					{/if}
				</button>

				<div class="mb-4">
					<label for="device-name" class="block text-sm text-white/70 mb-2">Device name (optional)</label>
					<input
						id="device-name"
						type="text"
						bind:value={deviceName}
						placeholder="e.g., Laptop, Phone"
						class="w-full px-4 py-2 bg-white/5 border border-white/20 rounded-lg text-white placeholder:text-white/30"
					/>
				</div>

				<label class="flex items-center gap-3 mb-6 cursor-pointer">
					<input
						type="checkbox"
						bind:checked={confirmed}
						class="w-5 h-5 rounded border-white/20 bg-white/5"
					/>
					<span class="text-white/80 text-sm">I have saved this seed phrase securely</span>
				</label>

				{#if error}
					<div class="mb-4 p-3 bg-red-500/20 border border-red-500/30 rounded-lg text-red-400 text-sm">
						{error}
					</div>
				{/if}

				<div class="flex gap-3">
					<button
						type="button"
						onclick={handleClose}
						class="flex-1 px-4 py-3 bg-white/10 hover:bg-white/20 rounded-lg text-white transition-colors"
					>
						Cancel
					</button>
					<button
						type="button"
						onclick={handleAddDevice}
						disabled={!confirmed}
						class="flex-1 px-4 py-3 bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-500 hover:to-pink-500 rounded-lg text-white font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed"
					>
						Add Device
					</button>
				</div>

			{:else if step === 'adding'}
				<div class="text-center py-8">
					<div class="text-5xl animate-pulse mb-4">🔗</div>
					<h2 class="text-2xl font-bold text-white mb-2">Linking Device</h2>
					<p class="text-white/60">Adding new device to your account...</p>
				</div>

			{:else if step === 'success'}
				<div class="text-center py-8">
					<div class="text-5xl mb-4">✅</div>
					<h2 class="text-2xl font-bold text-white mb-2">Device Added!</h2>
					<p class="text-white/60 mb-6">
						You can now use the seed phrase to sign in on your new device.
					</p>
					<button
						type="button"
						onclick={handleClose}
						class="px-8 py-3 bg-gradient-to-r from-purple-600 to-pink-600 rounded-lg text-white font-medium"
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
							class="px-6 py-3 bg-white/10 hover:bg-white/20 rounded-lg text-white transition-colors"
						>
							Cancel
						</button>
						<button
							type="button"
							onclick={() => (step = 'generate')}
							class="px-6 py-3 bg-gradient-to-r from-purple-600 to-pink-600 rounded-lg text-white font-medium"
						>
							Try Again
						</button>
					</div>
				</div>
			{/if}
		</div>
	</div>
{/if}
