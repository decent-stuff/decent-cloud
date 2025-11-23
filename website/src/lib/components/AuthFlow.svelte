<script lang="ts">
	import { authStore, type AccountInfo } from '$lib/stores/auth';
	import { getAccountByPublicKey } from '$lib/services/account-api';
	import { identityFromSeed, bytesToHex } from '$lib/utils/identity';
	import UsernameInput from './UsernameInput.svelte';
	import SeedPhraseStep from './SeedPhraseStep.svelte';

	let { onSuccess } = $props<{
		onSuccess: (account: AccountInfo) => void;
	}>();

	type Step =
		| 'seed'
		| 'checking-account'
		| 'enter-username'
		| 'processing'
		| 'success';

	let currentStep = $state<Step>('seed');
	let seedPhrase = $state('');
	let username = $state('');
	let usernameValid = $state(false);
	let normalizedUsername = $state('');
	let error = $state<string | null>(null);
	let createdAccount = $state<AccountInfo | null>(null);

	async function handleSeedComplete(seed: string, deviceName?: string) {
		seedPhrase = seed;
		// Note: deviceName is not used in account creation flow

		// Check if account exists for this seed
		currentStep = 'checking-account';
		error = null;

		try {
			const identity = identityFromSeed(seedPhrase);
			const publicKeyBytes = new Uint8Array(identity.getPublicKey().rawKey);
			const publicKeyHex = bytesToHex(publicKeyBytes);

			// Check if account exists
			const account = await getAccountByPublicKey(publicKeyHex);

			if (account) {
				// Existing account found - login directly
				await loginWithExistingAccount(account);
			} else {
				// No account found - proceed to username entry for new account
				currentStep = 'enter-username';
			}
		} catch (err) {
			console.error('Account check error:', err);
			error = err instanceof Error ? err.message : 'Failed to check account';
			currentStep = 'seed';
		}
	}

	async function loginWithExistingAccount(account: AccountInfo) {
		currentStep = 'processing';
		try {
			await authStore.loginWithSeedPhrase(seedPhrase, '/dashboard');
			createdAccount = account;
			currentStep = 'success';
		} catch (err) {
			error = err instanceof Error ? err.message : 'Login failed';
			currentStep = 'seed';
		}
	}

	function handleUsernameValidChange(valid: boolean, normalized: string) {
		usernameValid = valid;
		normalizedUsername = normalized;
	}

	async function registerAndLogin() {
		if (!usernameValid) {
			error = 'Please enter a valid username';
			return;
		}

		currentStep = 'processing';
		error = null;

		try {
			const identity = identityFromSeed(seedPhrase);

			// Register account (this also sets up the identity with account data and persists seed phrase)
			const account = await authStore.registerNewAccount(
				identity,
				normalizedUsername,
				seedPhrase
			);

			createdAccount = account;
			currentStep = 'success';

			// Navigate to dashboard - account is already set and persisted in auth store
			if (typeof window !== 'undefined') {
				window.location.href = '/dashboard';
			}
		} catch (err) {
			error = err instanceof Error ? err.message : 'Registration failed';
			currentStep = 'enter-username';
		}
	}

	function handleSuccess() {
		if (createdAccount) {
			onSuccess(createdAccount);
		}
	}

	function goBack() {
		if (currentStep === 'enter-username') {
			currentStep = 'seed';
			seedPhrase = '';
		}
	}
</script>

<div class="space-y-6">
	<!-- Step 1: Seed Phrase (Generate or Import) -->
	{#if currentStep === 'seed'}
		<SeedPhraseStep
			initialMode="choose"
			showModeChoice={true}
			onComplete={handleSeedComplete}
		/>
	{/if}

	<!-- Step 4: Checking Account -->
	{#if currentStep === 'checking-account'}
		<div class="space-y-4 text-center py-8">
			<div class="text-6xl animate-pulse">🔍</div>
			<h3 class="text-2xl font-bold text-white">Detecting Account</h3>
			<p class="text-white/60">Looking up your account...</p>
			<div class="flex justify-center">
				<div
					class="w-8 h-8 border-4 border-purple-500/30 border-t-purple-500 rounded-full animate-spin"
				></div>
			</div>
		</div>
	{/if}

	<!-- Step 5: Enter Username -->
	{#if currentStep === 'enter-username'}
		<div class="space-y-4">
			<h3 class="text-2xl font-bold text-white">Choose Your Username</h3>
			<p class="text-white/60">
				This will be your unique identifier on Decent Cloud
			</p>

			<UsernameInput
				bind:value={username}
				onValidChange={handleUsernameValidChange}
			/>

			{#if error}
				<div
					class="p-4 bg-red-500/20 border border-red-500/30 rounded-lg text-red-400 text-sm"
				>
					{error}
				</div>
			{/if}

			<div class="flex gap-3">
				<button
					type="button"
					onclick={goBack}
					class="flex-1 px-4 py-3 bg-white/10 hover:bg-white/20 rounded-lg text-white transition-colors"
				>
					Back
				</button>
				<button
					type="button"
					onclick={registerAndLogin}
					class="flex-1 px-4 py-3 bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-500 hover:to-pink-500 rounded-lg text-white font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed"
					disabled={!usernameValid}
				>
					Create Account
				</button>
			</div>
		</div>
	{/if}

	<!-- Step 6: Processing -->
	{#if currentStep === 'processing'}
		<div class="space-y-4 text-center py-8">
			<div class="text-6xl animate-bounce">🔐</div>
			<h3 class="text-2xl font-bold text-white">
				{createdAccount ? 'Signing You In' : 'Creating Your Account'}
			</h3>
			<p class="text-white/60">Please wait...</p>
			<div class="flex justify-center">
				<div
					class="w-8 h-8 border-4 border-purple-500/30 border-t-purple-500 rounded-full animate-spin"
				></div>
			</div>
		</div>
	{/if}

	<!-- Step 7: Success -->
	{#if currentStep === 'success' && createdAccount}
		<div class="space-y-4 text-center py-8">
			<div class="text-6xl">👋</div>
			<h3 class="text-2xl font-bold text-white">
				Welcome to Decent Cloud!
			</h3>
			<p class="text-white/60">
				Signed in as
				<span class="text-white font-medium">@{createdAccount.username}</span>
			</p>

			<div class="pt-4">
				<button
					type="button"
					onclick={handleSuccess}
					class="px-8 py-3 bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-500 hover:to-pink-500 rounded-lg text-white font-medium transition-all"
				>
					Go to Dashboard
				</button>
			</div>
		</div>
	{/if}
</div>
