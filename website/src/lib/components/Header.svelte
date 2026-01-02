<script lang="ts">
	import { authStore } from '$lib/stores/auth';
	import { onMount, onDestroy } from 'svelte';
	import { navigateToLogin } from '$lib/utils/navigation';
	import type { IdentityInfo } from '$lib/stores/auth';
	import Icon from './Icons.svelte';

	let isAuthenticated = $state(false);
	let currentIdentity = $state<IdentityInfo | null>(null);
	let unsubscribeAuth: (() => void) | null = null;
	let unsubscribeIdentity: (() => void) | null = null;

	onMount(() => {
		unsubscribeAuth = authStore.isAuthenticated.subscribe((value) => {
			isAuthenticated = value;
		});
		unsubscribeIdentity = authStore.currentIdentity.subscribe((value) => {
			currentIdentity = value;
		});
	});

	onDestroy(() => {
		unsubscribeAuth?.();
		unsubscribeIdentity?.();
	});

	function handleConnect() {
		navigateToLogin('/dashboard/marketplace');
	}

	function truncate(str: string): string {
		if (str.length <= 12) return str;
		return `${str.slice(0, 6)}...${str.slice(-4)}`;
	}
</script>

<header class="fixed top-0 left-0 right-0 z-50 bg-base/90 backdrop-blur-md border-b border-neutral-800">
	<div class="max-w-7xl mx-auto px-6 h-16 flex items-center justify-between">
		<!-- Logo -->
		<a href="/" class="text-xl font-bold text-white hover:text-primary-400 transition-colors tracking-tight">
			Decent Cloud
		</a>

		<!-- Actions -->
		<div class="flex items-center gap-4">
			{#if isAuthenticated}
				{#if currentIdentity?.account}
					<a
						href="/dashboard/account"
						class="text-neutral-400 hover:text-white transition-colors text-sm font-medium"
						title="Account Settings"
					>
						@{currentIdentity.account.username}
					</a>
				{:else if currentIdentity?.displayName}
					<span class="text-neutral-400 text-sm">
						{currentIdentity.displayName}
					</span>
				{:else if currentIdentity?.principal}
					<span class="text-neutral-500 text-sm font-mono" title={currentIdentity.principal.toString()}>
						{truncate(currentIdentity.principal.toString())}
					</span>
				{/if}
				<a
					href="/dashboard/marketplace"
					class="inline-flex items-center gap-2 px-5 py-2 bg-primary-500 text-base font-semibold hover:bg-primary-400 transition-colors"
				>
					<span>Dashboard</span>
					<Icon name="arrow-right" size={16} />
				</a>
			{:else}
				<a
					href="/dashboard/marketplace"
					class="text-neutral-400 hover:text-white transition-colors text-sm font-medium"
				>
					Dashboard
				</a>
				<button
					type="button"
					onclick={handleConnect}
					class="inline-flex items-center gap-2 px-5 py-2 bg-primary-500 text-base font-semibold hover:bg-primary-400 transition-colors"
				>
					<span>Sign In</span>
				</button>
			{/if}
		</div>
	</div>
</header>
