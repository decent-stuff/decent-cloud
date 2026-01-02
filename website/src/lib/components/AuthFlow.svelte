<script lang="ts">
	import { authStore, type AccountInfo } from '$lib/stores/auth';
	import { getAccountByPublicKey } from '$lib/services/account-api';
	import { identityFromSeed, bytesToHex } from '$lib/utils/identity';
	import { API_BASE_URL } from '$lib/services/api';
	import UsernameInput from './UsernameInput.svelte';
	import SeedPhraseStep from './SeedPhraseStep.svelte';
	import GoogleSignInButton from './GoogleSignInButton.svelte';
	import Icon from './Icons.svelte';
	import { onMount } from 'svelte';

	let { onSuccess } = $props<{
		onSuccess: (account: AccountInfo) => void;
	}>();

	type Step =
		| 'seed'
		| 'checking-account'
		| 'enter-username'
		| 'oauth-username'
		| 'processing'
		| 'success';

	let currentStep = $state<Step>('seed');
	let seedPhrase = $state('');
	let username = $state('');
	let usernameValid = $state(false);
	let normalizedUsername = $state('');
	let email = $state('');
	let emailValid = $state(false);
	let error = $state<string | null>(null);
	let createdAccount = $state<AccountInfo | null>(null);

	onMount(async () => {
		if (typeof window === 'undefined') return;
		const urlParams = new URLSearchParams(window.location.search);
		if (urlParams.get('oauth') === 'google' && urlParams.get('step') === 'username') {
			currentStep = 'oauth-username';

			try {
				const response = await fetch(`${API_BASE_URL}/api/v1/oauth/info`, {
					credentials: 'include'
				});
				if (response.ok) {
					const result = await response.json();
					if (result.success && result.data?.email) {
						const emailPrefix = result.data.email.split('@')[0];
						const suggestedUsername = emailPrefix.replace(/[^a-z0-9_]/gi, '_');
						username = suggestedUsername;
					}
				}
			} catch (err) {
				console.error('Failed to fetch OAuth info:', err);
			}
		}
	});

	async function handleSeedComplete(seed: string, deviceName?: string) {
		seedPhrase = seed;

		currentStep = 'checking-account';
		error = null;

		try {
			const identity = identityFromSeed(seedPhrase);
			const publicKeyBytes = new Uint8Array(identity.getPublicKey().rawKey);
			const publicKeyHex = bytesToHex(publicKeyBytes);

			const account = await getAccountByPublicKey(publicKeyHex);

			if (account) {
				await loginWithExistingAccount(account);
			} else {
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
			await authStore.loginWithSeedPhrase(seedPhrase, '/dashboard/marketplace');
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

	function validateEmail() {
		const trimmed = email.trim();
		if (!trimmed) {
			emailValid = false;
			return;
		}
		const emailPattern = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
		emailValid = emailPattern.test(trimmed);
	}

	async function registerAndLogin() {
		if (!usernameValid) {
			error = 'Please enter a valid username';
			return;
		}
		if (!emailValid) {
			error = 'Please enter a valid email address';
			return;
		}

		currentStep = 'processing';
		error = null;

		try {
			const identity = identityFromSeed(seedPhrase);

			const account = await authStore.registerNewAccount(
				identity,
				normalizedUsername,
				email.trim(),
				seedPhrase
			);

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

	function goBack() {
		if (currentStep === 'enter-username') {
			currentStep = 'seed';
			seedPhrase = '';
			email = '';
			emailValid = false;
		}
	}

	async function submitOAuthUsername() {
		if (!usernameValid) {
			error = 'Please enter a valid username';
			return;
		}

		currentStep = 'processing';
		error = null;

		try {
			const response = await fetch(`${API_BASE_URL}/api/v1/oauth/register`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				credentials: 'include',
				body: JSON.stringify({ username: normalizedUsername })
			});

			const result = await response.json();

			if (result.success && result.data) {
				await authStore.loadOAuthSession();
				createdAccount = result.data;
				currentStep = 'success';
			} else {
				error = result.error || 'Registration failed';
				currentStep = 'oauth-username';
			}
		} catch (err) {
			console.error('OAuth registration error:', err);
			error = err instanceof Error ? err.message : 'Network error';
			currentStep = 'oauth-username';
		}
	}
</script>

<div class="space-y-6">
	<!-- Step 1: Seed Phrase (Generate or Import) -->
	{#if currentStep === 'seed'}
		<div class="space-y-4">
			<GoogleSignInButton />

			<div class="relative">
				<div class="absolute inset-0 flex items-center">
					<div class="w-full border-t border-neutral-800"></div>
				</div>
				<div class="relative flex justify-center text-sm">
					<span class="px-3 bg-surface text-neutral-500 text-xs uppercase tracking-wider">or</span>
				</div>
			</div>

			<SeedPhraseStep
				initialMode="choose"
				showModeChoice={true}
				onComplete={handleSeedComplete}
			/>
		</div>
	{/if}

	<!-- Checking Account -->
	{#if currentStep === 'checking-account'}
		<div class="space-y-6 text-center py-8">
			<div class="w-16 h-16 mx-auto bg-surface-elevated border border-neutral-700 flex items-center justify-center">
				<Icon name="search" size={28} class="text-primary-400 animate-pulse-subtle" />
			</div>
			<div>
				<h3 class="text-xl font-bold text-white mb-2">Detecting Account</h3>
				<p class="text-neutral-500">Looking up your account...</p>
			</div>
			<div class="flex justify-center">
				<div class="w-6 h-6 border-2 border-neutral-700 border-t-primary-500 rounded-full animate-spin"></div>
			</div>
		</div>
	{/if}

	<!-- Enter Username -->
	{#if currentStep === 'enter-username'}
		<div class="space-y-6">
			<div>
				<h3 class="text-xl font-bold text-white mb-2">Create Your Account</h3>
				<p class="text-neutral-500">Choose a username and provide your email address</p>
			</div>

			<UsernameInput
				bind:value={username}
				onValidChange={handleUsernameValidChange}
			/>

			<div class="space-y-2">
				<label for="email" class="block text-sm font-medium text-neutral-400">
					Email Address
				</label>
				<input
					id="email"
					type="email"
					bind:value={email}
					oninput={validateEmail}
					placeholder="you@example.com"
					class="w-full px-4 py-3 bg-surface-elevated border border-neutral-700 text-white placeholder-neutral-600 focus:outline-none focus:border-primary-500 transition-colors"
					required
				/>
				{#if email && !emailValid}
					<p class="text-xs text-danger">Please enter a valid email address</p>
				{/if}
			</div>

			{#if error}
				<div class="p-4 bg-danger/10 border border-danger/30 text-danger text-sm">
					{error}
				</div>
			{/if}

			<div class="flex gap-3">
				<button
					type="button"
					onclick={goBack}
					class="flex-1 px-4 py-3 bg-surface-elevated border border-neutral-700 hover:border-neutral-600 text-white transition-colors"
				>
					Back
				</button>
				<button
					type="button"
					onclick={registerAndLogin}
					class="flex-1 px-4 py-3 bg-primary-500 hover:bg-primary-400 text-base font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
					disabled={!usernameValid || !emailValid}
				>
					Create Account
				</button>
			</div>
		</div>
	{/if}

	<!-- OAuth Username Step -->
	{#if currentStep === 'oauth-username'}
		<div class="space-y-6">
			<div>
				<h3 class="text-xl font-bold text-white mb-2">Welcome to Decent Cloud</h3>
				<p class="text-neutral-500">Choose a username to complete your Google sign-in</p>
			</div>

			<UsernameInput
				bind:value={username}
				onValidChange={handleUsernameValidChange}
			/>

			{#if error}
				<div class="p-4 bg-danger/10 border border-danger/30 text-danger text-sm">
					{error}
				</div>
			{/if}

			<button
				type="button"
				onclick={submitOAuthUsername}
				class="w-full px-4 py-3 bg-primary-500 hover:bg-primary-400 text-base font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
				disabled={!usernameValid}
			>
				Create Account
			</button>
		</div>
	{/if}

	<!-- Processing -->
	{#if currentStep === 'processing'}
		<div class="space-y-6 text-center py-8">
			<div class="w-16 h-16 mx-auto bg-surface-elevated border border-neutral-700 flex items-center justify-center">
				<Icon name="lock" size={28} class="text-primary-400" />
			</div>
			<div>
				<h3 class="text-xl font-bold text-white mb-2">
					{createdAccount ? 'Signing You In' : 'Creating Your Account'}
				</h3>
				<p class="text-neutral-500">Please wait...</p>
			</div>
			<div class="flex justify-center">
				<div class="w-6 h-6 border-2 border-neutral-700 border-t-primary-500 rounded-full animate-spin"></div>
			</div>
		</div>
	{/if}

	<!-- Success -->
	{#if currentStep === 'success' && createdAccount}
		<div class="space-y-6 text-center py-8">
			<div class="w-16 h-16 mx-auto bg-primary-500/10 border border-primary-500/30 flex items-center justify-center">
				<Icon name="check" size={28} class="text-primary-400" />
			</div>
			<div>
				<h3 class="text-xl font-bold text-white mb-2">Welcome to Decent Cloud</h3>
				<p class="text-neutral-400">
					Signed in as <span class="text-white font-medium">@{createdAccount.username}</span>
				</p>
			</div>

			{#if email}
				<div class="p-4 bg-info/10 border border-info/30">
					<p class="text-info text-sm font-medium">Check your email to verify your account</p>
					<p class="text-neutral-500 text-xs mt-1">We sent a verification link to <span class="font-medium text-neutral-400">{email}</span></p>
				</div>
			{/if}

			<div class="pt-2">
				<button
					type="button"
					onclick={handleSuccess}
					class="px-8 py-3 bg-primary-500 hover:bg-primary-400 text-base font-medium transition-colors"
				>
					Go to Dashboard
				</button>
			</div>
		</div>
	{/if}
</div>
