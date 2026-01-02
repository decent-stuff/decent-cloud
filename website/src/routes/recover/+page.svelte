<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { requestRecovery, completeRecovery } from '$lib/services/account-api';
	import { identityFromSeed, bytesToHex } from '$lib/utils/identity';
	import SeedPhraseStep from '$lib/components/SeedPhraseStep.svelte';
	import Icon from '$lib/components/Icons.svelte';

	type State = 'request' | 'request-sent' | 'generate-seed' | 'processing' | 'success';

	let currentState = $state<State>('request');
	let email = $state('');
	let token = $state<string | null>(null);
	let seedPhrase = $state('');
	let error = $state<string | null>(null);
	let successMessage = $state('');

	onMount(() => {
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

<div class="min-h-screen bg-base bg-grid bg-radial flex items-center justify-center p-4">
	<div class="w-full max-w-lg">
		<!-- Header -->
		<div class="text-center mb-8">
			<a href="/" class="inline-block">
				<h1 class="text-2xl font-bold text-white tracking-tight mb-2">Decent Cloud</h1>
			</a>
			<p class="text-neutral-500 text-sm">Account Recovery</p>
		</div>

		<!-- Recovery Flow Card -->
		<div class="card p-6 md:p-8">
			<!-- Request Recovery Form -->
			{#if currentState === 'request'}
				<form onsubmit={handleRequestSubmit} class="space-y-4">
					<h3 class="text-xl font-semibold text-white">Request Account Recovery</h3>
					<p class="text-neutral-500 text-sm">
						Enter the email address associated with your account. We'll send you a recovery link.
					</p>

					<div class="space-y-2">
						<label for="email" class="data-label block">Email Address</label>
						<input
							id="email"
							type="email"
							bind:value={email}
							placeholder="your@email.com"
							class="input w-full"
							required
						/>
					</div>

					{#if error}
						<div class="bg-danger/10 border border-danger/20 p-4 text-danger text-sm">
							{error}
						</div>
					{/if}

					<button type="submit" class="btn-primary w-full">
						Send Recovery Link
					</button>
				</form>
			{/if}

			<!-- Request Sent Success -->
			{#if currentState === 'request-sent'}
				<div class="space-y-4 text-center py-8">
					<div class="icon-box-accent mx-auto">
						<Icon name="mail" size={20} />
					</div>
					<h3 class="text-xl font-semibold text-white">Check Your Email</h3>
					<p class="text-neutral-500 text-sm">
						{successMessage || 'We sent a recovery link to your email address. Click the link to continue.'}
					</p>

					<div class="pt-4">
						<button
							type="button"
							onclick={handleBackToRequest}
							class="text-neutral-500 hover:text-white transition-colors text-sm"
						>
							Send to a different email
						</button>
					</div>
				</div>
			{/if}

			<!-- Generate New Seed Phrase -->
			{#if currentState === 'generate-seed'}
				<div class="space-y-4">
					<h3 class="text-xl font-semibold text-white">Complete Recovery</h3>
					<p class="text-neutral-500 text-sm mb-4">
						Generate a new seed phrase to regain access to your account.
					</p>

					{#if error}
						<div class="bg-danger/10 border border-danger/20 p-4 text-danger text-sm">
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
					<div class="icon-box mx-auto">
						<Icon name="key" size={20} />
					</div>
					<h3 class="text-xl font-semibold text-white">Processing</h3>
					<p class="text-neutral-500 text-sm">Please wait...</p>
					<div class="flex justify-center">
						<div class="w-6 h-6 border-2 border-primary-500/30 border-t-primary-500 animate-spin"></div>
					</div>
				</div>
			{/if}

			<!-- Success -->
			{#if currentState === 'success'}
				<div class="space-y-4 text-center py-8">
					<div class="icon-box-accent mx-auto">
						<Icon name="check" size={20} />
					</div>
					<h3 class="text-xl font-semibold text-white">Recovery Complete!</h3>
					<p class="text-neutral-500 text-sm">
						Your account has been recovered. You can now login with your new seed phrase.
					</p>

					<div class="pt-4">
						<button type="button" onclick={handleGoToLogin} class="btn-primary">
							Go to Login
						</button>
					</div>
				</div>
			{/if}
		</div>

		<!-- Back link -->
		{#if currentState === 'request' || currentState === 'request-sent'}
			<div class="text-center mt-6">
				<a href="/login" class="text-neutral-500 hover:text-white transition-colors text-sm">
					← Back to login
				</a>
			</div>
		{/if}
	</div>
</div>
