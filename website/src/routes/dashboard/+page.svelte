<script lang="ts">
	import { onMount } from "svelte";
	import { browser } from "$app/environment";
	import { dashboardStore } from "$lib/stores/dashboard";
	import { authStore } from "$lib/stores/auth";
	import type { DashboardData } from "$lib/services/dashboard-data";
	import type { IdentityInfo } from "$lib/stores/auth";
	import { computePubkey } from "$lib/utils/contract-format";
	import { getProviderTrustMetrics, getProviderResponseMetrics, type ProviderTrustMetrics, type ProviderResponseMetrics } from "$lib/services/api";
	import TrustDashboard from "$lib/components/TrustDashboard.svelte";
	import Icon from "$lib/components/Icons.svelte";

	let dashboardData = $state<DashboardData>({
		totalProviders: 0,
		activeProviders: 0,
		totalOfferings: 0,
		totalContracts: 0,
		activeValidators: 0,
	});
	let error = $state<string | null>(null);
	let currentIdentity = $state<IdentityInfo | null>(null);
	let trustMetrics = $state<ProviderTrustMetrics | null>(null);
	let responseMetrics = $state<ProviderResponseMetrics | null>(null);
	let trustMetricsLoading = $state(false);

	async function loadTrustMetrics(publicKeyBytes: Uint8Array | null) {
		if (!publicKeyBytes) {
			trustMetrics = null;
			responseMetrics = null;
			return;
		}

		trustMetricsLoading = true;
		try {
			const pubkeyHex = computePubkey(publicKeyBytes);
			const [trustData, responseData] = await Promise.all([
				getProviderTrustMetrics(publicKeyBytes).catch(() => null),
				getProviderResponseMetrics(pubkeyHex).catch(() => null),
			]);
			trustMetrics = trustData;
			responseMetrics = responseData;
		} catch {
			trustMetrics = null;
			responseMetrics = null;
		} finally {
			trustMetricsLoading = false;
		}
	}

	onMount(() => {
		if (!browser) return;

		const unsubscribeData = dashboardStore.data.subscribe((value) => {
			dashboardData = value;
		});
		const unsubscribeError = dashboardStore.error.subscribe((value) => {
			error = value;
		});
		const unsubscribeAuth = authStore.currentIdentity.subscribe((value) => {
			currentIdentity = value;
			loadTrustMetrics(value?.publicKeyBytes ?? null);
		});

		dashboardStore.load();
		authStore.updateDisplayName();
		const interval = setInterval(() => dashboardStore.load(), 10000);

		return () => {
			unsubscribeData();
			unsubscribeError();
			unsubscribeAuth();
			clearInterval(interval);
		};
	});
</script>

<div class="space-y-8">
	<!-- User Info Section -->
	{#if currentIdentity}
		<div class="card-accent p-5">
			<div class="flex items-center gap-4">
				<div class="icon-box-accent">
					<Icon name="key" size={20} />
				</div>
				<div class="flex-1 min-w-0">
					<h2 class="text-lg font-semibold text-white">
						{#if currentIdentity.displayName}
							Welcome back, {currentIdentity.displayName}
						{:else}
							Welcome back
						{/if}
					</h2>
					<p class="text-neutral-500 text-xs mt-1">
						Logged in via Seed Phrase
					</p>
					<p class="text-neutral-600 text-[10px] font-mono mt-2 truncate" title={currentIdentity.principal.toString()}>
						{currentIdentity.principal.toString()}
					</p>
				</div>
			</div>
		</div>

		<!-- User's Trust Metrics -->
		{#if trustMetricsLoading}
			<div class="flex justify-center items-center p-8">
				<div class="w-5 h-5 border-2 border-primary-500/30 border-t-primary-500 animate-spin"></div>
			</div>
		{:else if trustMetrics}
			<div class="flex items-center justify-between mb-3">
				<h2 class="text-lg font-semibold text-white">Your Trust Score</h2>
				<a
					href="/dashboard/reputation/{computePubkey(currentIdentity.publicKeyBytes!)}"
					class="text-xs text-primary-400 hover:text-primary-300 transition-colors flex items-center gap-1"
				>
					<span>View full profile</span>
					<Icon name="arrow-right" size={20} />
				</a>
			</div>
			<TrustDashboard metrics={trustMetrics} {responseMetrics} />
		{/if}
	{/if}

	<!-- Page Header -->
	<div>
		<h1 class="text-2xl font-bold text-white tracking-tight">Dashboard</h1>
		<p class="text-neutral-500 text-sm mt-1">Marketplace statistics and quick actions</p>
	</div>

	{#if error}
		<div class="bg-danger/10 border border-danger/20 p-4">
			<p class="font-medium text-danger text-sm">Error loading dashboard data</p>
			<p class="text-xs text-neutral-400 mt-1">{error}</p>
		</div>
	{/if}

	<!-- Stats Grid -->
	<div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-5 gap-3">
		<div class="metric-card">
			<div class="flex items-center gap-2 mb-3">
				<Icon name="server" size={20} class="text-neutral-600" />
				<span class="metric-label mb-0">Providers</span>
			</div>
			<p class="metric-value">{dashboardData.totalProviders.toLocaleString()}</p>
			<p class="metric-subtext">Registered</p>
		</div>

		<div class="metric-card">
			<div class="flex items-center gap-2 mb-3">
				<Icon name="check" size={20} class="text-success" />
				<span class="metric-label mb-0">Active</span>
			</div>
			<p class="metric-value">{dashboardData.activeProviders.toLocaleString()}</p>
			<p class="metric-subtext">Online now</p>
		</div>

		<div class="metric-card">
			<div class="flex items-center gap-2 mb-3">
				<Icon name="package" size={20} class="text-neutral-600" />
				<span class="metric-label mb-0">Offerings</span>
			</div>
			<p class="metric-value">{dashboardData.totalOfferings.toLocaleString()}</p>
			<p class="metric-subtext">Available</p>
		</div>

		<div class="metric-card">
			<div class="flex items-center gap-2 mb-3">
				<Icon name="file" size={20} class="text-neutral-600" />
				<span class="metric-label mb-0">Contracts</span>
			</div>
			<p class="metric-value">{dashboardData.totalContracts.toLocaleString()}</p>
			<p class="metric-subtext">Total</p>
		</div>

		<div class="metric-card">
			<div class="flex items-center gap-2 mb-3">
				<Icon name="shield" size={20} class="text-neutral-600" />
				<span class="metric-label mb-0">Validators</span>
			</div>
			<p class="metric-value">{dashboardData.activeValidators.toLocaleString()}</p>
			<p class="metric-subtext">Active</p>
		</div>
	</div>

	<!-- Quick Actions -->
	<div class="card p-5">
		<h2 class="text-base font-semibold text-white mb-4">Quick Actions</h2>
		<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
			<a
				href="/dashboard/marketplace"
				class="group flex items-center gap-3 p-4 bg-surface-elevated border border-neutral-800 hover:border-neutral-700 hover:bg-surface-hover transition-all"
			>
				<div class="icon-box group-hover:border-primary-500/30 transition-colors">
					<Icon name="cart" size={20} />
				</div>
				<div>
					<h3 class="text-sm font-medium text-white group-hover:text-primary-400 transition-colors">
						Marketplace
					</h3>
					<p class="text-xs text-neutral-500">Browse services</p>
				</div>
			</a>

			<a
				href="/dashboard/offerings"
				class="group flex items-center gap-3 p-4 bg-surface-elevated border border-neutral-800 hover:border-neutral-700 hover:bg-surface-hover transition-all"
			>
				<div class="icon-box group-hover:border-primary-500/30 transition-colors">
					<Icon name="package" size={20} />
				</div>
				<div>
					<h3 class="text-sm font-medium text-white group-hover:text-primary-400 transition-colors">
						My Offerings
					</h3>
					<p class="text-xs text-neutral-500">Manage services</p>
				</div>
			</a>

			<a
				href="/dashboard/rentals"
				class="group flex items-center gap-3 p-4 bg-surface-elevated border border-neutral-800 hover:border-neutral-700 hover:bg-surface-hover transition-all"
			>
				<div class="icon-box group-hover:border-primary-500/30 transition-colors">
					<Icon name="file" size={20} />
				</div>
				<div>
					<h3 class="text-sm font-medium text-white group-hover:text-primary-400 transition-colors">
						My Rentals
					</h3>
					<p class="text-xs text-neutral-500">Active contracts</p>
				</div>
			</a>

			<a
				href="/dashboard/validators"
				class="group flex items-center gap-3 p-4 bg-surface-elevated border border-neutral-800 hover:border-neutral-700 hover:bg-surface-hover transition-all"
			>
				<div class="icon-box group-hover:border-primary-500/30 transition-colors">
					<Icon name="shield" size={20} />
				</div>
				<div>
					<h3 class="text-sm font-medium text-white group-hover:text-primary-400 transition-colors">
						Validators
					</h3>
					<p class="text-xs text-neutral-500">Network nodes</p>
				</div>
			</a>
		</div>
	</div>
</div>
