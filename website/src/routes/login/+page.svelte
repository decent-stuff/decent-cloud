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
		const urlReturnUrl = $page.url.searchParams.get('returnUrl');
		if (urlReturnUrl) {
			returnUrl = urlReturnUrl;
		}

		const currentlyAuthenticated = get(authStore.isAuthenticated);
		if (currentlyAuthenticated) {
			goto(returnUrl);
		}
	});

	function handleSuccess(account: AccountInfo) {
		console.log('Auth success:', account);
		goto(returnUrl);
	}

	function handleCancel() {
		goto('/');
	}
</script>

<svelte:head>
	<title>Login - Decent Cloud</title>
</svelte:head>

<div class="min-h-screen bg-base bg-grid bg-radial flex items-center justify-center p-4">
	<div class="w-full max-w-lg">
		<!-- Header -->
		<div class="text-center mb-8">
			<a href="/" class="inline-block">
				<h1 class="text-2xl font-bold text-white tracking-tight mb-2">Decent Cloud</h1>
			</a>
			<p class="text-neutral-500 text-sm">Login or create your account</p>
		</div>

		<!-- Auth Flow Card -->
		<div class="card p-6 md:p-8">
			<AuthFlow onSuccess={handleSuccess} />
		</div>

		<!-- Footer links -->
		<div class="text-center mt-6 space-y-2">
			<div>
				<a href="/recover" class="text-neutral-600 hover:text-neutral-400 transition-colors text-xs">
					Lost access? Recover your account
				</a>
			</div>
			<div>
				<button
					type="button"
					onclick={handleCancel}
					class="text-neutral-500 hover:text-white transition-colors text-sm"
				>
					← Back to home
				</button>
			</div>
		</div>
	</div>
</div>
