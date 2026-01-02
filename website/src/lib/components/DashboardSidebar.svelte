<script lang="ts">
	import { page } from '$app/stores';
	import { authStore } from '$lib/stores/auth';
	import { navigateToLogin } from '$lib/utils/navigation';
	import { onMount, onDestroy } from 'svelte';
	import type { IdentityInfo } from '$lib/stores/auth';
	import {
		getProviderOfferings,
		getProviderOnboarding,
		hexEncode
	} from '$lib/services/api';
	import type { ProviderOnboarding } from '$lib/services/api';
	import Icon from './Icons.svelte';
	import type { IconName } from './Icons.svelte';

	let { isOpen = $bindable(false), isAuthenticated = false } = $props();

	let currentPath = $state('');
	let currentIdentity = $state<IdentityInfo | null>(null);
	let unsubscribeIdentity: (() => void) | null = null;
	let offeringsCount = $state(0);
	let onboardingData = $state<ProviderOnboarding | null>(null);
	let providerDataLoading = $state(false);

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
		{ href: '/dashboard/rentals', icon: 'file', label: 'My Rentals' }
	];

	// Provider section - for users who provide services
	const providerSetupItem: NavItem = {
		href: '/dashboard/provider/support',
		icon: 'settings',
		label: 'Provider Setup'
	};

	// Items visible only after onboarding is complete
	const providerOnboardedItems: NavItem[] = [
		{ href: '/dashboard/provider/requests', icon: 'inbox', label: 'Rental Requests' },
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
			const pubkeyHex = hexEncode(currentIdentity.publicKeyBytes);

			const [offerings, onboarding] = await Promise.all([
				getProviderOfferings(pubkeyHex).catch(() => []),
				getProviderOnboarding(pubkeyHex).catch(() => null)
			]);

			offeringsCount = offerings.length;
			onboardingData = onboarding;
		} catch (err) {
			console.error('Failed to load provider data:', err);
		} finally {
			providerDataLoading = false;
		}
	}

	onMount(() => {
		unsubscribeIdentity = authStore.currentIdentity.subscribe((value) => {
			currentIdentity = value;
			if (value?.publicKeyBytes) {
				loadProviderData();
			} else {
				offeringsCount = 0;
				onboardingData = null;
			}
		});
	});

	onDestroy(() => {
		unsubscribeIdentity?.();
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
		class="fixed inset-0 bg-base/80 z-40 md:hidden"
		onclick={closeSidebar}
		aria-label="Close sidebar"
	></button>
{/if}

<aside
	class="fixed left-0 top-0 h-screen w-64 bg-surface border-r border-neutral-800 flex flex-col z-50 transition-transform duration-200 {isOpen
		? 'translate-x-0'
		: '-translate-x-full md:translate-x-0'}"
>
	<!-- Logo -->
	<div class="h-16 px-6 flex items-center border-b border-neutral-800">
		<a href="/" class="text-xl font-bold text-white hover:text-primary-400 transition-colors">
			Decent Cloud
		</a>
	</div>

	<!-- Navigation -->
	<nav class="flex-1 p-4 space-y-1 overflow-y-auto">
		<!-- Browse section -->
		<div class="pb-2 px-3">
			<div class="text-xs uppercase tracking-widest text-neutral-600">Browse</div>
		</div>
		{#each browseItems as item}
			{@const isActive =
				currentPath === item.href ||
				(item.label === 'Reputation' && currentPath.startsWith('/dashboard/reputation'))}
			<a
				href={item.href}
				onclick={closeSidebar}
				class="flex items-center gap-3 px-3 py-2 transition-colors {isActive
					? 'bg-primary-500/10 text-primary-400 border-l-2 border-primary-500 -ml-px'
					: 'text-neutral-400 hover:bg-surface-elevated hover:text-white'}"
			>
				<Icon name={item.icon} size={18} />
				<span class="text-sm font-medium">{item.label}</span>
			</a>
		{/each}

		{#if isAuthenticated}
			<!-- My Activity section -->
			<div class="pt-6 pb-2 px-3">
				<div class="text-xs uppercase tracking-widest text-neutral-600">My Activity</div>
			</div>
			{#each activityItems as item}
				{@const isActive = currentPath === item.href || currentPath.startsWith(item.href)}
				<a
					href={item.href}
					onclick={closeSidebar}
					class="flex items-center gap-3 px-3 py-2 transition-colors {isActive
						? 'bg-primary-500/10 text-primary-400 border-l-2 border-primary-500 -ml-px'
						: 'text-neutral-400 hover:bg-surface-elevated hover:text-white'}"
				>
					<Icon name={item.icon} size={18} />
					<span class="text-sm font-medium">{item.label}</span>
				</a>
			{/each}

			<!-- Provider section -->
			<div class="pt-6 pb-2 px-3">
				<div class="text-xs uppercase tracking-widest text-neutral-600">Provider</div>
			</div>

			<!-- Provider Setup -->
			{@const setupActive =
				currentPath === providerSetupItem.href ||
				currentPath.startsWith(providerSetupItem.href)}
			<a
				href={providerSetupItem.href}
				onclick={closeSidebar}
				class="flex items-center gap-3 px-3 py-2 transition-colors {setupActive
					? 'bg-primary-500/10 text-primary-400 border-l-2 border-primary-500 -ml-px'
					: 'text-neutral-400 hover:bg-surface-elevated hover:text-white'}"
			>
				<Icon name={providerSetupItem.icon} size={18} />
				<span class="text-sm font-medium">{providerSetupItem.label}</span>
				{#if hasOfferings && !onboardingCompleted}
					<span class="ml-auto w-2 h-2 bg-warning" title="Setup incomplete"></span>
				{/if}
			</a>

			<!-- Items only visible after onboarding is complete -->
			{#if onboardingCompleted}
				<!-- My Offerings -->
				{@const offeringsActive =
					currentPath === '/dashboard/offerings' ||
					currentPath.startsWith('/dashboard/offerings')}
				<a
					href="/dashboard/offerings"
					onclick={closeSidebar}
					class="flex items-center gap-3 px-3 py-2 transition-colors {offeringsActive
						? 'bg-primary-500/10 text-primary-400 border-l-2 border-primary-500 -ml-px'
						: 'text-neutral-400 hover:bg-surface-elevated hover:text-white'}"
				>
					<Icon name="package" size={18} />
					<span class="text-sm font-medium">My Offerings</span>
				</a>

				{#each providerOnboardedItems as item}
					{@const isActive = currentPath === item.href || currentPath.startsWith(item.href)}
					<a
						href={item.href}
						onclick={closeSidebar}
						class="flex items-center gap-3 px-3 py-2 transition-colors {isActive
							? 'bg-primary-500/10 text-primary-400 border-l-2 border-primary-500 -ml-px'
							: 'text-neutral-400 hover:bg-surface-elevated hover:text-white'}"
					>
						<Icon name={item.icon} size={18} />
						<span class="text-sm font-medium">{item.label}</span>
					</a>
				{/each}

				{#if CHATWOOT_BASE_URL}
					<a
						href={supportDashboardUrl}
						target="_blank"
						rel="noopener noreferrer"
						onclick={closeSidebar}
						class="flex items-center gap-3 px-3 py-2 text-neutral-400 hover:bg-surface-elevated hover:text-white transition-colors"
						title="Open Chatwoot support dashboard"
					>
						<Icon name="headphones" size={18} />
						<span class="text-sm font-medium">Support Dashboard</span>
						<Icon name="external" size={12} class="ml-auto opacity-50" />
					</a>
				{/if}
			{/if}
		{/if}

		{#if isAdmin}
			<!-- Admin section -->
			<div class="pt-6 pb-2 px-3">
				<div class="text-xs uppercase tracking-widest text-neutral-600">Admin</div>
			</div>
			<a
				href="/dashboard/admin"
				onclick={closeSidebar}
				class="flex items-center gap-3 px-3 py-2 transition-colors {currentPath.startsWith(
					'/dashboard/admin'
				)
					? 'bg-primary-500/10 text-primary-400 border-l-2 border-primary-500 -ml-px'
					: 'text-neutral-400 hover:bg-surface-elevated hover:text-white'}"
			>
				<Icon name="wrench" size={18} />
				<span class="text-sm font-medium">Admin</span>
			</a>
		{/if}
	</nav>

	<!-- User Section -->
	<div class="p-4 border-t border-neutral-800 space-y-2">
		{#if isAuthenticated}
			{#if currentIdentity?.account}
				<a
					href="/dashboard/account"
					onclick={closeSidebar}
					class="block px-3 py-2 text-center text-neutral-300 hover:text-white border-b border-neutral-800 mb-2 transition-colors"
					title="View account settings"
				>
					<span class="font-medium text-sm">@{currentIdentity.account.username}</span>
				</a>
			{/if}
			<a
				href="/dashboard/account"
				onclick={closeSidebar}
				class="flex items-center gap-3 px-3 py-2 transition-colors {currentPath.startsWith(
					'/dashboard/account'
				)
					? 'bg-primary-500/10 text-primary-400'
					: 'text-neutral-400 hover:bg-surface-elevated hover:text-white'}"
			>
				<Icon name="user" size={18} />
				<span class="text-sm font-medium">Account</span>
			</a>
			<button
				type="button"
				onclick={handleLogout}
				class="w-full flex items-center gap-3 px-3 py-2 text-neutral-400 hover:bg-surface-elevated hover:text-white transition-colors"
			>
				<Icon name="logout" size={18} />
				<span class="text-sm font-medium">Logout</span>
			</button>
		{:else}
			<button
				type="button"
				onclick={handleLogin}
				class="w-full flex items-center justify-center gap-2 px-4 py-2.5 bg-primary-500 text-base font-semibold hover:bg-primary-400 transition-colors"
			>
				<Icon name="login" size={18} />
				<span>Sign In</span>
			</button>
		{/if}
	</div>
</aside>
