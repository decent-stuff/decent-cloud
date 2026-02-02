<script lang="ts">
	import { onMount } from "svelte";
	import { browser } from "$app/environment";
	import { dashboardStore } from "$lib/stores/dashboard";
	import { authStore } from "$lib/stores/auth";
	import type { DashboardData } from "$lib/services/dashboard-data";
	import type { IdentityInfo } from "$lib/stores/auth";
	import { computePubkey } from "$lib/utils/contract-format";
	import { getProviderTrustMetrics, getProviderResponseMetrics, getProviderHealthSummary, getMyOfferings, type ProviderTrustMetrics, type ProviderResponseMetrics, type ProviderHealthSummary, type Offering } from "$lib/services/api";
	import { signRequest } from "$lib/services/auth-api";
	import TrustDashboard from "$lib/components/TrustDashboard.svelte";
	import RentalRequestDialog from "$lib/components/RentalRequestDialog.svelte";
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
	let healthSummary = $state<ProviderHealthSummary | null>(null);
	let trustMetricsLoading = $state(false);
	let trustMetricsError = $state<string | null>(null);

	// My Resources state
	let myOfferings = $state<Offering[]>([]);
	let myOfferingsLoading = $state(false);
	let myOfferingsError = $state<string | null>(null);
	let selectedOfferingForRental = $state<Offering | null>(null);

	async function loadTrustMetrics(publicKeyBytes: Uint8Array | null) {
		if (!publicKeyBytes) {
			trustMetrics = null;
			responseMetrics = null;
			healthSummary = null;
			trustMetricsError = null;
			return;
		}

		trustMetricsLoading = true;
		trustMetricsError = null;
		try {
			const pubkeyHex = computePubkey(publicKeyBytes);
			const [trustData, responseData, healthData] = await Promise.all([
				getProviderTrustMetrics(publicKeyBytes),
				getProviderResponseMetrics(pubkeyHex).catch(() => null),
				getProviderHealthSummary(pubkeyHex).catch(() => null),
			]);
			trustMetrics = trustData;
			responseMetrics = responseData;
			healthSummary = healthData;
		} catch (err) {
			console.error('Failed to load trust metrics:', err);
			trustMetrics = null;
			responseMetrics = null;
			healthSummary = null;
			trustMetricsError = err instanceof Error ? err.message : 'Failed to load trust metrics';
		} finally {
			trustMetricsLoading = false;
		}
	}

	async function loadMyOfferings(identity: IdentityInfo | null) {
		if (!identity) {
			myOfferings = [];
			myOfferingsError = null;
			return;
		}

		myOfferingsLoading = true;
		myOfferingsError = null;
		try {
			const { headers } = await signRequest(identity.identity, 'GET', '/api/v1/provider/my-offerings', '');
			const offerings = await getMyOfferings(headers);
			myOfferings = offerings;
		} catch (err) {
			console.error('Failed to load my offerings:', err);
			myOfferings = [];
			// Don't show error if user simply has no offerings
			if (err instanceof Error && !err.message.includes('404')) {
				myOfferingsError = err.message;
			}
		} finally {
			myOfferingsLoading = false;
		}
	}

	function handleRentalSuccess(contractId: string) {
		selectedOfferingForRental = null;
		// Redirect to rentals page to see the new contract
		window.location.href = `/dashboard/rentals`;
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
			loadMyOfferings(value);
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
		{:else if trustMetricsError}
			<div class="bg-danger/10 border border-danger/20 p-4">
				<p class="text-sm text-danger">Failed to load trust metrics</p>
				<p class="text-xs text-neutral-500 mt-1">{trustMetricsError}</p>
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
			<TrustDashboard metrics={trustMetrics} {responseMetrics} {healthSummary} />
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

	<!-- My Resources Section -->
	{#if currentIdentity}
		<div class="card p-5">
			<div class="flex items-center justify-between mb-4">
				<div>
					<h2 class="text-base font-semibold text-white">My Resources</h2>
					<p class="text-xs text-neutral-500 mt-1">Your infrastructure offerings - rent for free (self-rental)</p>
				</div>
				<a
					href="/dashboard/offerings"
					class="text-xs text-primary-400 hover:text-primary-300 transition-colors flex items-center gap-1"
				>
					<span>Manage offerings</span>
					<Icon name="arrow-right" size={16} />
				</a>
			</div>

			{#if myOfferingsLoading}
				<div class="flex justify-center items-center p-8">
					<div class="w-5 h-5 border-2 border-primary-500/30 border-t-primary-500 animate-spin"></div>
				</div>
			{:else if myOfferingsError}
				<div class="bg-danger/10 border border-danger/20 p-4">
					<p class="text-sm text-danger">Failed to load resources</p>
					<p class="text-xs text-neutral-500 mt-1">{myOfferingsError}</p>
				</div>
			{:else if myOfferings.length === 0}
				<div class="text-center py-8 border border-dashed border-neutral-800">
					<Icon name="server" size={32} class="mx-auto text-neutral-600 mb-3" />
					<p class="text-sm text-neutral-500">No resources yet</p>
					<p class="text-xs text-neutral-600 mt-1 mb-4">Create offerings to manage your own infrastructure</p>
					<a
						href="/dashboard/provider"
						class="inline-flex items-center gap-2 px-4 py-2 text-sm bg-primary-500 hover:bg-primary-600 text-white transition-colors"
					>
						<Icon name="plus" size={16} />
						<span>Get Started</span>
					</a>
				</div>
			{:else}
				<div class="space-y-3">
					{#each myOfferings.slice(0, 5) as offering (offering.id)}
						<div class="flex items-center gap-4 p-3 bg-surface-elevated border border-neutral-800">
							<div class="flex-1 min-w-0">
								<div class="flex items-center gap-2">
									<h3 class="text-sm font-medium text-white truncate">{offering.offer_name}</h3>
									{#if offering.visibility?.toLowerCase() === 'private'}
										<span class="px-1.5 py-0.5 text-[10px] font-medium bg-neutral-700 text-neutral-300">Private</span>
									{:else}
										<span class="px-1.5 py-0.5 text-[10px] font-medium bg-success/20 text-success">Public</span>
									{/if}
								</div>
								<div class="flex items-center gap-3 text-xs text-neutral-500 mt-1">
									{#if offering.processor_cores}
										<span>{offering.processor_cores} cores</span>
									{/if}
									{#if offering.memory_amount}
										<span>{offering.memory_amount}</span>
									{/if}
									{#if offering.total_ssd_capacity}
										<span>{offering.total_ssd_capacity} SSD</span>
									{/if}
									<span class="text-neutral-600">|</span>
									<span>{offering.datacenter_country}</span>
								</div>
							</div>
							<div class="text-right">
								<p class="text-sm font-medium text-white">
									{offering.monthly_price.toFixed(2)} {offering.currency}
								</p>
								<p class="text-[10px] text-neutral-500">/month</p>
							</div>
							<button
								onclick={() => selectedOfferingForRental = offering}
								class="px-3 py-1.5 text-xs font-medium bg-primary-500 hover:bg-primary-600 text-white transition-colors"
							>
								Rent Free
							</button>
						</div>
					{/each}
					{#if myOfferings.length > 5}
						<div class="text-center pt-2">
							<a
								href="/dashboard/offerings"
								class="text-xs text-primary-400 hover:text-primary-300 transition-colors"
							>
								View all {myOfferings.length} offerings
							</a>
						</div>
					{/if}
				</div>
			{/if}
		</div>
	{/if}
</div>

<!-- Rental Dialog -->
{#if selectedOfferingForRental}
	<RentalRequestDialog
		offering={selectedOfferingForRental}
		onClose={() => selectedOfferingForRental = null}
		onSuccess={handleRentalSuccess}
	/>
{/if}
