<script lang="ts">
	import { onMount } from "svelte";
	import { browser } from "$app/environment";
	import { goto } from "$app/navigation";
	import { dashboardStore } from "$lib/stores/dashboard";
	import { authStore } from "$lib/stores/auth";
	import type { DashboardData } from "$lib/services/dashboard-data";
	import type { IdentityInfo } from "$lib/stores/auth";
	import { computePubkey, formatContractPrice } from "$lib/utils/contract-format";
	import { getProviderTrustMetrics, getProviderResponseMetrics, getProviderHealthSummary, getMyOfferings, getPendingProviderRequests, type ProviderTrustMetrics, type ProviderResponseMetrics, type ProviderHealthSummary, type Offering } from "$lib/services/api";
	import { getUserActivity, type UserActivity } from "$lib/services/api-user-activity";
	import { signRequest } from "$lib/services/auth-api";
	import { detectUserRole, countActiveRentals, countExpiringSoon, countActiveRentalsAsProvider } from "$lib/utils/role-detection";
	import TrustDashboard from "$lib/components/TrustDashboard.svelte";
	import RentalRequestDialog from "$lib/components/RentalRequestDialog.svelte";
	import WelcomeModal from "$lib/components/WelcomeModal.svelte";
	import ExpiryBanner from "$lib/components/ExpiryBanner.svelte";
	import Icon from "$lib/components/Icons.svelte";

	let dashboardData = $state<DashboardData>({
		totalProviders: 0,
		activeProviders: 0,
		totalOfferings: 0,
		totalContracts: 0,
		activeValidators: 0,
		totalTransfers: 0,
		totalVolumeE9s: 0,
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

	// Recent Activity state
	let activity = $state<UserActivity | null>(null);
	let activityLoading = $state(false);

	// Pending provider requests
	let pendingRequestsCount = $state(0);

	// Platform stats collapsible
	let platformStatsExpanded = $state(false);

	// Derived role for personalized stats
	let userRole = $derived(detectUserRole(activity, myOfferings));

	// Spending insights derived from tenant rental activity
	let spendingInsights = $derived.by(() => {
		const rentals = activity?.rentals_as_requester ?? [];
		if (rentals.length === 0) return null;

		const now = new Date();
		const thisMonthStart = new Date(now.getFullYear(), now.getMonth(), 1).getTime();
		const lastMonthStart = new Date(now.getFullYear(), now.getMonth() - 1, 1).getTime();
		const lastMonthEnd = thisMonthStart;
		const daysLeftInMonth = new Date(now.getFullYear(), now.getMonth() + 1, 0).getDate() - now.getDate();

		let thisMonth = 0;
		let lastMonth = 0;
		const activeContracts: typeof rentals = [];

		for (const c of rentals) {
			const createdMs = (c.created_at_ns ?? 0) / 1_000_000;
			const amountIcp = (c.payment_amount_e9s ?? 0) / 1e9;

			if (createdMs >= thisMonthStart) thisMonth += amountIcp;
			else if (createdMs >= lastMonthStart && createdMs < lastMonthEnd) lastMonth += amountIcp;

			if (c.status === 'active' || c.status === 'provisioned') activeContracts.push(c);
		}

		// Sort active contracts by payment descending, take top 3
		const top3 = [...activeContracts]
			.sort((a, b) => (b.payment_amount_e9s ?? 0) - (a.payment_amount_e9s ?? 0))
			.slice(0, 3);

		// Projected: for active contracts with an end_timestamp_ns, compute remaining daily rate
		let projected = 0;
		for (const c of activeContracts) {
			if (!c.end_timestamp_ns || !c.duration_hours || !c.payment_amount_e9s) continue;
			const totalIcp = c.payment_amount_e9s / 1e9;
			const dailyRate = totalIcp / (c.duration_hours / 24);
			projected += dailyRate * daysLeftInMonth;
		}

		const trend: 'up' | 'down' | 'same' =
			lastMonth === 0 ? 'same' :
			thisMonth > lastMonth * 1.05 ? 'up' :
			thisMonth < lastMonth * 0.95 ? 'down' : 'same';

		return { thisMonth, lastMonth, trend, top3, projected, daysLeftInMonth };
	});

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
			pendingRequestsCount = 0;
			return;
		}

		myOfferingsLoading = true;
		myOfferingsError = null;
		try {
			const { headers } = await signRequest(identity.identity, 'GET', '/api/v1/provider/my-offerings', '');
			const offerings = await getMyOfferings(headers);
			myOfferings = offerings;
			if (offerings.length > 0) {
				loadPendingRequestsCount(identity);
			}
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

	async function loadPendingRequestsCount(identity: IdentityInfo) {
		try {
			const { headers } = await signRequest(identity.identity, 'GET', '/api/v1/provider/rental-requests/pending', '');
			const requests = await getPendingProviderRequests(headers);
			pendingRequestsCount = requests.length;
		} catch {
			pendingRequestsCount = 0;
		}
	}

	async function loadActivity(identity: IdentityInfo | null) {
		if (!identity) {
			activity = null;
			return;
		}

		activityLoading = true;
		try {
			const pubkeyHex = computePubkey(identity.publicKeyBytes!);
			const { headers } = await signRequest(identity.identity, 'GET', `/api/v1/users/${pubkeyHex}/activity`, '');
			activity = await getUserActivity(pubkeyHex, headers);
		} catch {
			activity = null;
		} finally {
			activityLoading = false;
		}
	}

	function formatExpiry(endNs: number | undefined): { text: string; urgent: boolean } | null {
		if (!endNs) return null;
		const endMs = endNs / 1_000_000;
		const hoursLeft = (endMs - Date.now()) / (1000 * 60 * 60);
		if (hoursLeft < 0) return null;
		if (hoursLeft < 24) {
			return { text: `Expires in ${Math.round(hoursLeft)}h`, urgent: true };
		}
		return { text: `Expires ${new Date(endMs).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })}`, urgent: false };
	}

	function handleRentalSuccess(contractId: string) {
		selectedOfferingForRental = null;
		// Navigate to contract detail page with welcome state
		goto(`/dashboard/rentals/${contractId}?welcome=true`);
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
			loadActivity(value);
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
						Logged in via {currentIdentity.type === 'oauth' ? 'OAuth' : 'Seed Phrase'}
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
		<p class="text-neutral-500 text-sm mt-1">
			{#if currentIdentity && !activityLoading && !myOfferingsLoading}
				{userRole === 'provider' ? 'Your provider overview' : userRole === 'tenant' ? 'Your rental overview' : 'Get started with Decent Cloud'}
			{:else}
				Marketplace statistics and quick actions
			{/if}
		</p>
	</div>

	{#if pendingRequestsCount > 0}
		<a
			href="/dashboard/provider/requests"
			class="flex items-center gap-4 p-4 bg-amber-500/10 border border-amber-500/30 hover:bg-amber-500/15 transition-colors"
		>
			<div class="w-9 h-9 shrink-0 bg-amber-500/20 border border-amber-500/30 flex items-center justify-center">
				<Icon name="inbox" size={18} class="text-amber-400" />
			</div>
			<div class="flex-1 min-w-0">
				<p class="text-sm font-semibold text-amber-300">
					{pendingRequestsCount} rental {pendingRequestsCount === 1 ? 'request requires' : 'requests require'} your action
				</p>
				<p class="text-xs text-amber-400/70 mt-0.5">Review and accept or reject pending rental requests</p>
			</div>
			<Icon name="arrow-right" size={16} class="text-amber-400 shrink-0" />
		</a>
	{/if}

	{#if currentIdentity && !activityLoading}
		<ExpiryBanner {activity} />
	{/if}

	{#if error}
		<div class="bg-danger/10 border border-danger/20 p-4">
			<p class="font-medium text-danger text-sm">Error loading dashboard data</p>
			<p class="text-xs text-neutral-400 mt-1">{error}</p>
		</div>
	{/if}

	<!-- Personalized Stats: role-aware -->
	{#if !currentIdentity || activityLoading || myOfferingsLoading}
		<!-- Loading or anonymous: show global platform stats -->
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
	{:else if userRole === 'new'}
		<!-- New user: prominent Get Started CTAs -->
		<div class="card p-6 border-primary-500/30 bg-primary-500/5">
			<h2 class="text-base font-semibold text-white mb-1">Ready to get started?</h2>
			<p class="text-sm text-neutral-400 mb-5">Choose how you want to use Decent Cloud.</p>
			<div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
				<a
					href="/dashboard/marketplace"
					class="group flex flex-col items-center gap-2 p-4 bg-surface-elevated border border-neutral-700 hover:border-primary-500/50 hover:bg-primary-500/5 transition-all text-center"
				>
					<div class="icon-box group-hover:border-primary-500/30 transition-colors">
						<Icon name="cart" size={20} />
					</div>
					<span class="text-sm font-medium text-white group-hover:text-primary-400 transition-colors">Browse Marketplace</span>
					<span class="text-xs text-neutral-500">Find and rent cloud VMs</span>
				</a>
				<a
					href="/dashboard/account"
					class="group flex flex-col items-center gap-2 p-4 bg-surface-elevated border border-neutral-700 hover:border-primary-500/50 hover:bg-primary-500/5 transition-all text-center"
				>
					<div class="icon-box group-hover:border-primary-500/30 transition-colors">
						<Icon name="user" size={20} />
					</div>
					<span class="text-sm font-medium text-white group-hover:text-primary-400 transition-colors">Add Cloud Account</span>
					<span class="text-xs text-neutral-500">Set up your profile</span>
				</a>
				<a
					href="/dashboard/validators"
					class="group flex flex-col items-center gap-2 p-4 bg-surface-elevated border border-neutral-700 hover:border-primary-500/50 hover:bg-primary-500/5 transition-all text-center"
				>
					<div class="icon-box group-hover:border-primary-500/30 transition-colors">
						<Icon name="shield" size={20} />
					</div>
					<span class="text-sm font-medium text-white group-hover:text-primary-400 transition-colors">View Validators</span>
					<span class="text-xs text-neutral-500">Network trust nodes</span>
				</a>
			</div>
		</div>
	{:else if userRole === 'tenant'}
		<!-- Tenant personalized stats -->
		<div class="grid grid-cols-2 md:grid-cols-4 gap-3">
			<div class="metric-card">
				<div class="flex items-center gap-2 mb-3">
					<Icon name="file" size={20} class="text-primary-500" />
					<span class="metric-label mb-0">Active Rentals</span>
				</div>
				<p class="metric-value">{countActiveRentals(activity)}</p>
				<p class="metric-subtext">Running now</p>
			</div>
			<div class="metric-card">
				<div class="flex items-center gap-2 mb-3">
					<Icon name="download" size={20} class="text-neutral-600" />
					<span class="metric-label mb-0">Total Spent</span>
				</div>
				<p class="metric-value text-base">
					{((activity?.rentals_as_requester ?? []).reduce((sum, c) => sum + (c.payment_amount_e9s ?? 0), 0) / 1e9).toFixed(2)}
				</p>
				<p class="metric-subtext">ICP lifetime</p>
			</div>
			<div class="metric-card">
				<div class="flex items-center gap-2 mb-3">
					<Icon name="clock" size={20} class="text-amber-500" />
					<span class="metric-label mb-0">Expiring Soon</span>
				</div>
				<p class="metric-value {countExpiringSoon(activity, 7) > 0 ? 'text-amber-400' : ''}">{countExpiringSoon(activity, 7)}</p>
				<p class="metric-subtext">Within 7 days</p>
			</div>
			<div class="metric-card">
				<div class="flex items-center gap-2 mb-3">
					<Icon name="package" size={20} class="text-neutral-600" />
					<span class="metric-label mb-0">Total Contracts</span>
				</div>
				<p class="metric-value">{activity?.rentals_as_requester.length ?? 0}</p>
				<p class="metric-subtext">All time</p>
			</div>
		</div>

		<!-- Spending Insights Widget -->
		{#if spendingInsights}
			<div class="card p-5">
				<div class="flex items-center justify-between mb-4">
					<h2 class="text-base font-semibold text-white">Spending Insights</h2>
					<a href="/dashboard/rentals" class="text-xs text-primary-400 hover:text-primary-300 transition-colors flex items-center gap-1">
						<span>View rentals</span>
						<Icon name="arrow-right" size={16} />
					</a>
				</div>

				<!-- Monthly comparison row -->
				<div class="grid grid-cols-2 sm:grid-cols-4 gap-3 mb-4">
					<div class="metric-card">
						<div class="flex items-center gap-2 mb-2">
							<Icon name="download" size={16} class="text-primary-500" />
							<span class="metric-label mb-0 text-xs">This Month</span>
						</div>
						<p class="text-lg font-bold text-white">{spendingInsights.thisMonth.toFixed(2)}</p>
						<p class="metric-subtext">ICP</p>
					</div>
					<div class="metric-card">
						<div class="flex items-center gap-2 mb-2">
							<Icon name="clock" size={16} class="text-neutral-600" />
							<span class="metric-label mb-0 text-xs">Last Month</span>
						</div>
						<p class="text-lg font-bold text-white">{spendingInsights.lastMonth.toFixed(2)}</p>
						<p class="metric-subtext">ICP</p>
					</div>
					<div class="metric-card">
						<div class="flex items-center gap-2 mb-2">
							<Icon name="arrow-right" size={16} class={spendingInsights.trend === 'up' ? 'text-danger' : spendingInsights.trend === 'down' ? 'text-success' : 'text-neutral-500'} />
							<span class="metric-label mb-0 text-xs">Trend</span>
						</div>
						<p class="text-lg font-bold {spendingInsights.trend === 'up' ? 'text-danger' : spendingInsights.trend === 'down' ? 'text-success' : 'text-neutral-400'}">
							{spendingInsights.trend === 'up' ? '↑↑' : spendingInsights.trend === 'down' ? '↓↓' : '→'}
						</p>
						<p class="metric-subtext">vs last month</p>
					</div>
					{#if spendingInsights.projected > 0}
						<div class="metric-card">
							<div class="flex items-center gap-2 mb-2">
								<Icon name="file" size={16} class="text-amber-500" />
								<span class="metric-label mb-0 text-xs">Projected</span>
							</div>
							<p class="text-lg font-bold text-white">{spendingInsights.projected.toFixed(2)}</p>
							<p class="metric-subtext">{spendingInsights.daysLeftInMonth}d remaining</p>
						</div>
					{/if}
				</div>

				<!-- Top 3 most expensive active contracts -->
				{#if spendingInsights.top3.length > 0}
					<div>
						<p class="text-xs text-neutral-500 font-medium mb-2">Top Active Contracts</p>
						<div class="space-y-2">
							{#each spendingInsights.top3 as contract (contract.contract_id)}
								<a
									href="/dashboard/rentals/{contract.contract_id}"
									class="flex items-center justify-between p-3 bg-surface-elevated border border-neutral-800 hover:border-neutral-700 transition-colors"
								>
									<div class="flex-1 min-w-0">
										<p class="text-sm text-neutral-300 font-mono truncate">{contract.offering_id}</p>
										<p class="text-xs text-neutral-600 font-mono">{contract.contract_id.slice(0, 12)}…</p>
									</div>
									<span class="text-sm font-mono text-primary-400 ml-3">
										{(contract.payment_amount_e9s / 1e9).toFixed(2)} ICP
									</span>
								</a>
							{/each}
						</div>
					</div>
				{/if}
			</div>
		{/if}
	{:else}
		<!-- Provider personalized stats -->
		<div class="grid grid-cols-2 md:grid-cols-4 gap-3">
			<div class="metric-card">
				<div class="flex items-center gap-2 mb-3">
					<Icon name="server" size={20} class="text-primary-500" />
					<span class="metric-label mb-0">Active Offerings</span>
				</div>
				<p class="metric-value">{myOfferings.length}</p>
				<p class="metric-subtext">Listed</p>
			</div>
			<div class="metric-card">
				<div class="flex items-center gap-2 mb-3">
					<Icon name="file" size={20} class="text-neutral-600" />
					<span class="metric-label mb-0">Active Rentals</span>
				</div>
				<p class="metric-value">{countActiveRentalsAsProvider(activity)}</p>
				<p class="metric-subtext">As provider</p>
			</div>
			<div class="metric-card">
				<div class="flex items-center gap-2 mb-3">
					<Icon name="check" size={20} class="text-success" />
					<span class="metric-label mb-0">Earnings</span>
				</div>
				<p class="metric-value text-base">
					{((activity?.rentals_as_provider ?? []).filter(c => c.status === 'active' || c.status === 'provisioned').reduce((sum, c) => sum + (c.payment_amount_e9s ?? 0), 0) / 1e9).toFixed(2)}
				</p>
				<p class="metric-subtext">ICP active</p>
			</div>
			<div class="metric-card">
				<div class="flex items-center gap-2 mb-3">
					<Icon name="inbox" size={20} class="text-amber-500" />
					<span class="metric-label mb-0">Pending</span>
				</div>
				<p class="metric-value {pendingRequestsCount > 0 ? 'text-amber-400' : ''}">{pendingRequestsCount}</p>
				<p class="metric-subtext">Requests</p>
			</div>
		</div>
	{/if}

	<!-- Platform Overview (collapsible, shown to authenticated users with a role) -->
	{#if currentIdentity && !activityLoading && !myOfferingsLoading && userRole !== 'new'}
		<div>
			<button
				type="button"
				onclick={() => platformStatsExpanded = !platformStatsExpanded}
				class="flex items-center gap-2 text-xs text-neutral-500 hover:text-neutral-400 transition-colors"
			>
				<Icon name={platformStatsExpanded ? 'chevron-down' : 'chevron-right'} size={14} />
				Platform Overview
			</button>
			{#if platformStatsExpanded}
				<div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-5 gap-3 mt-3">
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
			{/if}
		</div>
	{/if}

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

			{#if !currentIdentity || myOfferingsLoading || myOfferings.length > 0}
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
			{:else}
				<a
					href="/dashboard/provider/support"
					class="group flex items-center gap-3 p-4 bg-surface-elevated border border-neutral-800 hover:border-neutral-700 hover:bg-surface-hover transition-all"
				>
					<div class="icon-box group-hover:border-primary-500/30 transition-colors">
						<Icon name="server" size={20} />
					</div>
					<div>
						<h3 class="text-sm font-medium text-white group-hover:text-primary-400 transition-colors">
							Provider Setup
						</h3>
						<p class="text-xs text-neutral-500">Share your resources</p>
					</div>
				</a>
			{/if}

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

			{#if !currentIdentity || myOfferings.length > 0}
				<a
					href="/dashboard/provider/requests"
					class="group flex items-center gap-3 p-4 bg-surface-elevated border {pendingRequestsCount > 0 ? 'border-amber-500/40 hover:border-amber-500/60' : 'border-neutral-800 hover:border-neutral-700'} hover:bg-surface-hover transition-all relative"
				>
					<div class="icon-box group-hover:border-primary-500/30 transition-colors">
						<Icon name="inbox" size={20} />
					</div>
					<div>
						<h3 class="text-sm font-medium text-white group-hover:text-primary-400 transition-colors">
							Rental Requests
						</h3>
						<p class="text-xs text-neutral-500">Review &amp; respond</p>
					</div>
					{#if pendingRequestsCount > 0}
						<span class="absolute top-2 right-2 min-w-5 h-5 px-1 flex items-center justify-center text-[10px] font-bold bg-amber-500 text-neutral-900 rounded-full">{pendingRequestsCount}</span>
					{/if}
				</a>
			{:else}
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
			{/if}
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

		<!-- Recent Activity -->
		{#if activityLoading}
			<div class="flex justify-center items-center p-4">
				<div class="w-4 h-4 border-2 border-primary-500/30 border-t-primary-500 animate-spin"></div>
			</div>
		{:else if activity}
			<div class="card p-5">
				<div class="flex items-center justify-between mb-4">
					<div>
						<h2 class="text-base font-semibold text-white">Recent Activity</h2>
						<p class="text-xs text-neutral-500 mt-1">Your latest contracts and offerings</p>
					</div>
					<div class="flex items-center gap-4">
						{#if activity.rentals_as_provider.length > 0}
							<a
								href="/dashboard/provider/earnings"
								class="text-xs text-emerald-400 hover:text-emerald-300 transition-colors flex items-center gap-1"
							>
								<span>View earnings</span>
								<Icon name="arrow-right" size={16} />
							</a>
						{/if}
						<a
							href="/dashboard/rentals"
							class="text-xs text-primary-400 hover:text-primary-300 transition-colors flex items-center gap-1"
						>
							<span>View all rentals</span>
							<Icon name="arrow-right" size={16} />
						</a>
					</div>
				</div>
				<div class="grid grid-cols-2 sm:grid-cols-4 gap-3 mb-4">
					<div class="bg-surface-elevated border border-neutral-800 p-3">
						<div class="text-xs text-neutral-500 mb-1">Rentals (requester)</div>
						<div class="text-xl font-semibold text-white">{activity.rentals_as_requester.length}</div>
					</div>
					<div class="bg-surface-elevated border border-neutral-800 p-3">
						<div class="text-xs text-neutral-500 mb-1">Rentals (provider)</div>
						<div class="text-xl font-semibold text-white">{activity.rentals_as_provider.length}</div>
					</div>
					<div class="bg-surface-elevated border border-neutral-800 p-3">
						<div class="text-xs text-neutral-500 mb-1">Offerings provided</div>
						<div class="text-xl font-semibold text-white">{activity.offerings_provided.length}</div>
					</div>
					<div class="bg-surface-elevated border border-neutral-800 p-3">
						<div class="text-xs text-neutral-500 mb-1">Active contracts</div>
						<div class="text-xl font-semibold text-white">{activity.rentals_as_requester.filter(c => ['active', 'provisioned'].includes(c.status)).length}</div>
					</div>
				</div>
				{#if activity.rentals_as_requester.length > 0}
					<div class="space-y-2">
						<div class="text-xs text-neutral-500 font-medium mb-2">As Tenant (last 3)</div>
						{#each activity.rentals_as_requester.slice(0, 3) as contract (contract.contract_id)}
							{@const expiry = (contract.status === 'active' || contract.status === 'provisioned') ? formatExpiry(contract.end_timestamp_ns) : null}
							<a
								href="/dashboard/rentals/{contract.contract_id}"
								class="flex items-center justify-between p-3 bg-surface-elevated border border-neutral-800 hover:border-neutral-700 transition-colors"
							>
								<div class="flex-1 min-w-0">
									<div class="text-sm text-neutral-300 truncate font-mono">{contract.contract_id.slice(0, 16)}...</div>
									<div class="flex items-center gap-3 mt-0.5">
										<span class="text-xs text-neutral-500">{contract.offering_id}</span>
										{#if expiry}
											<span class="text-xs {expiry.urgent ? 'text-amber-400' : 'text-neutral-500'}">{expiry.text}</span>
										{/if}
									</div>
								</div>
								<span class="ml-3 px-2 py-0.5 text-xs font-medium rounded-full
									{contract.status === 'active' || contract.status === 'provisioned' ? 'bg-emerald-500/20 text-emerald-400' :
									contract.status === 'cancelled' || contract.status === 'rejected' ? 'bg-red-500/20 text-red-400' :
									'bg-neutral-700 text-neutral-300'}">{contract.status}</span>
							</a>
						{/each}
					</div>
				{:else}
					<div class="text-center py-6 border border-dashed border-neutral-800">
						<Icon name="cart" size={28} class="mx-auto text-neutral-600 mb-3" />
						<p class="text-sm text-neutral-400 font-medium mb-1">Deploy your first VM</p>
						<p class="text-xs text-neutral-600 mb-4">Browse available offerings on the marketplace</p>
						<a
							href="/dashboard/marketplace"
							class="inline-flex items-center gap-2 px-4 py-2 bg-primary-500 hover:bg-primary-400 text-neutral-900 text-sm font-semibold transition-colors"
						>
							<Icon name="cart" size={14} />
							<span>Browse Marketplace</span>
						</a>
					</div>
				{/if}
				{#if activity.rentals_as_provider.length > 0}
					<div class="space-y-2 mt-4">
						<div class="flex items-center justify-between mb-2">
							<div class="text-xs text-neutral-500 font-medium">As Provider (last 3)</div>
							<a
								href="/dashboard/provider/earnings"
								class="text-xs text-emerald-400 hover:text-emerald-300 transition-colors flex items-center gap-1"
							>
								<span>View earnings</span>
								<Icon name="arrow-right" size={16} />
							</a>
						</div>
						{#each activity.rentals_as_provider.slice(0, 3) as contract (contract.contract_id)}
							<div class="flex items-center justify-between p-3 bg-surface-elevated border border-neutral-800">
								<div class="flex-1 min-w-0">
									<div class="text-sm text-neutral-300 truncate font-mono">{contract.contract_id.slice(0, 16)}...</div>
								</div>
								<div class="flex items-center gap-3 ml-3">
									<span class="text-sm font-mono text-emerald-400">{formatContractPrice(contract.payment_amount_e9s, 'ICP')}</span>
									<span class="px-2 py-0.5 text-xs font-medium rounded-full
										{contract.status === 'active' || contract.status === 'provisioned' ? 'bg-emerald-500/20 text-emerald-400' :
										contract.status === 'cancelled' || contract.status === 'rejected' ? 'bg-red-500/20 text-red-400' :
										'bg-neutral-700 text-neutral-300'}">{contract.status}</span>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</div>
		{/if}
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

<!-- Welcome Modal: shown only on first login -->
{#if currentIdentity}
	<WelcomeModal />
{/if}
