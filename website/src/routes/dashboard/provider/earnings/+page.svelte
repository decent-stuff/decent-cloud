<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { page } from "$app/stores";
	import { navigateToLogin } from "$lib/utils/navigation";
	import {
		getProviderStats,
		getProviderFeedbackStats,
		getProviderBandwidthStats,
		getProviderOnboarding,
		getProviderRevenueByMonth,
		hexEncode,
		type ProviderStats,
		type ProviderFeedbackStats,
		type BandwidthStatsResponse,
		type RevenueByMonth,
	} from "$lib/services/api";
	import ProviderSetupBanner from "$lib/components/ProviderSetupBanner.svelte";
	import { getAccountBalance } from "$lib/services/api-reputation";
	import { signRequest } from "$lib/services/auth-api";
	import { authStore } from "$lib/stores/auth";
	import { Ed25519KeyIdentity } from "@dfinity/identity";

	let stats = $state<ProviderStats | null>(null);
	let feedbackStats = $state<ProviderFeedbackStats | null>(null);
	let bandwidthStats = $state<BandwidthStatsResponse[]>([]);
	let revenueByMonth = $state<RevenueByMonth[]>([]);
	let tokenBalance = $state<number>(0);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let isAuthenticated = $state(false);
	let onboardingCompleted = $state<boolean | null>(null);
	let unsubscribeAuth: (() => void) | null = null;

	function formatRevenue(e9s: number): string {
		return (e9s / 1_000_000_000).toFixed(2);
	}

	function formatBalance(e9s: number): string {
		return (e9s / 1_000_000_000).toFixed(4);
	}

	function formatBytes(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(2)} KB`;
		if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
		return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
	}

	function formatNsTimestamp(ns: number): string {
		return new Date(ns / 1_000_000).toLocaleString();
	}

	onMount(() => {
		unsubscribeAuth = authStore.isAuthenticated.subscribe((isAuth) => {
			isAuthenticated = isAuth;
			if (isAuth) {
				loadData();
			} else {
				loading = false;
			}
		});
	});

	async function loadData() {
		try {
			loading = true;
			error = null;

			const info = await authStore.getSigningIdentity();
			if (!info) {
				error = "You must be authenticated to view earnings";
				return;
			}

			const providerHex = hexEncode(info.publicKeyBytes);

			const bandwidthStats_ = await (async () => {
				if (!(info.identity instanceof Ed25519KeyIdentity)) return [];
				const signed = await signRequest(
					info.identity,
					"GET",
					`/api/v1/providers/${providerHex}/bandwidth`,
				);
				return getProviderBandwidthStats(providerHex, signed.headers).catch(() => []);
			})();

			const [providerStats, feedback, balance, onboarding, revenueData] = await Promise.all([
				getProviderStats(providerHex),
				getProviderFeedbackStats(providerHex).catch(() => null),
				getAccountBalance(providerHex).catch(() => 0),
				getProviderOnboarding(providerHex).catch(() => null),
				getProviderRevenueByMonth(providerHex).catch(() => []),
			]);

			stats = providerStats;
			feedbackStats = feedback;
			tokenBalance = balance;
			bandwidthStats = bandwidthStats_;
			onboardingCompleted = !!onboarding?.onboarding_completed_at;
			revenueByMonth = revenueData;
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load earnings data";
		} finally {
			loading = false;
		}
	}

	function handleLogin() {
		navigateToLogin($page.url.pathname);
	}

	onDestroy(() => {
		unsubscribeAuth?.();
	});
</script>

<div class="space-y-8">
	<ProviderSetupBanner completed={onboardingCompleted} />

	<header>
		<h1 class="text-2xl font-bold text-white tracking-tight">Provider Earnings</h1>
		<p class="text-neutral-500">Revenue, contracts, and customer feedback at a glance</p>
	</header>

	{#if !isAuthenticated}
		<div class="card p-8 border border-neutral-800 text-center">
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">📊</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-neutral-400">
					Create an account or login to view your provider earnings, contract statistics, and customer feedback.
				</p>
				<button
					onclick={handleLogin}
					class="px-8 py-3 bg-gradient-to-r from-primary-500 to-primary-600 font-semibold text-white hover:brightness-110 hover:scale-105 transition-all"
				>
					Login / Create Account
				</button>
			</div>
		</div>
	{:else}
		{#if error}
			<div class="bg-red-500/20 border border-red-500/30 p-4 text-red-300">
				{error}
			</div>
		{/if}

		{#if loading}
			<div class="flex justify-center items-center py-12">
				<div class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-primary-400"></div>
			</div>
		{:else if stats}
			<!-- Revenue Overview -->
			<section class="space-y-4">
				<h2 class="text-xl font-semibold text-white">Revenue Overview</h2>
				<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
					<div class="bg-surface-elevated border border-neutral-800 p-6">
						<p class="text-neutral-500 text-sm">Total Revenue</p>
						<p class="text-3xl font-bold text-primary-400 mt-1">
							${formatRevenue(stats.total_revenue_e9s)}
						</p>
					</div>
					<div class="bg-surface-elevated border border-neutral-800 p-6">
						<p class="text-neutral-500 text-sm">Token Balance</p>
						<p class="text-3xl font-bold text-white mt-1">
							{formatBalance(tokenBalance)}
						</p>
					</div>
					<div class="bg-surface-elevated border border-neutral-800 p-6">
						<p class="text-neutral-500 text-sm">Total Contracts</p>
						<p class="text-3xl font-bold text-white mt-1">
							{stats.total_contracts}
						</p>
					</div>
					<div class="bg-surface-elevated border border-neutral-800 p-6">
						<p class="text-neutral-500 text-sm">Pending Contracts</p>
						<p class="text-3xl font-bold text-white mt-1">
							{stats.pending_contracts}
						</p>
					</div>
				</div>
			</section>

			<!-- Revenue Trend (last 12 months) -->
		{#if revenueByMonth.length > 0}
			{@const maxRevenue = Math.max(...revenueByMonth.map(m => m.revenue_e9s), 1)}
			{@const chartHeight = 120}
			{@const barWidth = Math.min(40, Math.floor(600 / revenueByMonth.length) - 4)}
			{@const gap = 4}
			{@const totalWidth = revenueByMonth.length * (barWidth + gap)}
			<section class="space-y-4">
				<h2 class="text-xl font-semibold text-white">Revenue Trend</h2>
				<div class="bg-surface-elevated border border-neutral-800 p-6">
					<svg
						viewBox="0 0 {totalWidth} {chartHeight + 30}"
						class="w-full overflow-visible"
						aria-label="Monthly revenue chart"
					>
						{#each revenueByMonth as month, i}
							{@const barH = Math.max(2, (month.revenue_e9s / maxRevenue) * chartHeight)}
							{@const x = i * (barWidth + gap)}
							{@const y = chartHeight - barH}
							<g>
								<rect
									x={x}
									y={y}
									width={barWidth}
									height={barH}
									class="fill-primary-500/80 hover:fill-primary-400 transition-colors"
									rx="2"
								>
									<title>{month.month}: ${(month.revenue_e9s / 1e9).toFixed(2)} ({month.contract_count} contracts)</title>
								</rect>
								<text
									x={x + barWidth / 2}
									y={chartHeight + 14}
									text-anchor="middle"
									class="fill-neutral-500 text-[8px]"
									font-size="8"
								>{month.month.slice(5)}</text>
							</g>
						{/each}
					</svg>
					<p class="text-xs text-neutral-500 mt-2">Monthly revenue (last 12 months). Hover bars for details.</p>
				</div>
			</section>
		{/if}

		<!-- Offerings -->
			<section class="space-y-4">
				<h2 class="text-xl font-semibold text-white">Offerings</h2>
				<div class="bg-surface-elevated border border-neutral-800 p-6">
					<p class="text-neutral-500 text-sm">Total Offerings</p>
					<p class="text-3xl font-bold text-white mt-1">{stats.offerings_count}</p>
				</div>
			</section>

			<!-- Bandwidth Usage -->
			<section class="space-y-4">
				<h2 class="text-xl font-semibold text-white">Bandwidth Usage</h2>
				{#if bandwidthStats.length === 0}
					<div class="bg-surface-elevated border border-neutral-800 p-6 text-neutral-500 text-sm">
						No bandwidth data
					</div>
				{:else}
					<div class="bg-surface-elevated border border-neutral-800 overflow-x-auto">
						<table class="w-full text-sm">
							<thead>
								<tr class="border-b border-neutral-800">
									<th class="text-left text-neutral-500 font-medium px-4 py-3">Contract</th>
									<th class="text-left text-neutral-500 font-medium px-4 py-3">Gateway</th>
									<th class="text-right text-neutral-500 font-medium px-4 py-3">In</th>
									<th class="text-right text-neutral-500 font-medium px-4 py-3">Out</th>
									<th class="text-right text-neutral-500 font-medium px-4 py-3">Last Updated</th>
								</tr>
							</thead>
							<tbody>
								{#each bandwidthStats as row}
									<tr class="border-b border-neutral-800/50 hover:bg-neutral-800/30 transition-colors">
										<td class="px-4 py-3 font-mono text-neutral-300">
											{row.contractId.slice(0, 8)}...
										</td>
										<td class="px-4 py-3 text-neutral-300">{row.gatewaySlug}</td>
										<td class="px-4 py-3 text-right text-neutral-300">{formatBytes(row.bytesIn)}</td>
										<td class="px-4 py-3 text-right text-neutral-300">{formatBytes(row.bytesOut)}</td>
										<td class="px-4 py-3 text-right text-neutral-400 text-xs">{formatNsTimestamp(row.lastUpdatedNs)}</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				{/if}
			</section>

			<!-- Customer Feedback -->
			{#if feedbackStats && feedbackStats.total_responses > 0}
				<section class="space-y-4">
					<h2 class="text-xl font-semibold text-white">Customer Feedback</h2>
					<div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
						<!-- Service Match Rate -->
						<div class="bg-surface-elevated border border-neutral-800 p-6 space-y-3">
							<div class="flex items-center justify-between">
								<p class="text-neutral-500 text-sm">Service Match Rate</p>
								<p class="text-lg font-bold text-white">{feedbackStats.service_match_rate_pct.toFixed(0)}%</p>
							</div>
							<div class="w-full bg-neutral-800 h-2 overflow-hidden">
								<div
									class="h-full bg-emerald-500 transition-all duration-500"
									style="width: {feedbackStats.service_match_rate_pct}%"
								></div>
							</div>
							<p class="text-neutral-600 text-xs">
								{feedbackStats.service_matched_yes} of {feedbackStats.service_matched_yes + feedbackStats.service_matched_no} said service matched description
							</p>
						</div>

						<!-- Would Rent Again Rate -->
						<div class="bg-surface-elevated border border-neutral-800 p-6 space-y-3">
							<div class="flex items-center justify-between">
								<p class="text-neutral-500 text-sm">Would Rent Again</p>
								<p class="text-lg font-bold text-white">{feedbackStats.would_rent_again_rate_pct.toFixed(0)}%</p>
							</div>
							<div class="w-full bg-neutral-800 h-2 overflow-hidden">
								<div
									class="h-full bg-primary-500 transition-all duration-500"
									style="width: {feedbackStats.would_rent_again_rate_pct}%"
								></div>
							</div>
							<p class="text-neutral-600 text-xs">
								{feedbackStats.would_rent_again_yes} of {feedbackStats.would_rent_again_yes + feedbackStats.would_rent_again_no} would rent again
							</p>
						</div>
					</div>
					<div class="bg-surface-elevated border border-neutral-800 p-6">
						<p class="text-neutral-500 text-sm">Total Reviews</p>
						<p class="text-3xl font-bold text-white mt-1">{feedbackStats.total_responses}</p>
					</div>
				</section>
			{/if}
		{/if}
	{/if}
</div>
