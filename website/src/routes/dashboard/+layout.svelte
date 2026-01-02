<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { authStore } from '$lib/stores/auth';
	import type { AccountInfo } from '$lib/stores/auth';
	import DashboardSidebar from '$lib/components/DashboardSidebar.svelte';
	import AuthPromptBanner from '$lib/components/AuthPromptBanner.svelte';
	import EmailVerificationBanner from '$lib/components/EmailVerificationBanner.svelte';
	import Icon from '$lib/components/Icons.svelte';

	let { children } = $props();
	let isAuthenticated = $state(false);
	let isInitialized = $state(false);
	let isSidebarOpen = $state(false);
	let account = $state<AccountInfo | null>(null);
	let unsubscribe: (() => void) | null = null;
	let unsubscribeIdentity: (() => void) | null = null;

	onMount(async () => {
		await authStore.initialize();
		isInitialized = true;

		unsubscribe = authStore.isAuthenticated.subscribe((value) => {
			isAuthenticated = value;
		});

		unsubscribeIdentity = authStore.activeIdentity.subscribe((identity) => {
			account = identity?.account || null;
		});
	});

	onDestroy(() => {
		unsubscribe?.();
		unsubscribeIdentity?.();
	});

	function toggleSidebar() {
		isSidebarOpen = !isSidebarOpen;
	}

	const showEmailVerificationBanner = $derived(isAuthenticated && account && !account.emailVerified);
</script>

<div class="min-h-screen bg-base">
	<!-- Sidebar -->
	<DashboardSidebar bind:isOpen={isSidebarOpen} {isAuthenticated} />

	<!-- Mobile header -->
	<header class="fixed top-0 left-0 right-0 h-14 bg-surface border-b border-neutral-800/80 flex items-center px-4 md:hidden z-30">
		<button
			type="button"
			onclick={toggleSidebar}
			class="text-neutral-400 p-2 hover:bg-surface-hover hover:text-white transition-colors"
			aria-label="Toggle menu"
		>
			<Icon name="menu" size={20} />
		</button>
		<span class="ml-3 text-white font-semibold text-sm">Decent Cloud</span>
	</header>

	<!-- Auth prompt banner for anonymous users -->
	{#if !isAuthenticated}
		<AuthPromptBanner />
	{:else if showEmailVerificationBanner}
		<EmailVerificationBanner />
	{/if}

	<!-- Main content area -->
	<main class="md:ml-60 p-4 md:p-6 pt-18 md:pt-6 {showEmailVerificationBanner ? 'pt-44 md:pt-20' : ''} {!isAuthenticated ? 'md:pt-20' : ''}">
		<div class="max-w-6xl mx-auto">
			{@render children()}
		</div>
	</main>
</div>
