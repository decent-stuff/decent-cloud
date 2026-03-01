<script lang="ts">
	import { page } from '$app/stores';
	import { browser } from '$app/environment';
	import { authStore } from '$lib/stores/auth';
	import { navigateToLogin } from '$lib/utils/navigation';
	import { onMount, onDestroy } from 'svelte';
	import type { IdentityInfo } from '$lib/stores/auth';
	import {
		getProviderOfferings,
		getProviderOnboarding,
		getPendingProviderRequests,
		getPendingPasswordResets,
		hexEncode
	} from '$lib/services/api';
	import type { ProviderOnboarding } from '$lib/services/api';
	import { signRequest } from '$lib/services/auth-api';
	import { Ed25519KeyIdentity } from '@dfinity/identity';
	import Icon from './Icons.svelte';
	import type { IconName } from './Icons.svelte';
	import UnreadBadge from './UnreadBadge.svelte';

	let { isOpen = $bindable(false), isAuthenticated = false } = $props();

	let currentPath = $state('');
	let currentIdentity = $state<IdentityInfo | null>(null);
	let unsubscribeIdentity: (() => void) | null = null;
	let offeringsCount = $state(0);
	let onboardingData = $state<ProviderOnboarding | null>(null);
	let providerDataLoading = $state(false);
	let providerDataError = $state(false);
	let pendingRequestsCount = $state(0);
	let pendingPasswordResetsCount = $state(0);

	// Section keys and their default collapsed state (false = expanded by default)
	type SectionKey = 'discover' | 'activity' | 'provider';
	const SECTION_DEFAULTS: Record<SectionKey, boolean> = {
		discover: false,
		activity: false,
		provider: true
	};

	let sectionCollapsed = $state<Record<SectionKey, boolean>>({ ...SECTION_DEFAULTS });

	function loadSectionState(key: SectionKey): boolean {
		const stored = localStorage.getItem(`sidebar_section_${key}`);
		if (stored === null) return SECTION_DEFAULTS[key];
		return stored === 'true';
	}

	function toggleSection(key: SectionKey) {
		const next = !sectionCollapsed[key];
		sectionCollapsed[key] = next;
		localStorage.setItem(`sidebar_section_${key}`, String(next));
	}

	// Auto-expand provider section when the user has offerings
	$effect(() => {
		if (offeringsCount > 0) {
			const stored = localStorage.getItem('sidebar_section_provider');
			if (stored === null || stored === 'true') {
				sectionCollapsed.provider = false;
				localStorage.setItem('sidebar_section_provider', 'false');
			}
		}
	});

	const CHATWOOT_BASE_URL =
		import.meta.env.VITE_CHATWOOT_BASE_URL || 'https://support.decent-cloud.org';
	const CHATWOOT_ACCOUNT_ID = import.meta.env.VITE_CHATWOOT_ACCOUNT_ID || '1';
	const supportDashboardUrl = `${CHATWOOT_BASE_URL}/app/accounts/${CHATWOOT_ACCOUNT_ID}/dashboard`;

	interface NavItem {
		href: string;
		icon: IconName;
		label: string;
	}

	// Browse section - discovery and exploration
	const browseItems: NavItem[] = [
		{ href: '/dashboard/marketplace', icon: 'cart', label: 'Marketplace' },
		{ href: '/dashboard/reputation', icon: 'star', label: 'Reputation' },
		{ href: '/dashboard/validators', icon: 'check', label: 'Validators' }
	];

	// My Activity section - user's rentals (customer perspective)
	const activityItems: NavItem[] = [
		{ href: '/dashboard/saved', icon: 'bookmark', label: 'Saved' },
		{ href: '/dashboard/rentals', icon: 'file', label: 'My Rentals' }
	];

	// Provider section - for users who provide services
	const providerSetupItem: NavItem = {
		href: '/dashboard/provider/support',
		icon: 'settings',
		label: 'Support Account'
	};

	// Items locked until onboarding is complete
	const providerOnboardedItems: NavItem[] = [
		{ href: '/dashboard/provider/requests', icon: 'inbox', label: 'Rental Requests' },
		{ href: '/dashboard/provider/feedback', icon: 'star', label: 'Tenant Feedback' },
		{ href: '/dashboard/provider/password-resets', icon: 'key', label: 'Password Resets' },
		{ href: '/dashboard/provider/agents', icon: 'bot', label: 'Agents' },
		{ href: '/dashboard/provider/reseller', icon: 'briefcase', label: 'Reseller' }
	];

	const isAdmin = $derived(currentIdentity?.account?.isAdmin ?? false);
	const hasOfferings = $derived(offeringsCount > 0);
	const onboardingCompleted = $derived(onboardingData?.onboarding_completed_at !== undefined);

	page.subscribe((p) => {
		currentPath = p.url.pathname;
	});

	async function loadProviderData() {
		if (!currentIdentity?.publicKeyBytes || providerDataLoading) {
			return;
		}

		try {
			providerDataLoading = true;
			providerDataError = false;
			const pubkeyHex = hexEncode(currentIdentity.publicKeyBytes);

			const [offerings, onboarding] = await Promise.all([
				getProviderOfferings(pubkeyHex),
				getProviderOnboarding(pubkeyHex).catch(() => null)
			]);

			offeringsCount = offerings.length;
			onboardingData = onboarding;

			if (onboarding?.onboarding_completed_at) {
				try {
					const info = await authStore.getSigningIdentity();
					if (info?.identity instanceof Ed25519KeyIdentity) {
						const pubkeyHexSigning = hexEncode(info.publicKeyBytes);
						const [signedRequests, signedResets] = await Promise.all([
							signRequest(info.identity, 'GET', '/api/v1/provider/rental-requests/pending'),
							signRequest(
								info.identity,
								'GET',
								`/api/v1/providers/${pubkeyHexSigning}/contracts/pending-password-reset`
							)
						]);
						const [requests, resets] = await Promise.all([
							getPendingProviderRequests(signedRequests.headers),
							getPendingPasswordResets(pubkeyHexSigning, signedResets.headers)
						]);
						pendingRequestsCount = requests.length;
						pendingPasswordResetsCount = resets.length;
					}
				} catch {
					// keep counts at 0 - don't break sidebar on error
				}
			}
		} catch (err) {
			console.error('Failed to load provider data:', err);
			providerDataError = true;
			offeringsCount = 0;
			onboardingData = null;
		} finally {
			providerDataLoading = false;
		}
	}

	function handleProviderDataUpdate() {
		loadProviderData();
	}

	onMount(() => {
		// Restore section collapse state from localStorage
		for (const key of Object.keys(SECTION_DEFAULTS) as SectionKey[]) {
			sectionCollapsed[key] = loadSectionState(key);
		}

		unsubscribeIdentity = authStore.currentIdentity.subscribe((value) => {
			currentIdentity = value;
			if (value?.publicKeyBytes) {
				loadProviderData();
			} else {
				offeringsCount = 0;
				onboardingData = null;
				pendingRequestsCount = 0;
				pendingPasswordResetsCount = 0;
			}
		});
		// Listen for provider data updates from other components
		window.addEventListener('provider-data-updated', handleProviderDataUpdate);
	});

	onDestroy(() => {
		unsubscribeIdentity?.();
		if (browser) window.removeEventListener('provider-data-updated', handleProviderDataUpdate);
	});

	async function handleLogout() {
		await authStore.logout();
		window.location.href = '/';
	}

	function handleLogin() {
		closeSidebar();
		navigateToLogin(currentPath);
	}

	function closeSidebar() {
		isOpen = false;
	}
</script>

<!-- Mobile overlay -->
{#if isOpen}
	<button
		type="button"
		class="fixed inset-0 bg-base/90 backdrop-blur-sm z-40 md:hidden"
		onclick={closeSidebar}
		aria-label="Close sidebar"
	></button>
{/if}

<aside
	class="fixed left-0 top-0 h-screen w-60 bg-surface border-r border-neutral-800/80 flex flex-col z-50 transition-transform duration-200 {isOpen
		? 'translate-x-0'
		: '-translate-x-full md:translate-x-0'}"
>
	<!-- Logo -->
	<div class="h-14 px-5 flex items-center border-b border-neutral-800/80">
		<a href="/" class="text-base font-bold text-white hover:text-primary-400 transition-colors tracking-tight">
			Decent Cloud
		</a>
	</div>

	<!-- Navigation -->
	<nav class="flex-1 px-3 py-4 space-y-0.5 overflow-y-auto">
		<!-- Browse section -->
		<button
			type="button"
			class="section-toggle"
			onclick={() => toggleSection('discover')}
			aria-expanded={!sectionCollapsed.discover}
		>
			<span class="section-label">Browse</span>
			<Icon name="chevron-down" size={14} class="ml-auto text-neutral-500 transition-transform {sectionCollapsed.discover ? '-rotate-90' : ''}" />
		</button>
		{#if !sectionCollapsed.discover}
			{#each browseItems as item}
				{@const isActive =
					currentPath === item.href ||
					(item.label === 'Reputation' && currentPath.startsWith('/dashboard/reputation'))}
				<a
					href={item.href}
					onclick={closeSidebar}
					class="nav-item {isActive ? 'nav-item-active' : ''}"
				>
					<Icon name={item.icon} size={20} />
					<span class="text-sm">{item.label}</span>
				</a>
			{/each}
		{/if}

		<!-- My Activity section - show to all users -->
		<button
			type="button"
			class="section-toggle mt-3"
			onclick={() => toggleSection('activity')}
			aria-expanded={!sectionCollapsed.activity}
		>
			<span class="section-label">My Activity</span>
			<Icon name="chevron-down" size={14} class="ml-auto text-neutral-500 transition-transform {sectionCollapsed.activity ? '-rotate-90' : ''}" />
		</button>
		{#if !sectionCollapsed.activity}
			{#if isAuthenticated}
				{#each activityItems as item}
					{@const isActive = currentPath === item.href || currentPath.startsWith(item.href)}
					<a
						href={item.href}
						onclick={closeSidebar}
						class="nav-item {isActive ? 'nav-item-active' : ''}"
					>
						<Icon name={item.icon} size={20} />
						<span class="text-sm">{item.label}</span>
					</a>
				{/each}
			{:else}
				<div class="px-3 py-2">
					<p class="text-xs text-neutral-500 mb-2">Sign in to access:</p>
					{#each activityItems as item}
						<div class="flex items-center gap-2 py-1 text-neutral-600">
							<Icon name={item.icon} size={18} />
							<span class="text-xs">{item.label}</span>
						</div>
					{/each}
					<button
						type="button"
						onclick={handleLogin}
						class="mt-2 w-full text-center px-3 py-1.5 text-xs bg-primary-500/20 text-primary-400 hover:bg-primary-500/30 transition-colors rounded"
					>
						Sign In
					</button>
				</div>
			{/if}
		{/if}

		{#if isAuthenticated}
			<!-- Provider section -->
			<button
				type="button"
				class="section-toggle mt-3"
				onclick={() => toggleSection('provider')}
				aria-expanded={!sectionCollapsed.provider}
			>
				<span class="section-label flex items-center gap-2">
					Provider
					{#if providerDataError}
						<span class="status-dot status-dot-danger" title="Failed to load provider data"></span>
					{/if}
				</span>
				<Icon name="chevron-down" size={14} class="ml-auto text-neutral-500 transition-transform {sectionCollapsed.provider ? '-rotate-90' : ''}" />
			</button>
			{#if !sectionCollapsed.provider}
				<!-- Provider Setup -->
				{@const setupActive =
					currentPath === providerSetupItem.href ||
					currentPath.startsWith(providerSetupItem.href)}
				<a
					href={providerSetupItem.href}
					onclick={closeSidebar}
					class="nav-item {setupActive ? 'nav-item-active' : ''}"
				>
					<Icon name={providerSetupItem.icon} size={20} />
					<span class="text-sm">{providerSetupItem.label}</span>
					{#if hasOfferings && !onboardingCompleted}
						<span class="ml-auto status-dot status-dot-warning" title="Setup incomplete"></span>
					{/if}
				</a>

				<!-- My Offerings - always visible -->
				{@const offeringsActive =
					currentPath === '/dashboard/offerings' ||
					currentPath.startsWith('/dashboard/offerings')}
				<a
					href="/dashboard/offerings"
					onclick={closeSidebar}
					class="nav-item {offeringsActive ? 'nav-item-active' : ''}"
				>
					<Icon name="package" size={20} />
					<span class="text-sm">My Offerings</span>
				</a>

				<!-- Provider items: only show when onboarding is complete -->
				{#if onboardingCompleted}
					<!-- Earnings -->
					{@const earningsActive =
						currentPath === '/dashboard/provider/earnings' ||
						currentPath.startsWith('/dashboard/provider/earnings')}
					<a
						href="/dashboard/provider/earnings"
						onclick={closeSidebar}
						class="nav-item {earningsActive ? 'nav-item-active' : ''}"
					>
						<Icon name="trending-up" size={20} />
						<span class="text-sm">Earnings</span>
					</a>

					<!-- Analytics -->
					{@const analyticsActive =
						currentPath === '/dashboard/provider/analytics' ||
						currentPath.startsWith('/dashboard/provider/analytics')}
					<a
						href="/dashboard/provider/analytics"
						onclick={closeSidebar}
						class="nav-item {analyticsActive ? 'nav-item-active' : ''}"
					>
						<Icon name="chart" size={20} />
						<span class="text-sm">Analytics</span>
					</a>

					<!-- SLA Monitor -->
					{@const slaActive =
						currentPath === '/dashboard/provider/sla' ||
						currentPath.startsWith('/dashboard/provider/sla')}
					<a
						href="/dashboard/provider/sla"
						onclick={closeSidebar}
						class="nav-item {slaActive ? 'nav-item-active' : ''}"
					>
						<Icon name="shield" size={20} />
						<span class="text-sm">SLA Monitor</span>
					</a>

					{#each providerOnboardedItems as item}
						{@const isActive = currentPath === item.href || currentPath.startsWith(item.href)}
						<a
							href={item.href}
							onclick={closeSidebar}
							class="nav-item {isActive ? 'nav-item-active' : ''}"
						>
							<Icon name={item.icon} size={20} />
							<span class="text-sm">{item.label}</span>
							{#if item.href === '/dashboard/provider/requests'}
								<span class="ml-auto"><UnreadBadge count={pendingRequestsCount} /></span>
							{:else if item.href === '/dashboard/provider/password-resets'}
								<span class="ml-auto"><UnreadBadge count={pendingPasswordResetsCount} /></span>
							{/if}
						</a>
					{/each}

					{#if CHATWOOT_BASE_URL}
						<a
							href={supportDashboardUrl}
							target="_blank"
							rel="noopener noreferrer"
							onclick={closeSidebar}
							class="nav-item"
							title="Open support dashboard"
						>
							<Icon name="headphones" size={20} />
							<span class="text-sm">Support Dashboard</span>
							<Icon name="external" size={20} class="ml-auto text-neutral-600" />
						</a>
					{/if}
				{/if}
			{/if}
		{/if}

		{#if isAdmin}
			<!-- Admin section -->
			<div class="pt-3 pb-2 px-3">
				<div class="section-label">Admin</div>
			</div>
			<a
				href="/dashboard/admin"
				onclick={closeSidebar}
				class="nav-item {currentPath.startsWith('/dashboard/admin') ? 'nav-item-active' : ''}"
			>
				<Icon name="wrench" size={20} />
				<span class="text-sm">Admin</span>
			</a>
		{/if}
	</nav>

	<!-- User Section -->
	<div class="p-3 border-t border-neutral-800/80 space-y-1">
		{#if isAuthenticated}
			<a
				href="/dashboard/account"
				onclick={closeSidebar}
				class="nav-item {currentPath.startsWith('/dashboard/account') ? 'nav-item-active' : ''}"
			>
				<Icon name="user" size={20} />
				<span class="text-sm">Account</span>
			</a>
			<button
				type="button"
				onclick={handleLogout}
				class="nav-item w-full"
			>
				<Icon name="logout" size={20} />
				<span class="text-sm">Logout</span>
			</button>
		{:else}
			<button
				type="button"
				onclick={handleLogin}
				class="w-full flex items-center justify-center gap-2 px-4 py-2 bg-primary-500 text-neutral-900 text-sm font-semibold hover:bg-primary-400 transition-colors"
			>
				<Icon name="login" size={20} />
				<span>Sign In</span>
			</button>
		{/if}
	</div>
</aside>
