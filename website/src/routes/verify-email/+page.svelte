<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { verifyEmail } from '$lib/services/account-api';

	type State = 'verifying' | 'success' | 'error';

	let currentState = $state<State>('verifying');
	let error = $state<string | null>(null);
	let successMessage = $state('');

	onMount(async () => {
		const token = $page.url.searchParams.get('token');
		if (!token) {
			error = 'Verification token is missing from the URL';
			currentState = 'error';
			return;
		}

		try {
			const message = await verifyEmail(token);
			successMessage = message;
			currentState = 'success';
		} catch (err) {
			error = err instanceof Error ? err.message : 'Email verification failed';
			currentState = 'error';
		}
	});

	function handleGoToLogin() {
		goto('/login');
	}

	function handleGoToDashboard() {
		goto('/dashboard/marketplace');
	}
</script>

<svelte:head>
	<title>Verify Email - Decent Cloud</title>
</svelte:head>

<div class="min-h-screen bg-gradient-to-br from-base via-surface to-surface flex items-center justify-center p-4">
	<div class="w-full max-w-lg">
		<!-- Header -->
		<div class="text-center mb-8">
			<a href="/" class="inline-block">
				<h1 class="text-4xl font-bold text-white mb-2">Decent Cloud</h1>
			</a>
			<p class="text-white/70">Email Verification</p>
		</div>

		<!-- Verification Card -->
		<div class="bg-surface/95 backdrop-blur-lg rounded-2xl p-6 md:p-8 border border-glass/15 shadow-2xl">
			<!-- Verifying -->
			{#if currentState === 'verifying'}
				<div class="space-y-4 text-center py-8">
					<div class="text-6xl animate-pulse">✉️</div>
					<h3 class="text-2xl font-bold text-white">Verifying Email</h3>
					<p class="text-white/60">Please wait...</p>
					<div class="flex justify-center">
						<div
							class="w-8 h-8 border-4 border-primary-500/30 border-t-primary-500 rounded-full animate-spin"
						></div>
					</div>
				</div>
			{/if}

			<!-- Success -->
			{#if currentState === 'success'}
				<div class="space-y-4 text-center py-8">
					<div class="text-6xl">✅</div>
					<h3 class="text-2xl font-bold text-green-400">Email Verified!</h3>
					<div class="space-y-2">
						<p class="text-white text-lg">
							Thank you for verifying your email!
						</p>
						<p class="text-white/70">
							Your account reputation has been improved. You now have full access to all platform features.
						</p>
					</div>

					<div class="pt-4 flex flex-col gap-3">
						<button
							type="button"
							onclick={handleGoToDashboard}
							class="px-8 py-3 bg-gradient-to-r from-primary-500 to-primary-600 hover:from-primary-400 hover:to-primary-500 rounded-lg text-white font-medium transition-all"
						>
							Go to Dashboard
						</button>
						<button
							type="button"
							onclick={handleGoToLogin}
							class="text-white/60 hover:text-white transition-colors text-sm"
						>
							Go to Login
						</button>
					</div>
				</div>
			{/if}

			<!-- Error -->
			{#if currentState === 'error'}
				<div class="space-y-4 text-center py-8">
					<div class="text-6xl">❌</div>
					<h3 class="text-2xl font-bold text-white">Verification Failed</h3>
					<div class="p-4 bg-red-500/20 border border-red-500/30 rounded-lg text-red-400 text-sm">
						{error || 'Email verification failed'}
					</div>
					<p class="text-white/60 text-sm">
						The verification link may have expired or been used already.
					</p>

					<div class="pt-4">
						<button
							type="button"
							onclick={handleGoToLogin}
							class="px-8 py-3 bg-gradient-to-r from-primary-500 to-primary-600 hover:from-primary-400 hover:to-primary-500 rounded-lg text-white font-medium transition-all"
						>
							Go to Login
						</button>
					</div>
				</div>
			{/if}
		</div>

		<!-- Back link -->
		<div class="text-center mt-6">
			<a href="/" class="text-white/60 hover:text-white transition-colors text-sm">
				← Back to home
			</a>
		</div>
	</div>
</div>
