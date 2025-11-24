<script lang="ts">
	import { authStore } from '$lib/stores/auth';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import AuthDialog from './AuthDialog.svelte';
	import { onMount, onDestroy } from 'svelte';
	import type { IdentityInfo } from '$lib/stores/auth';

	let isAuthenticated = $state(false);
	let currentIdentity = $state<IdentityInfo | null>(null);
	let showAuthDialog = $state(false);
	let returnUrl = $state<string | null>(null);
	let unsubscribeAuth: (() => void) | null = null;
	let unsubscribeIdentity: (() => void) | null = null;
	let unsubscribePage: (() => void) | null = null;

	onMount(() => {
		unsubscribeAuth = authStore.isAuthenticated.subscribe((value) => {
			isAuthenticated = value;
			// If user just authenticated and there's a returnUrl, navigate to it
			if (value && returnUrl) {
				const url = returnUrl;
				returnUrl = null;
				goto(url);
			}
		});
		unsubscribeIdentity = authStore.currentIdentity.subscribe((value) => {
			currentIdentity = value;
		});
		unsubscribePage = page.subscribe(($page) => {
			const action = $page.url.searchParams.get('action');
			const urlReturnUrl = $page.url.searchParams.get('returnUrl');

			// Store returnUrl for post-auth navigation
			if (urlReturnUrl) {
				returnUrl = urlReturnUrl;
			}

			// Auto-open auth dialog if action parameter is present
			if ((action === 'signup' || action === 'login') && !isAuthenticated) {
				showAuthDialog = true;
			}
		});
	});

	onDestroy(() => {
		unsubscribeAuth?.();
		unsubscribeIdentity?.();
		unsubscribePage?.();
	});

	function handleConnect() {
		showAuthDialog = true;
	}

	function truncate(str: string): string {
		if (str.length <= 12) return str;
		return `${str.slice(0, 6)}...${str.slice(-4)}`;
	}
</script>

<header
	class="fixed top-0 left-0 right-0 z-50 bg-black/20 backdrop-blur-md border-b border-white/10"
>
	<div class="max-w-7xl mx-auto px-6 py-4 flex items-center justify-between">
		<!-- Logo -->
		<a href="/" class="text-2xl font-bold text-white hover:text-blue-400 transition-colors">
			Decent Cloud
		</a>

		<!-- Actions -->
		<div class="flex items-center gap-4">
			{#if isAuthenticated}
				{#if currentIdentity?.account}
					<a
						href="/dashboard/account"
						class="text-white/70 hover:text-white transition-colors"
						title="Account Settings"
					>
						<span class="font-medium">@{currentIdentity.account.username}</span>
					</a>
				{:else if currentIdentity?.displayName}
					<span class="text-white/70 text-sm">
						{currentIdentity.displayName}
					</span>
				{:else if currentIdentity?.principal}
					<span class="text-white/70 text-sm font-mono" title={currentIdentity.principal.toString()}>
						{truncate(currentIdentity.principal.toString())}
					</span>
				{/if}
				<a
					href="/dashboard"
					class="px-6 py-2 bg-gradient-to-r from-blue-500 to-purple-600 rounded-full font-semibold hover:brightness-110 hover:scale-105 transition-all"
				>
					Dashboard →
				</a>
			{:else}
				<a
					href="/dashboard"
					class="px-6 py-2 text-white/70 hover:text-white transition-colors font-medium"
				>
					Dashboard
				</a>
				<button
					type="button"
					onclick={handleConnect}
					class="px-6 py-2 bg-gradient-to-r from-blue-500 to-purple-600 rounded-full font-semibold hover:brightness-110 hover:scale-105 transition-all"
				>
					Connect Wallet
				</button>
			{/if}
		</div>
	</div>
</header>

<AuthDialog bind:open={showAuthDialog} />
