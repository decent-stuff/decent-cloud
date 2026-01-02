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

<header class="fixed top-0 left-0 right-0 z-50 bg-base/95 backdrop-blur-lg border-b border-neutral-800/80">
	<div class="max-w-7xl mx-auto px-6 h-14 flex items-center justify-between">
		<!-- Logo -->
		<a href="/" class="group flex items-center gap-2">
			<span class="text-lg font-bold text-white tracking-tight group-hover:text-primary-400 transition-colors">
				Decent Cloud
			</span>
		</a>

		<!-- Actions -->
		<div class="flex items-center gap-3">
			{#if isAuthenticated}
				{#if currentIdentity?.account}
					<a
						href="/dashboard/account"
						class="px-3 py-1.5 text-neutral-400 hover:text-white text-sm font-medium transition-colors"
						title="Account Settings"
					>
						@{currentIdentity.account.username}
					</a>
				{:else if currentIdentity?.displayName}
					<span class="px-3 py-1.5 text-neutral-500 text-sm">
						{currentIdentity.displayName}
					</span>
				{:else if currentIdentity?.principal}
					<span class="px-3 py-1.5 text-neutral-600 text-xs font-mono" title={currentIdentity.principal.toString()}>
						{truncate(currentIdentity.principal.toString())}
					</span>
				{/if}
				<a
					href="/dashboard/marketplace"
					class="inline-flex items-center gap-2 px-4 py-2 bg-primary-500 text-neutral-900 text-sm font-semibold hover:bg-primary-400 transition-colors"
				>
					<span>Dashboard</span>
					<Icon name="arrow-right" size={20} />
				</a>
			{:else}
				<a
					href="/dashboard/marketplace"
					class="px-3 py-1.5 text-neutral-400 hover:text-white text-sm font-medium transition-colors"
				>
					Explore
				</a>
				<button
					type="button"
					onclick={handleConnect}
					class="inline-flex items-center gap-2 px-4 py-2 bg-primary-500 text-neutral-900 text-sm font-semibold hover:bg-primary-400 transition-colors"
				>
					<span>Sign In</span>
				</button>
			{/if}
		</div>
	</div>
</header>
