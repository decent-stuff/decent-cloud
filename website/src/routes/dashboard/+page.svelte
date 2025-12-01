<script lang="ts">
	import { onMount } from "svelte";
	import { browser } from "$app/environment";
	import { dashboardStore } from "$lib/stores/dashboard";
	import { authStore } from "$lib/stores/auth";
	import type { DashboardData } from "$lib/services/dashboard-data";
	import type { IdentityInfo } from "$lib/stores/auth";
	import { computePubkey } from "$lib/utils/contract-format";
	import { getProviderTrustMetrics, type ProviderTrustMetrics } from "$lib/services/api";
	import TrustDashboard from "$lib/components/TrustDashboard.svelte";

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
	let trustMetricsLoading = $state(false);

	async function loadTrustMetrics(publicKeyBytes: Uint8Array | null) {
		if (!publicKeyBytes) {
			trustMetrics = null;
			return;
		}

		trustMetricsLoading = true;
		try {
			trustMetrics = await getProviderTrustMetrics(publicKeyBytes);
		} catch {
			trustMetrics = null; // User has no trust data yet
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
		<div
			class="bg-gradient-to-r from-blue-500/20 to-purple-600/20 backdrop-blur-lg rounded-xl p-6 border border-blue-500/30"
		>
			<div class="flex items-center gap-4">
				<div class="text-4xl">
					🔑
				</div>
				<div class="flex-1">
					<h2 class="text-2xl font-bold text-white mb-1">
						{#if currentIdentity.displayName}
							Welcome back, {currentIdentity.displayName}!
						{:else}
							Welcome back!
						{/if}
					</h2>
					<p class="text-white/70 text-sm">
						Logged in via Seed Phrase
					</p>
					<p class="text-white/50 text-xs font-mono mt-1">
						Principal: {currentIdentity.principal.toString()}
					</p>
					{#if currentIdentity.publicKeyBytes}
						<p class="text-white/50 text-xs font-mono">
							Public key (hex): {computePubkey(
								currentIdentity.publicKeyBytes,
							)}
						</p>
					{/if}
				</div>
			</div>
		</div>

		<!-- User's Trust Metrics -->
		{#if trustMetricsLoading}
			<div class="flex justify-center items-center p-8">
				<div class="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-blue-400"></div>
			</div>
		{:else if trustMetrics}
			<div class="flex items-center justify-between mb-2">
				<h2 class="text-2xl font-bold text-white">Your Trust Score</h2>
				<a
					href="/dashboard/reputation/{computePubkey(currentIdentity.publicKeyBytes!)}"
					class="text-sm text-blue-400 hover:text-blue-300 transition-colors"
				>
					View full profile &rarr;
				</a>
			</div>
			<TrustDashboard metrics={trustMetrics} />
		{/if}
	{/if}

	<div>
		<h1 class="text-4xl font-bold text-white mb-2">Dashboard Overview</h1>
		<p class="text-white/60">Marketplace statistics and quick actions</p>
	</div>

	{#if error}
		<div
			class="bg-red-500/20 border border-red-500/30 rounded-lg p-4 text-red-400"
		>
			<p class="font-semibold">Error loading dashboard data</p>
			<p class="text-sm mt-1">{error}</p>
		</div>
	{/if}

	<!-- Stats Grid -->
	<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
		<!-- Total Providers -->
		<div
			class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20"
		>
			<div class="flex items-center justify-between mb-2">
				<h3 class="text-white/70 text-sm font-medium">Total Providers</h3>
				<span class="text-2xl">🖥️</span>
			</div>
			<p class="text-3xl font-bold text-white">
				{dashboardData.totalProviders.toLocaleString()}
			</p>
			<p class="text-white/50 text-sm mt-1">Registered providers</p>
		</div>

		<!-- Active Providers -->
		<div
			class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20"
		>
			<div class="flex items-center justify-between mb-2">
				<h3 class="text-white/70 text-sm font-medium">Active Providers</h3>
				<span class="text-2xl">✅</span>
			</div>
			<p class="text-3xl font-bold text-white">
				{dashboardData.activeProviders.toLocaleString()}
			</p>
			<p class="text-white/50 text-sm mt-1">Currently active</p>
		</div>

		<!-- Total Offerings -->
		<div
			class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20"
		>
			<div class="flex items-center justify-between mb-2">
				<h3 class="text-white/70 text-sm font-medium">Available Offerings</h3>
				<span class="text-2xl">📦</span>
			</div>
			<p class="text-3xl font-bold text-white">
				{dashboardData.totalOfferings.toLocaleString()}
			</p>
			<p class="text-white/50 text-sm mt-1">Services listed</p>
		</div>

		<!-- Total Contracts -->
		<div
			class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20"
		>
			<div class="flex items-center justify-between mb-2">
				<h3 class="text-white/70 text-sm font-medium">
					Total Contracts
				</h3>
				<span class="text-2xl">📝</span>
			</div>
			<p class="text-3xl font-bold text-white">
				{dashboardData.totalContracts.toLocaleString()}
			</p>
			<p class="text-white/50 text-sm mt-1">Marketplace agreements</p>
		</div>

		<!-- Active Validators -->
		<div
			class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20"
		>
			<div class="flex items-center justify-between mb-2">
				<h3 class="text-white/70 text-sm font-medium">Active Validators</h3>
				<span class="text-2xl">🛡️</span>
			</div>
			<p class="text-3xl font-bold text-white">
				{dashboardData.activeValidators.toLocaleString()}
			</p>
			<p class="text-white/50 text-sm mt-1">Network security</p>
		</div>
	</div>

	<!-- Quick Actions -->
	<div
		class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20"
	>
		<h2 class="text-2xl font-bold text-white mb-4">Quick Actions</h2>
		<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
			<a
				href="/dashboard/marketplace"
				class="p-4 bg-gradient-to-r from-blue-500/20 to-purple-600/20 rounded-lg border border-blue-500/30 hover:border-blue-400 transition-all group"
			>
				<span class="text-3xl mb-2 block">🛒</span>
				<h3
					class="text-white font-semibold mb-1 group-hover:text-blue-400"
				>
					Browse Marketplace
				</h3>
				<p class="text-white/60 text-sm">Find cloud services</p>
			</a>

			<a
				href="/dashboard/offerings"
				class="p-4 bg-gradient-to-r from-purple-500/20 to-pink-600/20 rounded-lg border border-purple-500/30 hover:border-purple-400 transition-all group"
			>
				<span class="text-3xl mb-2 block">📦</span>
				<h3
					class="text-white font-semibold mb-1 group-hover:text-purple-400"
				>
					Manage Offerings
				</h3>
				<p class="text-white/60 text-sm">Your cloud services</p>
			</a>

			<a
				href="/dashboard/rentals"
				class="p-4 bg-gradient-to-r from-amber-500/20 to-orange-600/20 rounded-lg border border-amber-500/30 hover:border-amber-400 transition-all group"
			>
				<span class="text-3xl mb-2 block">📋</span>
				<h3
					class="text-white font-semibold mb-1 group-hover:text-amber-400"
				>
					My Rentals
				</h3>
				<p class="text-white/60 text-sm">Rented services</p>
			</a>

			<a
				href="/dashboard/validators"
				class="p-4 bg-gradient-to-r from-green-500/20 to-teal-600/20 rounded-lg border border-green-500/30 hover:border-green-400 transition-all group"
			>
				<span class="text-3xl mb-2 block">✓</span>
				<h3
					class="text-white font-semibold mb-1 group-hover:text-green-400"
				>
					View Validators
				</h3>
				<p class="text-white/60 text-sm">Network participants</p>
			</a>
		</div>
	</div>
</div>
