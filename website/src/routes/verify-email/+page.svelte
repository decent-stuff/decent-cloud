<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { verifyEmail } from '$lib/services/account-api';
	import Icon from '$lib/components/Icons.svelte';

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

<div class="min-h-screen bg-base bg-grid bg-radial flex items-center justify-center p-4">
	<div class="w-full max-w-lg">
		<!-- Header -->
		<div class="text-center mb-8">
			<a href="/" class="inline-block">
				<h1 class="text-2xl font-bold text-white tracking-tight mb-2">Decent Cloud</h1>
			</a>
			<p class="text-neutral-500 text-sm">Email Verification</p>
		</div>

		<!-- Verification Card -->
		<div class="card p-6 md:p-8">
			<!-- Verifying -->
			{#if currentState === 'verifying'}
				<div class="space-y-4 text-center py-8">
					<div class="icon-box mx-auto">
						<Icon name="mail" size={20} />
					</div>
					<h3 class="text-xl font-semibold text-white">Verifying Email</h3>
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
					<h3 class="text-xl font-semibold text-success">Email Verified!</h3>
					<div class="space-y-2">
						<p class="text-white text-base">
							Thank you for verifying your email!
						</p>
						<p class="text-neutral-500 text-sm">
							Your account reputation has been improved. You now have full access to all platform features.
						</p>
					</div>

					<div class="pt-4 flex flex-col gap-3">
						<button type="button" onclick={handleGoToDashboard} class="btn-primary">
							Go to Dashboard
						</button>
						<button
							type="button"
							onclick={handleGoToLogin}
							class="text-neutral-500 hover:text-white transition-colors text-sm"
						>
							Go to Login
						</button>
					</div>
				</div>
			{/if}

			<!-- Error -->
			{#if currentState === 'error'}
				<div class="space-y-4 text-center py-8">
					<div class="icon-box mx-auto">
						<Icon name="x" size={20} />
					</div>
					<h3 class="text-xl font-semibold text-white">Verification Failed</h3>
					<div class="bg-danger/10 border border-danger/20 p-4 text-danger text-sm">
						{error || 'Email verification failed'}
					</div>
					<p class="text-neutral-500 text-sm">
						The verification link may have expired or been used already.
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
		<div class="text-center mt-6">
			<a href="/" class="text-neutral-500 hover:text-white transition-colors text-sm">
				← Back to home
			</a>
		</div>
	</div>
</div>
