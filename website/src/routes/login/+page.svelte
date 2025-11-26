<script lang="ts">
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { authStore } from '$lib/stores/auth';
	import AuthFlow from '$lib/components/AuthFlow.svelte';
	import type { AccountInfo } from '$lib/stores/auth';

	let returnUrl = $state<string>('/dashboard/marketplace');

	onMount(() => {
		// Get returnUrl from query params
		const urlReturnUrl = $page.url.searchParams.get('returnUrl');
		if (urlReturnUrl) {
			returnUrl = urlReturnUrl;
		}

		// Check if already authenticated on page load only
		// Don't subscribe to changes, as we want the auth flow to show the success screen
		const currentlyAuthenticated = get(authStore.isAuthenticated);
		if (currentlyAuthenticated) {
			// Already logged in when arriving at page, redirect immediately
			goto(returnUrl);
		}
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
			<AuthFlow onSuccess={handleSuccess} />
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
