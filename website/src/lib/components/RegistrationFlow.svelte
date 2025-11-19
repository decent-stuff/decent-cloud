<script lang="ts">
	import { authStore, type AccountInfo } from '$lib/stores/auth';
	import { Ed25519KeyIdentity } from '@dfinity/identity';
	import { generateMnemonic } from 'bip39';
	import { hmac } from '@noble/hashes/hmac';
	import { sha512 } from '@noble/hashes/sha512';
	import { mnemonicToSeedSync } from 'bip39';
	import UsernameInput from './UsernameInput.svelte';

	let { onSuccess, onCancel } = $props<{
		onSuccess: (account: AccountInfo) => void;
		onCancel: () => void;
	}>();

	type Step = 'username' | 'auth-method' | 'seed-backup' | 'confirm' | 'processing' | 'success';
	type AuthMethod = 'seedPhrase' | 'ii';

	let currentStep = $state<Step>('username');
	let username = $state('');
	let usernameValid = $state(false);
	let normalizedUsername = $state('');
	let authMethod = $state<AuthMethod>('seedPhrase');
	let generatedSeedPhrase = $state('');
	let seedBackedUp = $state(false);
	let error = $state<string | null>(null);
	let createdAccount = $state<AccountInfo | null>(null);

	// Generate Ed25519 identity from seed phrase
	function identityFromSeed(seedPhrase: string): Ed25519KeyIdentity {
		const seedBuffer = mnemonicToSeedSync(seedPhrase, '');
		const seedBytes = new Uint8Array(seedBuffer);
		const keyMaterial = hmac(sha512, 'ed25519 seed', seedBytes);
		const derivedSeed = keyMaterial.slice(0, 32);
		return Ed25519KeyIdentity.fromSecretKey(derivedSeed);
	}

	function handleUsernameValidChange(valid: boolean, normalized: string) {
		usernameValid = valid;
		normalizedUsername = normalized;
	}

	function nextStep() {
		if (currentStep === 'username' && usernameValid) {
			currentStep = 'auth-method';
		} else if (currentStep === 'auth-method') {
			if (authMethod === 'seedPhrase') {
				// Generate seed phrase
				generatedSeedPhrase = generateMnemonic();
				currentStep = 'seed-backup';
			} else {
				// Internet Identity flow
				currentStep = 'confirm';
			}
		} else if (currentStep === 'seed-backup' && seedBackedUp) {
			currentStep = 'confirm';
		} else if (currentStep === 'confirm') {
			registerAccount();
		}
	}

	function prevStep() {
		if (currentStep === 'auth-method') {
			currentStep = 'username';
		} else if (currentStep === 'seed-backup') {
			currentStep = 'auth-method';
		} else if (currentStep === 'confirm') {
			if (authMethod === 'seedPhrase') {
				currentStep = 'seed-backup';
			} else {
				currentStep = 'auth-method';
			}
		}
	}

	async function registerAccount() {
		currentStep = 'processing';
		error = null;

		try {
			if (authMethod === 'seedPhrase') {
				// Create identity from seed phrase
				const identity = identityFromSeed(generatedSeedPhrase);

				// Register account
				const account = await authStore.registerNewAccount(identity, normalizedUsername);

				// Store seed phrase and create session
				await authStore.loginWithSeedPhrase(generatedSeedPhrase, '/dashboard');

				createdAccount = account;
				currentStep = 'success';
			} else {
				// Internet Identity flow
				// TODO: Implement II registration
				error = 'Internet Identity registration not yet implemented';
				currentStep = 'confirm';
			}
		} catch (err) {
			error = err instanceof Error ? err.message : 'Registration failed';
			currentStep = 'confirm';
		}
	}

	function copySeedPhrase() {
		navigator.clipboard.writeText(generatedSeedPhrase);
	}

	function skipBackup() {
		// Allow skipping but warn
		if (
			confirm(
				'⚠️ Are you sure? Without backing up your seed phrase, you will lose access to your account if you lose this device.'
			)
		) {
			seedBackedUp = true;
			nextStep();
		}
	}

	function handleSuccess() {
		if (createdAccount) {
			onSuccess(createdAccount);
		}
	}

	// Progress indicator
	const steps = ['username', 'auth-method', 'seed-backup', 'confirm'] as const;
	const currentStepIndex = $derived(
		steps.indexOf(currentStep as (typeof steps)[number]) + 1 || 0
	);
	const totalSteps = $derived(authMethod === 'seedPhrase' ? 4 : 3);
</script>

<div class="space-y-6">
	<!-- Progress indicator -->
	{#if currentStep !== 'processing' && currentStep !== 'success'}
		<div class="flex items-center justify-center gap-2">
			{#each Array(totalSteps) as _, i}
				<div
					class="h-1 flex-1 rounded-full transition-all {i < currentStepIndex
						? 'bg-blue-500'
						: 'bg-white/20'}"
				></div>
			{/each}
		</div>
		<div class="text-center text-sm text-white/60">
			Step {currentStepIndex} of {totalSteps}
		</div>
	{/if}

	<!-- Step 1: Username -->
	{#if currentStep === 'username'}
		<div class="space-y-4">
			<h3 class="text-2xl font-bold text-white">Choose Your Username</h3>
			<p class="text-white/60">This will be your unique identifier on Decent Cloud</p>

			<UsernameInput bind:value={username} onValidChange={handleUsernameValidChange} />

			<div class="flex gap-3">
				<button
					type="button"
					onclick={onCancel}
					class="flex-1 px-4 py-3 bg-white/10 hover:bg-white/20 rounded-lg text-white transition-colors"
				>
					Cancel
				</button>
				<button
					type="button"
					onclick={nextStep}
					disabled={!usernameValid}
					class="flex-1 px-4 py-3 bg-gradient-to-r from-blue-600 to-purple-600 hover:from-blue-500 hover:to-purple-500 rounded-lg text-white font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed"
				>
					Continue
				</button>
			</div>
		</div>
	{/if}

	<!-- Step 2: Auth Method -->
	{#if currentStep === 'auth-method'}
		<div class="space-y-4">
			<h3 class="text-2xl font-bold text-white">Choose Authentication Method</h3>
			<p class="text-white/60">How would you like to secure your account?</p>

			<div class="space-y-3">
				<!-- Seed Phrase (Recommended) -->
				<button
					type="button"
					onclick={() => (authMethod = 'seedPhrase')}
					class="w-full p-4 border-2 rounded-xl transition-all text-left {authMethod ===
					'seedPhrase'
						? 'border-blue-500 bg-blue-500/20'
						: 'border-white/20 bg-white/5 hover:border-white/30'}"
				>
					<div class="flex items-start gap-3">
						<span class="text-3xl">🔑</span>
						<div class="flex-1">
							<div class="flex items-center gap-2">
								<h4 class="text-white font-semibold">Seed Phrase</h4>
								<span
									class="px-2 py-0.5 bg-green-500/20 border border-green-500/30 rounded text-xs text-green-400"
									>Recommended</span
								>
							</div>
							<p class="text-sm text-white/60 mt-1">
								12-word recovery phrase. You have full control of your keys.
							</p>
						</div>
						{#if authMethod === 'seedPhrase'}
							<span class="text-blue-400 text-xl">✓</span>
						{/if}
					</div>
				</button>

				<!-- Internet Identity -->
				<button
					type="button"
					onclick={() => (authMethod = 'ii')}
					class="w-full p-4 border-2 rounded-xl transition-all text-left {authMethod === 'ii'
						? 'border-blue-500 bg-blue-500/20'
						: 'border-white/20 bg-white/5 hover:border-white/30'}"
				>
					<div class="flex items-start gap-3">
						<span class="text-3xl">🆔</span>
						<div class="flex-1">
							<h4 class="text-white font-semibold">Internet Identity</h4>
							<p class="text-sm text-white/60 mt-1">
								ICP's secure authentication. Managed by Internet Computer.
							</p>
						</div>
						{#if authMethod === 'ii'}
							<span class="text-blue-400 text-xl">✓</span>
						{/if}
					</div>
				</button>
			</div>

			<div class="flex gap-3">
				<button
					type="button"
					onclick={prevStep}
					class="flex-1 px-4 py-3 bg-white/10 hover:bg-white/20 rounded-lg text-white transition-colors"
				>
					Back
				</button>
				<button
					type="button"
					onclick={nextStep}
					class="flex-1 px-4 py-3 bg-gradient-to-r from-blue-600 to-purple-600 hover:from-blue-500 hover:to-purple-500 rounded-lg text-white font-medium transition-all"
				>
					Continue
				</button>
			</div>
		</div>
	{/if}

	<!-- Step 3: Seed Backup -->
	{#if currentStep === 'seed-backup'}
		<div class="space-y-4">
			<h3 class="text-2xl font-bold text-white">Backup Your Seed Phrase</h3>
			<p class="text-white/60">
				Save these 12 words in a secure location. You'll need them to recover your account.
			</p>

			<!-- Seed phrase display -->
			<div class="p-4 bg-black/40 border border-white/20 rounded-lg">
				<div class="grid grid-cols-3 gap-2 text-sm">
					{#each generatedSeedPhrase.split(' ') as word, i}
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

			<div class="flex gap-3">
				<button
					type="button"
					onclick={prevStep}
					class="flex-1 px-4 py-3 bg-white/10 hover:bg-white/20 rounded-lg text-white transition-colors"
				>
					Back
				</button>
				<button
					type="button"
					onclick={skipBackup}
					class="px-4 py-3 text-white/60 hover:text-white text-sm transition-colors"
				>
					Skip
				</button>
				<button
					type="button"
					onclick={nextStep}
					disabled={!seedBackedUp}
					class="flex-1 px-4 py-3 bg-gradient-to-r from-blue-600 to-purple-600 hover:from-blue-500 hover:to-purple-500 rounded-lg text-white font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed"
				>
					Continue
				</button>
			</div>
		</div>
	{/if}

	<!-- Step 4: Confirm -->
	{#if currentStep === 'confirm'}
		<div class="space-y-4">
			<h3 class="text-2xl font-bold text-white">Confirm Your Details</h3>
			<p class="text-white/60">Review your account details before creating</p>

			<div class="space-y-3 p-4 bg-white/5 border border-white/20 rounded-lg">
				<div>
					<div class="text-sm text-white/60">Username</div>
					<div class="text-white font-medium">@{normalizedUsername}</div>
				</div>
				<div class="border-t border-white/10"></div>
				<div>
					<div class="text-sm text-white/60">Authentication Method</div>
					<div class="text-white font-medium">
						{authMethod === 'seedPhrase' ? 'Seed Phrase' : 'Internet Identity'}
					</div>
				</div>
			</div>

			{#if error}
				<div class="p-4 bg-red-500/20 border border-red-500/30 rounded-lg text-red-400 text-sm">
					{error}
				</div>
			{/if}

			<div class="flex gap-3">
				<button
					type="button"
					onclick={prevStep}
					class="flex-1 px-4 py-3 bg-white/10 hover:bg-white/20 rounded-lg text-white transition-colors"
				>
					Back
				</button>
				<button
					type="button"
					onclick={nextStep}
					class="flex-1 px-4 py-3 bg-gradient-to-r from-blue-600 to-purple-600 hover:from-blue-500 hover:to-purple-500 rounded-lg text-white font-medium transition-all"
				>
					Create Account
				</button>
			</div>
		</div>
	{/if}

	<!-- Step 5: Processing -->
	{#if currentStep === 'processing'}
		<div class="space-y-4 text-center py-8">
			<div class="text-6xl animate-bounce">🚀</div>
			<h3 class="text-2xl font-bold text-white">Creating Your Account</h3>
			<p class="text-white/60">This will only take a moment...</p>
			<div class="flex justify-center">
				<div class="w-8 h-8 border-4 border-blue-500/30 border-t-blue-500 rounded-full animate-spin"></div>
			</div>
		</div>
	{/if}

	<!-- Step 6: Success -->
	{#if currentStep === 'success' && createdAccount}
		<div class="space-y-4 text-center py-8">
			<div class="text-6xl">🎉</div>
			<h3 class="text-2xl font-bold text-white">Welcome to Decent Cloud!</h3>
			<p class="text-white/60">
				Your account <span class="text-white font-medium">@{createdAccount.username}</span> is ready
			</p>

			<div class="pt-4">
				<button
					type="button"
					onclick={handleSuccess}
					class="px-8 py-3 bg-gradient-to-r from-blue-600 to-purple-600 hover:from-blue-500 hover:to-purple-500 rounded-lg text-white font-medium transition-all"
				>
					Go to Dashboard
				</button>
			</div>
		</div>
	{/if}
</div>
