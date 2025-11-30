<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { requestRecovery, completeRecovery } from '$lib/services/account-api';
	import { identityFromSeed, bytesToHex } from '$lib/utils/identity';
	import SeedPhraseStep from '$lib/components/SeedPhraseStep.svelte';

	type State = 'request' | 'request-sent' | 'generate-seed' | 'processing' | 'success';

	let currentState = $state<State>('request');
	let email = $state('');
	let token = $state<string | null>(null);
	let seedPhrase = $state('');
	let error = $state<string | null>(null);
	let successMessage = $state('');

	onMount(() => {
		// Check if URL has token parameter
		const urlToken = $page.url.searchParams.get('token');
		if (urlToken) {
			token = urlToken;
			currentState = 'generate-seed';
		}
	});

	async function handleRequestSubmit(e: Event) {
		e.preventDefault();
		if (!email.trim()) {
			error = 'Please enter your email address';
			return;
		}

		currentState = 'processing';
		error = null;

		try {
			const message = await requestRecovery(email);
			successMessage = message;
			currentState = 'request-sent';
		} catch (err) {
			error = err instanceof Error ? err.message : 'Recovery request failed';
			currentState = 'request';
		}
	}

	async function handleSeedComplete(seed: string) {
		seedPhrase = seed;

		if (!token) {
			error = 'Recovery token is missing';
			currentState = 'generate-seed';
			return;
		}

		currentState = 'processing';
		error = null;

		try {
			const identity = identityFromSeed(seedPhrase);
			const publicKeyBytes = new Uint8Array(identity.getPublicKey().rawKey);
			const publicKeyHex = bytesToHex(publicKeyBytes);

			await completeRecovery(token, publicKeyHex);

			currentState = 'success';
		} catch (err) {
			error = err instanceof Error ? err.message : 'Recovery completion failed';
			currentState = 'generate-seed';
		}
	}

	function handleBackToRequest() {
		currentState = 'request';
		email = '';
		error = null;
	}

	function handleGoToLogin() {
		goto('/login');
	}
</script>

<svelte:head>
	<title>Account Recovery - Decent Cloud</title>
</svelte:head>

<div class="min-h-screen bg-gradient-to-br from-gray-900 via-blue-900 to-purple-900 flex items-center justify-center p-4">
	<div class="w-full max-w-lg">
		<!-- Header -->
		<div class="text-center mb-8">
			<a href="/" class="inline-block">
				<h1 class="text-4xl font-bold text-white mb-2">Decent Cloud</h1>
			</a>
			<p class="text-white/70">Account Recovery</p>
		</div>

		<!-- Recovery Flow Card -->
		<div class="bg-gray-900/95 backdrop-blur-lg rounded-2xl p-6 md:p-8 border border-white/20 shadow-2xl">
			<!-- Request Recovery Form -->
			{#if currentState === 'request'}
				<form onsubmit={handleRequestSubmit} class="space-y-4">
					<h3 class="text-2xl font-bold text-white">Request Account Recovery</h3>
					<p class="text-white/60">
						Enter the email address associated with your account. We'll send you a recovery link.
					</p>

					<div class="space-y-2">
						<label for="email" class="block text-sm font-medium text-white/70">
							Email Address
						</label>
						<input
							id="email"
							type="email"
							bind:value={email}
							placeholder="your@email.com"
							class="w-full px-4 py-3 bg-white/5 border border-white/20 rounded-lg text-white placeholder:text-white/30 focus:outline-none focus:ring-2 focus:ring-purple-500/50 transition-all"
							required
						/>
					</div>

					{#if error}
						<div class="p-4 bg-red-500/20 border border-red-500/30 rounded-lg text-red-400 text-sm">
							{error}
						</div>
					{/if}

					<button
						type="submit"
						class="w-full px-4 py-3 bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-500 hover:to-pink-500 rounded-lg text-white font-medium transition-all"
					>
						Send Recovery Link
					</button>
				</form>
			{/if}

			<!-- Request Sent Success -->
			{#if currentState === 'request-sent'}
				<div class="space-y-4 text-center py-8">
					<div class="text-6xl">✉️</div>
					<h3 class="text-2xl font-bold text-white">Check Your Email</h3>
					<p class="text-white/60">
						{successMessage || 'We sent a recovery link to your email address. Click the link to continue.'}
					</p>

					<div class="pt-4">
						<button
							type="button"
							onclick={handleBackToRequest}
							class="text-white/60 hover:text-white transition-colors text-sm"
						>
							Send to a different email
						</button>
					</div>
				</div>
			{/if}

			<!-- Generate New Seed Phrase -->
			{#if currentState === 'generate-seed'}
				<div class="space-y-4">
					<h3 class="text-2xl font-bold text-white">Complete Recovery</h3>
					<p class="text-white/60 mb-4">
						Generate a new seed phrase to regain access to your account.
					</p>

					{#if error}
						<div class="p-4 bg-red-500/20 border border-red-500/30 rounded-lg text-red-400 text-sm">
							{error}
						</div>
					{/if}

					<SeedPhraseStep
						initialMode="generate"
						showModeChoice={false}
						onComplete={handleSeedComplete}
					/>
				</div>
			{/if}

			<!-- Processing -->
			{#if currentState === 'processing'}
				<div class="space-y-4 text-center py-8">
					<div class="text-6xl animate-pulse">🔐</div>
					<h3 class="text-2xl font-bold text-white">Processing</h3>
					<p class="text-white/60">Please wait...</p>
					<div class="flex justify-center">
						<div
							class="w-8 h-8 border-4 border-purple-500/30 border-t-purple-500 rounded-full animate-spin"
						></div>
					</div>
				</div>
			{/if}

			<!-- Success -->
			{#if currentState === 'success'}
				<div class="space-y-4 text-center py-8">
					<div class="text-6xl">✅</div>
					<h3 class="text-2xl font-bold text-white">Recovery Complete!</h3>
					<p class="text-white/60">
						Your account has been recovered. You can now login with your new seed phrase.
					</p>

					<div class="pt-4">
						<button
							type="button"
							onclick={handleGoToLogin}
							class="px-8 py-3 bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-500 hover:to-pink-500 rounded-lg text-white font-medium transition-all"
						>
							Go to Login
						</button>
					</div>
				</div>
			{/if}
		</div>

		<!-- Back link -->
		{#if currentState === 'request' || currentState === 'request-sent'}
			<div class="text-center mt-6">
				<a href="/login" class="text-white/60 hover:text-white transition-colors text-sm">
					← Back to login
				</a>
			</div>
		{/if}
	</div>
</div>
