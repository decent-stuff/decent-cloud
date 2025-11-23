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
		| 'choose-mode'
		| 'seed'
		| 'checking-account'
		| 'enter-username'
		| 'processing'
		| 'success';

	let currentStep = $state<Step>('choose-mode');
	let seedPhrase = $state('');
	let username = $state('');
	let usernameValid = $state(false);
	let normalizedUsername = $state('');
	let error = $state<string | null>(null);
	let createdAccount = $state<AccountInfo | null>(null);
	let isNewAccount = $state(false);

	function chooseMode(isNew: boolean) {
		isNewAccount = isNew;
		currentStep = 'seed';
	}

	async function handleSeedComplete(seed: string, deviceName?: string) {
		seedPhrase = seed;
		// Note: deviceName is not used in account creation flow

		if (isNewAccount) {
			// New account - proceed to username entry
			currentStep = 'enter-username';
		} else {
			// Existing account - check if it exists
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
					// No account found - need to register
					error =
						'No account found with this seed phrase. Please check your seed phrase or create a new account.';
					currentStep = 'seed';
				}
			} catch (err) {
				console.error('Account check error:', err);
				error = err instanceof Error ? err.message : 'Failed to check account';
				currentStep = 'seed';
			}
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
		if (currentStep === 'seed') {
			currentStep = 'choose-mode';
			seedPhrase = '';
		} else if (currentStep === 'enter-username') {
			currentStep = 'seed';
		}
	}
</script>

<div class="space-y-6">
	<!-- Step 1: Choose Mode -->
	{#if currentStep === 'choose-mode'}
		<div class="space-y-4">
			<h3 class="text-2xl font-bold text-white">Welcome to Decent Cloud</h3>
			<p class="text-white/60">Sign in or create a new account</p>

			<div class="grid gap-4">
				<button
					type="button"
					onclick={() => chooseMode(true)}
					class="p-6 bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-500 hover:to-pink-500 rounded-xl text-left transition-all group"
				>
					<div class="text-3xl mb-2">✨</div>
					<h4 class="text-xl font-bold text-white mb-1">Create New Account</h4>
					<p class="text-white/80 text-sm">
						Generate a new seed phrase and choose your username
					</p>
				</button>

				<button
					type="button"
					onclick={() => chooseMode(false)}
					class="p-6 bg-white/5 hover:bg-white/10 border border-white/20 rounded-xl text-left transition-all group"
				>
					<div class="text-3xl mb-2">🔑</div>
					<h4 class="text-xl font-bold text-white mb-1">Sign In</h4>
					<p class="text-white/60 text-sm">Use your existing seed phrase to sign in</p>
				</button>
			</div>
		</div>
	{/if}

	<!-- Step 2: Seed Phrase (Generate or Import) -->
	{#if currentStep === 'seed'}
		<SeedPhraseStep
			initialMode={isNewAccount ? 'choose' : 'import'}
			showModeChoice={isNewAccount}
			onComplete={handleSeedComplete}
			onBack={goBack}
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
				{#if isNewAccount}
					This will be your unique identifier on Decent Cloud
				{:else}
					Enter your username to complete registration
				{/if}
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
					{isNewAccount ? 'Create Account' : 'Register & Sign In'}
				</button>
			</div>
		</div>
	{/if}

	<!-- Step 6: Processing -->
	{#if currentStep === 'processing'}
		<div class="space-y-4 text-center py-8">
			<div class="text-6xl animate-bounce">🔐</div>
			<h3 class="text-2xl font-bold text-white">
				{isNewAccount ? 'Creating Your Account' : 'Signing You In'}
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
			<div class="text-6xl">{isNewAccount ? '🎉' : '👋'}</div>
			<h3 class="text-2xl font-bold text-white">
				{isNewAccount ? 'Account Created!' : 'Welcome Back!'}
			</h3>
			<p class="text-white/60">
				{#if isNewAccount}
					Welcome to Decent Cloud,
				{:else}
					Signed in as
				{/if}
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
