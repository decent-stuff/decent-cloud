<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { authStore } from '$lib/stores/auth';
	import AuthFlow from '$lib/components/AuthFlow.svelte';
	import type { AccountInfo } from '$lib/stores/auth';

	let returnUrl = $state<string>('/dashboard');
	let isAuthenticated = $state(false);

	onMount(() => {
		// Get returnUrl from query params
		const urlReturnUrl = $page.url.searchParams.get('returnUrl');
		if (urlReturnUrl) {
			returnUrl = urlReturnUrl;
		}

		// Check if already authenticated
		const unsubscribe = authStore.isAuthenticated.subscribe((value) => {
			isAuthenticated = value;
			if (value) {
				// Already logged in, redirect to returnUrl
				goto(returnUrl);
			}
		});

		return () => {
			unsubscribe();
		};
	});

	function handleSuccess(account: AccountInfo) {
		console.log('Auth success:', account);
		// Navigate to returnUrl
		goto(returnUrl);
	}

	function handleCancel() {
		// Go back or to home
		goto('/');
	}
</script>

<svelte:head>
	<title>Login - Decent Cloud</title>
</svelte:head>

<div class="min-h-screen bg-gradient-to-br from-gray-900 via-blue-900 to-purple-900 flex items-center justify-center p-4">
	<div class="w-full max-w-lg">
		<!-- Header -->
		<div class="text-center mb-8">
			<a href="/" class="inline-block">
				<h1 class="text-4xl font-bold text-white mb-2">Decent Cloud</h1>
			</a>
			<p class="text-white/70">Login or create your account</p>
		</div>

		<!-- Auth Flow Card -->
		<div class="bg-gray-900/95 backdrop-blur-lg rounded-2xl p-6 md:p-8 border border-white/20 shadow-2xl">
			{#if !isAuthenticated}
				<AuthFlow onSuccess={handleSuccess} />
			{:else}
				<div class="text-center py-8">
					<div class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-400 mx-auto mb-4"></div>
					<p class="text-white/70">Redirecting...</p>
				</div>
			{/if}
		</div>

		<!-- Back link -->
		<div class="text-center mt-6">
			<button
				type="button"
				onclick={handleCancel}
				class="text-white/60 hover:text-white transition-colors text-sm"
			>
				← Back to home
			</button>
		</div>
	</div>
</div>
