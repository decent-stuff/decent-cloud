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
	import KeyboardHelpOverlay from '$lib/components/KeyboardHelpOverlay.svelte';
	import NotificationBell from '$lib/components/NotificationBell.svelte';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import Icon from '$lib/components/Icons.svelte';

	const SEED_BACKUP_DISMISSED_KEY = 'seedPhraseBackupDismissed';
	const EMAIL_BANNER_DISMISSED_KEY = 'emailVerificationBannerDismissed';

	let { children } = $props();
	let isAuthenticated = $state(false);
	let isInitialized = $state(false);
	let isSidebarOpen = $state(false);
	let commandPalette = $state<{ openPalette: () => void } | null>(null);
	let showKeyboardHelp = $state(false);
	let account = $state<AccountInfo | null>(null);
	let activeIdentity = $state<IdentityInfo | null>(null);
	let seedBackupDismissed = $state(browser ? localStorage.getItem(SEED_BACKUP_DISMISSED_KEY) === '1' : true);
	let emailBannerDismissed = $state(browser ? sessionStorage.getItem(EMAIL_BANNER_DISMISSED_KEY) === '1' : false);
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

	// True when the user is actively typing somewhere we must not hijack. Covers
	// INPUT/TEXTAREA/SELECT and contentEditable elements. Mirrors the marketplace
	// '/' guard (+page.svelte) but extended to the full set the overlay must avoid.
	function isTypingTarget(el: Element | null): boolean {
		if (!el) return false;
		const tag = el.tagName;
		return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' ||
			(el as HTMLElement).isContentEditable;
	}

	// '?' toggles the keyboard help overlay; Escape closes it. Both guards keep
	// the help from firing while typing (e.g. '?' in the search box inserts the
	// character instead of opening help).
	function handleGlobalKeydown(e: KeyboardEvent) {
		if (e.key === '?' && !isTypingTarget(document.activeElement)) {
			e.preventDefault();
			showKeyboardHelp = !showKeyboardHelp;
		} else if (e.key === 'Escape' && showKeyboardHelp) {
			showKeyboardHelp = false;
		}
	}

	// F4: confine the full-width verify-email + seed-backup banners to the
	// dashboard landing and the account pages. They are actionable there (verify
	// email / back up seed) but consumed vertical space and nagged on every
	// /dashboard/* sub-route. Contextual notices now surface the prerequisite
	// where it blocks action (e.g. the F3 rent-flow gate), so the global banner
	// no longer needs to dominate every screen.
	const isBannerSurface = $derived(
		$page.url.pathname === '/dashboard' ||
		$page.url.pathname.startsWith('/dashboard/account')
	);

	const showEmailVerificationBanner = $derived(
		isAuthenticated && account && !account.emailVerified && isBannerSurface && !emailBannerDismissed
	);
	// #438: previously gated on `!showEmailVerificationBanner`, which meant a
	// seed-phrase user with an unverified email NEVER saw the backup warning —
	// exactly the high-risk account that needs it most. The two banners now
	// stack independently inside a single fixed container; each has its own
	// dismissal path (seed banner via localStorage flag, email banner via
	// verification). Both are confined to the banner surfaces (F4).
	const showSeedPhraseBackupBanner = $derived(
		isAuthenticated && isBannerSurface && activeIdentity?.type === 'seedPhrase' && !seedBackupDismissed
	);

	// <main> top padding must clear the fixed banner stack. Mobile sits under
	// the h-14 (3.5rem) header; desktop starts at top-0. Each banner adds its
	// own height; the both-banners case is the new path added by #438.
	const mainTopPadding = $derived(
		!isAuthenticated
			? 'md:pt-20'
			: showEmailVerificationBanner && showSeedPhraseBackupBanner
				? 'pt-56 md:pt-36'
				: showEmailVerificationBanner
					? 'pt-44 md:pt-20'
					: showSeedPhraseBackupBanner
						? 'pt-28 md:pt-14'
						: ''
	);

	function dismissSeedPhraseBanner() {
		seedBackupDismissed = true;
		if (browser) localStorage.setItem(SEED_BACKUP_DISMISSED_KEY, '1');
	}

	function dismissEmailBanner() {
		emailBannerDismissed = true;
		if (browser) sessionStorage.setItem(EMAIL_BANNER_DISMISSED_KEY, '1');
	}
</script>

<svelte:window onkeydown={handleGlobalKeydown} />

<div class="min-h-screen bg-base">
	<!-- Command Palette -->
	<CommandPalette bind:this={commandPalette} />

	<!-- Keyboard shortcut help overlay (? key) -->
	<KeyboardHelpOverlay bind:open={showKeyboardHelp} />

	<!-- Sidebar -->
	<DashboardSidebar bind:isOpen={isSidebarOpen} {isAuthenticated} openPalette={() => commandPalette?.openPalette()} openKeyboardHelp={() => { showKeyboardHelp = true; }} />

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
	{:else if showEmailVerificationBanner || showSeedPhraseBackupBanner}
		<!-- #438: banners stack independently inside one fixed container so they
		can coexist (seed-phrase backup + unverified email). Each renders as a
		static block sibling; the container owns positioning + z-index. -->
		<div class="fixed top-14 md:top-0 left-0 md:left-60 right-0 z-40">
		{#if showEmailVerificationBanner}
			<EmailVerificationBanner onDismiss={dismissEmailBanner} />
		{/if}
			{#if showSeedPhraseBackupBanner}
				<SeedPhraseBackupBanner onDismiss={dismissSeedPhraseBanner} />
			{/if}
		</div>
	{/if}

	<!-- Main content area -->
	<main class="md:ml-60 p-4 md:p-6 pt-18 md:pt-6 {mainTopPadding}">
		<div class="max-w-6xl mx-auto">
			{@render children()}
		</div>
	</main>
</div>
