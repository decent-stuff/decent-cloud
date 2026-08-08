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
	import Button from '$lib/components/Button.svelte';
	import { getAuthCtaClass } from '$lib/utils/auth-cta';

	let { onSuccess } = $props<{
		onSuccess: (account: AccountInfo, isNewRegistration: boolean) => void;
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
	let showSeedPhrase = $state(false);
	let initialSeedMode = $state<'choose' | 'generate' | 'import'>('choose');
	// Whether the server reports Google OAuth as configured (null = still
	// loading). When false, the seed-phrase form is the default surface with
	// no extra click — the OAuth-first layout only makes sense when OAuth
	// actually works (#436). On fetch error we fall back to true (show
	// everything), the safe pre-capability behavior.
	let googleOAuthEnabled = $state<boolean | null>(null);
	// Tracks whether the success state came from a fresh registration (vs an
	// existing-account login) so the host page can route to /dashboard (where
	// the first-login WelcomeModal renders) instead of the generic returnUrl.
	let isNewRegistration = $state(false);

	// The seed-phrase form is shown by default whenever Google OAuth is off, so
	// users on a non-OAuth deployment don't need the extra "seed phrase instead"
	// click to reach credential sign-in.
	const effectiveShowSeed = $derived(showSeedPhrase || googleOAuthEnabled === false);

	function showRegistration() {
		initialSeedMode = 'generate';
		showSeedPhrase = true;
	}

	function showSignIn() {
		initialSeedMode = 'choose';
		showSeedPhrase = true;
	}

	onMount(async () => {
		if (typeof window === 'undefined') return;

		// Discover which sign-in methods the server actually supports so the
		// form layout matches reality (#436). Errors fall back to "show
		// everything" (googleOAuthEnabled = true) — the safe pre-capability
		// behavior — so a capability-fetch failure never blocks login.
		try {
			const response = await fetch(`${API_BASE_URL}/api/v1/auth/capabilities`);
			if (response.ok) {
				const caps = (await response.json()) as { google_oauth?: boolean };
				googleOAuthEnabled = Boolean(caps.google_oauth);
			} else {
				googleOAuthEnabled = true;
			}
		} catch {
			googleOAuthEnabled = true;
		}

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
			isNewRegistration = false;
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
			isNewRegistration = true;
			currentStep = 'success';
		} catch (err) {
			error = err instanceof Error ? err.message : 'Registration failed';
			currentStep = 'enter-username';
		}
	}

	function handleSuccess() {
		if (createdAccount) {
			onSuccess(createdAccount, isNewRegistration);
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
				isNewRegistration = true;
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
{#if currentStep === 'seed'}
	<div class="space-y-6">
		<div class="text-center">
			<h2 class="text-2xl font-bold text-white mb-2">Sign In or Create Account</h2>
			<p class="text-neutral-500 text-sm">
				{googleOAuthEnabled === false
					? 'Use your seed phrase to sign in'
					: 'Use your Google account or seed phrase'}
			</p>
		</div>

		{#if googleOAuthEnabled}
			<GoogleSignInButton />
		{/if}

		{#if !effectiveShowSeed}
			<button
				type="button"
				onclick={showSignIn}
				class={getAuthCtaClass('seed')}
			>
				Sign in with seed phrase instead
			</button>
		{:else}
			{#if googleOAuthEnabled}
				<div class="relative">
					<div class="absolute inset-0 flex items-center">
						<div class="w-full border-t border-neutral-800"></div>
					</div>
					<div class="relative flex justify-center text-sm">
						<span class="px-3 bg-surface text-neutral-500 text-xs uppercase tracking-wider">or</span>
					</div>
				</div>
			{/if}

		<!-- Key on the requested mode so the component remounts when the user
		picks "Create an account" (generate) vs "Sign in with seed phrase" (choose).
		SeedPhraseStep only reads initialMode on mount, and when OAuth is off the
		form is mounted up front in 'choose' mode, so without the key the mode
		switch would be silently ignored. -->
		{#key initialSeedMode}
			<SeedPhraseStep
				initialMode={initialSeedMode}
				showModeChoice={true}
				onComplete={handleSeedComplete}
			/>
		{/key}
		{/if}

	<!-- "Create an account" jumps straight to seed generation. It is the PRIMARY
	create path when the seed-phrase chooser is hidden (OAuth on), but redundant
	with the "Generate New" card when the chooser is already visible (OAuth off,
	choose mode) — F8 collapses to a single create path, so hide it then. -->
	{#if !effectiveShowSeed || initialSeedMode !== 'choose'}
		<p class="text-center text-sm text-neutral-500 pt-1">
			New here?
			<button type="button" onclick={showRegistration} class="text-primary-400 hover:text-primary-300 font-medium underline">
				Create an account
			</button>
		</p>
	{/if}
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
				<Button variant="secondary" type="button" onclick={goBack} class="flex-1">
					Back
				</Button>
				<Button
					variant="primary"
					type="button"
					onclick={registerAndLogin}
					class="flex-1 disabled:opacity-50 disabled:cursor-not-allowed"
					disabled={!usernameValid || !emailValid}
				>
					Create Account
				</Button>
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

			<Button
				variant="primary"
				type="button"
				onclick={submitOAuthUsername}
				class="w-full disabled:opacity-50 disabled:cursor-not-allowed"
				disabled={!usernameValid}
			>
				Create Account
			</Button>
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
				<Button variant="primary" type="button" onclick={handleSuccess}>
					Go to Dashboard
				</Button>
			</div>
		</div>
	{/if}
</div>
