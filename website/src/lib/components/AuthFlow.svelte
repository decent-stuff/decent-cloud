<script lang="ts">
	import { authStore, type AccountInfo } from '$lib/stores/auth';
	import { Ed25519KeyIdentity } from '@dfinity/identity';
	import { validateMnemonic, mnemonicToSeedSync, generateMnemonic } from 'bip39';
	import { hmac } from '@noble/hashes/hmac';
	import { sha512 } from '@noble/hashes/sha512';
	import { getAccountByPublicKey } from '$lib/services/account-api';
	import UsernameInput from './UsernameInput.svelte';

	let { onSuccess } = $props<{
		onSuccess: (account: AccountInfo) => void;
	}>();

	type Mode = 'new' | 'existing';
	type Step =
		| 'choose-mode'
		| 'enter-seed'
		| 'backup-seed'
		| 'checking-account'
		| 'enter-username'
		| 'processing'
		| 'success';

	let mode = $state<Mode | null>(null);
	let currentStep = $state<Step>('choose-mode');
	let seedPhrase = $state('');
	let showSeedEntry = $state(false);
	let seedBackedUp = $state(false);
	let username = $state('');
	let usernameValid = $state(false);
	let normalizedUsername = $state('');
	let error = $state<string | null>(null);
	let createdAccount = $state<AccountInfo | null>(null);
	let isNewAccount = $state(false);

	function identityFromSeed(seedPhrase: string): Ed25519KeyIdentity {
		const seedBuffer = mnemonicToSeedSync(seedPhrase, '');
		const seedBytes = new Uint8Array(seedBuffer);
		const keyMaterial = hmac(sha512, 'ed25519 seed', seedBytes);
		const derivedSeed = keyMaterial.slice(0, 32);
		return Ed25519KeyIdentity.fromSecretKey(derivedSeed);
	}

	function chooseMode(selectedMode: Mode) {
		mode = selectedMode;
		if (selectedMode === 'new') {
			// Generate new seed phrase
			seedPhrase = generateMnemonic();
			currentStep = 'backup-seed';
		} else {
			// Existing user enters seed phrase
			currentStep = 'enter-seed';
		}
	}

	function confirmSeedBackup() {
		if (!seedBackedUp) {
			error = 'Please confirm you have backed up your seed phrase';
			return;
		}
		// After backing up, proceed to username entry for new account
		currentStep = 'enter-username';
		isNewAccount = true;
	}

	function validateSeedPhrase() {
		const trimmed = seedPhrase.trim();
		if (!trimmed) {
			error = 'Please enter your seed phrase';
			return false;
		}
		if (!validateMnemonic(trimmed)) {
			error = 'Invalid seed phrase. Please check and try again.';
			return false;
		}
		error = null;
		return true;
	}

	async function continueWithExistingSeed() {
		if (!validateSeedPhrase()) return;

		currentStep = 'checking-account';
		error = null;

		try {
			const identity = identityFromSeed(seedPhrase);
			const publicKeyBytes = new Uint8Array(identity.getPublicKey().rawKey);
			const publicKeyHex = Array.from(publicKeyBytes)
				.map((b) => b.toString(16).padStart(2, '0'))
				.join('');

			// Check if account exists
			const account = await getAccountByPublicKey(publicKeyHex);

			if (account) {
				// Existing account found - login directly
				await loginWithExistingAccount(account);
			} else {
				// No account found - need to register
				error = 'No account found with this seed phrase. Please check your seed phrase or create a new account.';
				currentStep = 'enter-seed';
			}
		} catch (err) {
			console.error('Account check error:', err);
			error = err instanceof Error ? err.message : 'Failed to check account';
			currentStep = 'enter-seed';
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
			currentStep = 'enter-seed';
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

			// Register account (atomic operation)
			const account = await authStore.registerNewAccount(identity, normalizedUsername);

			// Login with the registered account
			await authStore.loginWithSeedPhrase(seedPhrase, '/dashboard');

			createdAccount = account;
			currentStep = 'success';
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

	function copySeedPhrase() {
		navigator.clipboard.writeText(seedPhrase);
	}

	function handlePaste(e: ClipboardEvent) {
		e.preventDefault();
		const pasted = e.clipboardData?.getData('text') || '';
		seedPhrase = pasted.trim();
	}

	function goBack() {
		if (currentStep === 'enter-seed') {
			currentStep = 'choose-mode';
			mode = null;
			seedPhrase = '';
		} else if (currentStep === 'backup-seed') {
			currentStep = 'choose-mode';
			mode = null;
			seedPhrase = '';
		} else if (currentStep === 'enter-username') {
			if (isNewAccount) {
				currentStep = 'backup-seed';
			} else {
				currentStep = 'enter-seed';
			}
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
					onclick={() => chooseMode('new')}
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
					onclick={() => chooseMode('existing')}
					class="p-6 bg-white/5 hover:bg-white/10 border border-white/20 rounded-xl text-left transition-all group"
				>
					<div class="text-3xl mb-2">🔑</div>
					<h4 class="text-xl font-bold text-white mb-1">Sign In</h4>
					<p class="text-white/60 text-sm">Use your existing seed phrase to sign in</p>
				</button>
			</div>
		</div>
	{/if}

	<!-- Step 2: Backup Seed (New Users) -->
	{#if currentStep === 'backup-seed'}
		<div class="space-y-4">
			<h3 class="text-2xl font-bold text-white">Backup Your Seed Phrase</h3>
			<p class="text-white/60">
				Save these 12 words in a secure location. You'll need them to recover your account.
			</p>

			<!-- Seed phrase display with 12 boxes -->
			<div class="p-4 bg-black/40 border border-white/20 rounded-lg">
				<div class="grid grid-cols-3 gap-2 text-sm">
					{#each seedPhrase.split(' ') as word, i}
						<div class="flex items-center gap-2 p-2 bg-white/5 rounded">
							<span class="text-white/40 text-xs w-4">{i + 1}.</span>
							<span class="text-white font-mono">{word}</span>
						</div>
					{/each}
				</div>
			</div>

			<!-- Copy button -->
			<button
				type="button"
				onclick={copySeedPhrase}
				class="w-full px-4 py-3 bg-white/10 hover:bg-white/20 border border-white/20 rounded-lg text-white transition-colors flex items-center justify-center gap-2"
			>
				<span>📋</span>
				<span>Copy to Clipboard</span>
			</button>

			<!-- Warning -->
			<div class="p-4 bg-yellow-500/10 border border-yellow-500/30 rounded-lg">
				<div class="flex gap-3">
					<span class="text-yellow-400 text-xl">⚠️</span>
					<div class="flex-1 space-y-2">
						<p class="text-sm text-yellow-400 font-medium">Never share your seed phrase!</p>
						<ul class="text-xs text-yellow-400/80 space-y-1 list-disc list-inside">
							<li>Anyone with these words can access your account</li>
							<li>Decent Cloud will never ask for your seed phrase</li>
							<li>Store it offline in a secure location</li>
						</ul>
					</div>
				</div>
			</div>

			<!-- Confirmation checkbox -->
			<label class="flex items-start gap-3 cursor-pointer">
				<input
					type="checkbox"
					bind:checked={seedBackedUp}
					class="mt-1 w-5 h-5 rounded border-white/20 bg-white/5 text-blue-600 focus:ring-2 focus:ring-blue-500/50"
				/>
				<span class="text-sm text-white/80">
					I have saved my seed phrase in a secure location
				</span>
			</label>

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
					onclick={confirmSeedBackup}
					class="flex-1 px-4 py-3 bg-gradient-to-r from-blue-600 to-purple-600 hover:from-blue-500 hover:to-purple-500 rounded-lg text-white font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed"
					disabled={!seedBackedUp}
				>
					Continue
				</button>
			</div>
		</div>
	{/if}

	<!-- Step 3: Enter Seed (Existing Users) -->
	{#if currentStep === 'enter-seed'}
		<div class="space-y-4">
			<h3 class="text-2xl font-bold text-white">Enter Your Seed Phrase</h3>
			<p class="text-white/60">Type or paste your 12-word recovery phrase</p>

			<div class="space-y-2">
				<label for="seedPhrase" class="block text-sm font-medium text-white/70">
					Seed Phrase
				</label>
				<div class="relative">
					<textarea
						id="seedPhrase"
						bind:value={seedPhrase}
						onpaste={handlePaste}
						placeholder="word1 word2 word3 ..."
						rows="4"
						class="w-full px-4 py-3 bg-white/5 border border-white/20 rounded-lg text-white placeholder:text-white/30 focus:outline-none focus:ring-2 focus:ring-purple-500/50 transition-all font-mono text-sm {showSeedEntry
							? ''
							: 'blur-sm'}"
					></textarea>
					<button
						type="button"
						onclick={() => (showSeedEntry = !showSeedEntry)}
						class="absolute top-3 right-3 px-3 py-1 bg-white/10 hover:bg-white/20 rounded text-xs text-white transition-colors"
					>
						{showSeedEntry ? '🙈 Hide' : '👁️ Show'}
					</button>
				</div>

				<!-- Word counter -->
				<div class="text-xs text-white/40">
					{seedPhrase
						.trim()
						.split(/\s+/)
						.filter((w) => w).length} / 12 words
				</div>
			</div>

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
					onclick={continueWithExistingSeed}
					class="flex-1 px-4 py-3 bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-500 hover:to-pink-500 rounded-lg text-white font-medium transition-all"
				>
					Continue
				</button>
			</div>

			<!-- Help text -->
			<div class="pt-4 border-t border-white/10">
				<button
					type="button"
					class="text-sm text-white/60 hover:text-white transition-colors"
				>
					Lost your seed phrase?
				</button>
			</div>
		</div>
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
