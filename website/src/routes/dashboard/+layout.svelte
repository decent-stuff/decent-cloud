<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { browser } from '$app/environment';
	import { page } from '$app/stores';
	import { authStore } from '$lib/stores/auth';
	import type { AccountInfo, IdentityInfo } from '$lib/stores/auth';
	import DashboardSidebar from '$lib/components/DashboardSidebar.svelte';
	import AuthPromptBanner from '$lib/components/AuthPromptBanner.svelte';
	import EmailVerificationBanner from '$lib/components/EmailVerificationBanner.svelte';
	import SeedPhraseBackupBanner from '$lib/components/SeedPhraseBackupBanner.svelte';
	import CommandPalette from '$lib/components/CommandPalette.svelte';
	import NotificationBell from '$lib/components/NotificationBell.svelte';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import Icon from '$lib/components/Icons.svelte';

	const SEED_BACKUP_DISMISSED_KEY = 'seedPhraseBackupDismissed';

	let { children } = $props();
	let isAuthenticated = $state(false);
	let isInitialized = $state(false);
	let isSidebarOpen = $state(false);
	let commandPalette = $state<{ openPalette: () => void } | null>(null);
	let account = $state<AccountInfo | null>(null);
	let activeIdentity = $state<IdentityInfo | null>(null);
	let seedBackupDismissed = $state(browser ? localStorage.getItem(SEED_BACKUP_DISMISSED_KEY) === '1' : true);
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
			activeIdentity = identity;
			seedBackupDismissed = browser ? localStorage.getItem(SEED_BACKUP_DISMISSED_KEY) === '1' : true;
		});
	});

	onDestroy(() => {
		unsubscribe?.();
		unsubscribeIdentity?.();
	});

	function toggleSidebar() {
		isSidebarOpen = !isSidebarOpen;
	}

	const isCheckoutOrMarketplacePage = $derived(
		$page.url.pathname.startsWith('/dashboard/marketplace') ||
		$page.url.pathname.startsWith('/dashboard/rentals')
	);

	const showEmailVerificationBanner = $derived(
		isAuthenticated && account && !account.emailVerified && !isCheckoutOrMarketplacePage
	);
	const showSeedPhraseBackupBanner = $derived(
		isAuthenticated && !showEmailVerificationBanner && activeIdentity?.type === 'seedPhrase' && !seedBackupDismissed
	);
</script>

<div class="min-h-screen bg-base">
	<!-- Command Palette -->
	<CommandPalette bind:this={commandPalette} />

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
		<span class="ml-3 text-white font-semibold text-sm flex-1">Decent Cloud</span>
		<ThemeToggle />
		<NotificationBell />
		<button
			type="button"
			onclick={() => commandPalette?.openPalette()}
			class="text-neutral-400 p-2 hover:bg-surface-hover hover:text-white transition-colors"
			aria-label="Open command palette"
		>
			<Icon name="search" size={20} />
		</button>
	</header>

	<!-- Auth prompt banner for anonymous users -->
	{#if !isAuthenticated}
		<AuthPromptBanner />
	{:else if showEmailVerificationBanner}
		<EmailVerificationBanner />
	{:else if showSeedPhraseBackupBanner}
		<SeedPhraseBackupBanner onDismiss={() => { seedBackupDismissed = true; if (browser) localStorage.setItem(SEED_BACKUP_DISMISSED_KEY, '1'); }} />
	{/if}

	<!-- Main content area -->
	<main class="md:ml-60 p-4 md:p-6 pt-18 md:pt-6 {showEmailVerificationBanner ? 'pt-44 md:pt-20' : ''} {showSeedPhraseBackupBanner ? 'pt-28 md:pt-14' : ''} {!isAuthenticated ? 'md:pt-20' : ''}">
		<div class="max-w-6xl mx-auto">
			{@render children()}
		</div>
	</main>
</div>
