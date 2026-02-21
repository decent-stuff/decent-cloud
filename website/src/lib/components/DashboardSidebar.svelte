<script lang="ts">
	import { page } from '$app/stores';
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
		{ href: '/dashboard/rentals', icon: 'file', label: 'My Rentals' },
		{ href: '/dashboard/transfers', icon: 'activity', label: 'Transfers' },
		{ href: '/dashboard/invoices', icon: 'download', label: 'Invoices' }
	];

	// Cloud section - self-provisioning
	const cloudItems: NavItem[] = [
		{ href: '/dashboard/cloud/accounts', icon: 'key', label: 'Cloud Accounts' },
		{ href: '/dashboard/cloud/resources', icon: 'server', label: 'Cloud Resources' }
	];

	// Provider section - for users who provide services
	const providerSetupItem: NavItem = {
		href: '/dashboard/provider/support',
		icon: 'settings',
		label: 'Provider Setup'
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
		window.removeEventListener('provider-data-updated', handleProviderDataUpdate);
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
		<div class="pb-2 px-3 pt-1">
			<div class="section-label">Browse</div>
		</div>
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

		{#if isAuthenticated}
			<!-- My Activity section -->
			<div class="pt-5 pb-2 px-3">
				<div class="section-label">My Activity</div>
			</div>
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

			<!-- Cloud section -->
			<div class="pt-5 pb-2 px-3">
				<div class="section-label">My Cloud</div>
			</div>
			{#each cloudItems as item}
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

			<!-- Provider section -->
			<div class="pt-5 pb-2 px-3">
				<div class="section-label flex items-center gap-2">
					Provider
					{#if providerDataError}
						<span class="status-dot status-dot-danger" title="Failed to load provider data"></span>
					{/if}
				</div>
			</div>

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

			<!-- Provider items: always visible, locked until onboarding is complete -->
			{@const providerLocked = !onboardingCompleted}
			{@const lockedClass = providerLocked ? 'opacity-40 pointer-events-none cursor-not-allowed' : ''}
			{@const lockedTitle = providerLocked ? 'Complete Provider Setup to unlock' : undefined}

			<!-- My Offerings -->
			{@const offeringsActive =
				currentPath === '/dashboard/offerings' ||
				currentPath.startsWith('/dashboard/offerings')}
			<a
				href="/dashboard/offerings"
				onclick={closeSidebar}
				class="nav-item {offeringsActive ? 'nav-item-active' : ''} {lockedClass}"
				title={lockedTitle}
			>
				<Icon name="package" size={20} />
				<span class="text-sm">My Offerings</span>
				{#if providerLocked}<Icon name="lock" size={14} class="ml-auto text-neutral-500" />{/if}
			</a>

			<!-- Earnings -->
			{@const earningsActive =
				currentPath === '/dashboard/provider/earnings' ||
				currentPath.startsWith('/dashboard/provider/earnings')}
			<a
				href="/dashboard/provider/earnings"
				onclick={closeSidebar}
				class="nav-item {earningsActive ? 'nav-item-active' : ''} {lockedClass}"
				title={lockedTitle}
			>
				<Icon name="trending-up" size={20} />
				<span class="text-sm">Earnings</span>
				{#if providerLocked}<Icon name="lock" size={14} class="ml-auto text-neutral-500" />{/if}
			</a>

			<!-- SLA Monitor -->
			{@const slaActive =
				currentPath === '/dashboard/provider/sla' ||
				currentPath.startsWith('/dashboard/provider/sla')}
			<a
				href="/dashboard/provider/sla"
				onclick={closeSidebar}
				class="nav-item {slaActive ? 'nav-item-active' : ''} {lockedClass}"
				title={lockedTitle}
			>
				<Icon name="shield" size={20} />
				<span class="text-sm">SLA Monitor</span>
				{#if providerLocked}<Icon name="lock" size={14} class="ml-auto text-neutral-500" />{/if}
			</a>

			{#each providerOnboardedItems as item}
				{@const isActive = currentPath === item.href || currentPath.startsWith(item.href)}
				<a
					href={item.href}
					onclick={closeSidebar}
					class="nav-item {isActive ? 'nav-item-active' : ''} {lockedClass}"
					title={lockedTitle}
				>
					<Icon name={item.icon} size={20} />
					<span class="text-sm">{item.label}</span>
					{#if item.href === '/dashboard/provider/requests' && !providerLocked}
						<span class="ml-auto"><UnreadBadge count={pendingRequestsCount} /></span>
					{:else if item.href === '/dashboard/provider/password-resets' && !providerLocked}
						<span class="ml-auto"><UnreadBadge count={pendingPasswordResetsCount} /></span>
					{:else if providerLocked}
						<Icon name="lock" size={14} class="ml-auto text-neutral-500" />
					{/if}
				</a>
			{/each}

			{#if CHATWOOT_BASE_URL}
				<a
					href={supportDashboardUrl}
					target="_blank"
					rel="noopener noreferrer"
					onclick={closeSidebar}
					class="nav-item {lockedClass}"
					title={providerLocked ? lockedTitle : 'Open Chatwoot support dashboard'}
				>
					<Icon name="headphones" size={20} />
					<span class="text-sm">Support Dashboard</span>
					{#if providerLocked}
						<Icon name="lock" size={14} class="ml-auto text-neutral-500" />
					{:else}
						<Icon name="external" size={20} class="ml-auto text-neutral-600" />
					{/if}
				</a>
			{/if}
		{/if}

		{#if isAdmin}
			<!-- Admin section -->
			<div class="pt-5 pb-2 px-3">
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
