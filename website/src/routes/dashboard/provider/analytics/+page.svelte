<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { page } from "$app/stores";
	import { navigateToLogin } from "$lib/utils/navigation";
	import {
		getProviderOfferingConversionStats,
		getProviderOfferingSatisfactionStats,
		hexEncode,
		type OfferingConversionStats,
		type OfferingSatisfactionStats,
	} from "$lib/services/api";
	import ProviderSetupBanner from "$lib/components/ProviderSetupBanner.svelte";
	import { signRequest } from "$lib/services/auth-api";
	import { authStore } from "$lib/stores/auth";
	import { Ed25519KeyIdentity } from "@dfinity/identity";
	import { getProviderOnboarding } from "$lib/services/api";

	let conversionStats = $state<OfferingConversionStats[]>([]);
	let satisfactionStats = $state<OfferingSatisfactionStats[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let isAuthenticated = $state(false);
	let onboardingCompleted = $state<boolean | null>(null);
	let unsubscribeAuth: (() => void) | null = null;

	type SortKey = 'offeringId' | 'views7d' | 'views30d' | 'rentals7d' | 'rentals30d' | 'conversionRate30d' | 'revenue30dE9s';
	let sortKey = $state<SortKey>('conversionRate30d');
	let sortAsc = $state(false);

	let sortedStats = $derived(
		[...conversionStats].sort((a, b) => {
			let av: string | number;
			let bv: string | number;
			switch (sortKey) {
				case 'offeringId': av = a.offeringId; bv = b.offeringId; break;
				case 'views7d': av = a.views7d; bv = b.views7d; break;
				case 'views30d': av = a.views30d; bv = b.views30d; break;
				case 'rentals7d': av = a.rentals7d; bv = b.rentals7d; break;
				case 'rentals30d': av = a.rentals30d; bv = b.rentals30d; break;
				case 'conversionRate30d': av = a.conversionRate30d; bv = b.conversionRate30d; break;
				case 'revenue30dE9s': av = a.revenue30dE9s; bv = b.revenue30dE9s; break;
			}
			const cmp = av < bv ? -1 : av > bv ? 1 : 0;
			return sortAsc ? cmp : -cmp;
		})
	);

	let totalViews30d = $derived(conversionStats.reduce((s, r) => s + r.views30d, 0));
	let totalRentals30d = $derived(conversionStats.reduce((s, r) => s + r.rentals30d, 0));
	let overallConversionRate = $derived(
		totalViews30d > 0 ? (totalRentals30d / totalViews30d) * 100 : 0
	);
	let totalRevenue30d = $derived(conversionStats.reduce((s, r) => s + r.revenue30dE9s, 0));

	let totalFeedback = $derived(satisfactionStats.reduce((s, r) => s + r.totalFeedback, 0));
	let overallSatisfactionRate = $derived(
		satisfactionStats.length > 0 && totalFeedback > 0
			? satisfactionStats.reduce((s, r) => s + r.satisfactionRatePct * r.totalFeedback, 0) / totalFeedback
			: 0
	);

	let sortedSatisfactionStats = $derived(
		[...satisfactionStats].sort((a, b) => b.satisfactionRatePct - a.satisfactionRatePct)
	);

	function toggleSort(key: SortKey) {
		if (sortKey === key) {
			sortAsc = !sortAsc;
		} else {
			sortKey = key;
			sortAsc = false;
		}
	}

	function sortIndicator(key: SortKey): string {
		if (sortKey !== key) return '';
		return sortAsc ? ' ↑' : ' ↓';
	}

	function conversionClass(rate: number): string {
		if (rate >= 5) return 'text-emerald-400';
		if (rate >= 1) return 'text-yellow-400';
		return 'text-red-400';
	}

	function satisfactionClass(rate: number): string {
		if (rate >= 75) return 'text-emerald-400';
		if (rate >= 50) return 'text-yellow-400';
		return 'text-red-400';
	}

	function formatRevenue(e9s: number): string {
		return (e9s / 1_000_000_000).toFixed(4);
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
				error = "You must be authenticated to view analytics";
				return;
			}

			if (!(info.identity instanceof Ed25519KeyIdentity)) {
				error = "Ed25519 identity required";
				return;
			}

			const providerHex = hexEncode(info.publicKeyBytes);

			const [conversionStats_, satisfactionStats_, onboarding] = await Promise.all([
				(async () => {
					const signed = await signRequest(
						info.identity as Ed25519KeyIdentity,
						"GET",
						`/api/v1/providers/${providerHex}/offering-conversion-stats`,
					);
					return getProviderOfferingConversionStats(providerHex, signed.headers).catch(() => []);
				})(),
				(async () => {
					const signed = await signRequest(
						info.identity as Ed25519KeyIdentity,
						"GET",
						`/api/v1/providers/${providerHex}/offering-satisfaction-stats`,
					);
					return getProviderOfferingSatisfactionStats(providerHex, signed.headers).catch(() => []);
				})(),
				getProviderOnboarding(providerHex).catch(() => null),
			]);

			conversionStats = conversionStats_;
			satisfactionStats = satisfactionStats_;
			onboardingCompleted = !!onboarding?.onboarding_completed_at;
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load analytics data";
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
		<h1 class="text-2xl font-bold text-white tracking-tight">Offering Analytics</h1>
		<p class="text-neutral-500">Conversion rates: views to rentals, per offering</p>
	</header>

	{#if !isAuthenticated}
		<div class="card p-8 border border-neutral-800 text-center">
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">📊</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-neutral-400">
					Login to view your offering conversion rates and analytics.
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
		{:else}
			<!-- Summary cards -->
			<section class="space-y-4">
				<h2 class="text-xl font-semibold text-white">30-Day Summary</h2>
				<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
					<div class="bg-surface-elevated border border-neutral-800 p-6">
						<p class="text-neutral-500 text-sm">Total Views</p>
						<p class="text-3xl font-bold text-white mt-1">{totalViews30d.toLocaleString()}</p>
						<p class="text-neutral-600 text-xs mt-1">Across all offerings</p>
					</div>
					<div class="bg-surface-elevated border border-neutral-800 p-6">
						<p class="text-neutral-500 text-sm">Total Rentals</p>
						<p class="text-3xl font-bold text-white mt-1">{totalRentals30d.toLocaleString()}</p>
						<p class="text-neutral-600 text-xs mt-1">Contract requests received</p>
					</div>
					<div class="bg-surface-elevated border border-neutral-800 p-6">
						<p class="text-neutral-500 text-sm">Overall Conversion</p>
						<p class="text-3xl font-bold mt-1 {conversionClass(overallConversionRate)}">
							{overallConversionRate.toFixed(2)}%
						</p>
						<p class="text-neutral-600 text-xs mt-1">Views to rentals</p>
					</div>
					<div class="bg-surface-elevated border border-neutral-800 p-6 border-l-2 border-l-emerald-500/50">
						<p class="text-neutral-500 text-sm">Revenue</p>
						<p class="text-3xl font-bold text-emerald-400 mt-1">{formatRevenue(totalRevenue30d)} ICP</p>
						<p class="text-neutral-600 text-xs mt-1">From rentals this month</p>
					</div>
				</div>
			</section>

			<!-- Per-offering breakdown -->
			<section class="space-y-4">
				<h2 class="text-xl font-semibold text-white">Per-Offering Breakdown</h2>
				{#if sortedStats.length === 0}
					<div class="bg-surface-elevated border border-neutral-800 p-8 text-center">
						<p class="text-neutral-400 text-sm">No offerings found. Create offerings to see conversion data.</p>
					</div>
				{:else}
					<div class="bg-surface-elevated border border-neutral-800 overflow-x-auto">
						<table class="w-full text-sm">
							<thead>
								<tr class="border-b border-neutral-800">
									<th
										class="text-left text-neutral-500 font-medium px-4 py-3 cursor-pointer select-none hover:text-neutral-300"
										onclick={() => toggleSort('offeringId')}
									>Offering{sortIndicator('offeringId')}</th>
									<th
										class="text-right text-neutral-500 font-medium px-4 py-3 cursor-pointer select-none hover:text-neutral-300"
										onclick={() => toggleSort('views7d')}
									>Views 7d{sortIndicator('views7d')}</th>
									<th
										class="text-right text-neutral-500 font-medium px-4 py-3 cursor-pointer select-none hover:text-neutral-300"
										onclick={() => toggleSort('views30d')}
									>Views 30d{sortIndicator('views30d')}</th>
									<th
										class="text-right text-neutral-500 font-medium px-4 py-3 cursor-pointer select-none hover:text-neutral-300"
										onclick={() => toggleSort('rentals7d')}
									>Rentals 7d{sortIndicator('rentals7d')}</th>
									<th
										class="text-right text-neutral-500 font-medium px-4 py-3 cursor-pointer select-none hover:text-neutral-300"
										onclick={() => toggleSort('rentals30d')}
									>Rentals 30d{sortIndicator('rentals30d')}</th>
									<th
										class="text-right text-neutral-500 font-medium px-4 py-3 cursor-pointer select-none hover:text-neutral-300"
										onclick={() => toggleSort('conversionRate30d')}
									>Conversion 30d{sortIndicator('conversionRate30d')}</th>
									<th
										class="text-right text-neutral-500 font-medium px-4 py-3 cursor-pointer select-none hover:text-neutral-300"
										onclick={() => toggleSort('revenue30dE9s')}
									>Revenue 30d (ICP){sortIndicator('revenue30dE9s')}</th>
								</tr>
							</thead>
							<tbody>
								{#each sortedStats as row}
									<tr class="border-b border-neutral-800/50 hover:bg-neutral-800/30 transition-colors">
										<td class="px-4 py-3">
											<div class="font-medium text-neutral-200">{row.offerName}</div>
											<div class="flex items-center gap-2 mt-0.5">
												<span class="text-neutral-500 font-mono text-xs">{row.offeringId}</span>
												<span class="px-1.5 py-0.5 text-xs font-medium bg-neutral-700/50 text-neutral-400 border border-neutral-600/30">
													{row.productType}
												</span>
											</div>
										</td>
										<td class="px-4 py-3 text-right text-neutral-300">{row.views7d.toLocaleString()}</td>
										<td class="px-4 py-3 text-right text-neutral-300">{row.views30d.toLocaleString()}</td>
										<td class="px-4 py-3 text-right text-neutral-300">{row.rentals7d.toLocaleString()}</td>
										<td class="px-4 py-3 text-right text-neutral-300">{row.rentals30d.toLocaleString()}</td>
										<td class="px-4 py-3 text-right font-semibold {conversionClass(row.conversionRate30d)}">
											{row.conversionRate30d.toFixed(2)}%
										</td>
										<td class="px-4 py-3 text-right text-emerald-400 font-mono">
											{formatRevenue(row.revenue30dE9s)}
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
					<p class="text-xs text-neutral-600">
						Conversion rate: green &ge;5%, yellow 1–5%, red &lt;1%. Sorted by conversion rate (highest first) by default.
					</p>
				{/if}
			</section>
		<!-- Tenant Satisfaction -->
		<section class="space-y-4">
			<h2 class="text-xl font-semibold text-white">Tenant Satisfaction</h2>
			<p class="text-neutral-500 text-sm">Based on post-contract boolean feedback from tenants</p>

			<!-- Satisfaction summary card -->
			<div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
				<div class="bg-surface-elevated border border-neutral-800 p-6">
					<p class="text-neutral-500 text-sm">Overall Satisfaction Rate</p>
					<p class="text-3xl font-bold mt-1 {satisfactionClass(overallSatisfactionRate)}">
						{overallSatisfactionRate.toFixed(1)}%
					</p>
					<p class="text-neutral-600 text-xs mt-1">Weighted across all offerings</p>
				</div>
				<div class="bg-surface-elevated border border-neutral-800 p-6">
					<p class="text-neutral-500 text-sm">Total Feedback Received</p>
					<p class="text-3xl font-bold text-white mt-1">{totalFeedback.toLocaleString()}</p>
					<p class="text-neutral-600 text-xs mt-1">Tenant responses across all offerings</p>
				</div>
			</div>

			<!-- Per-offering satisfaction table -->
			{#if sortedSatisfactionStats.length === 0}
				<div class="bg-surface-elevated border border-neutral-800 p-8 text-center">
					<p class="text-neutral-400 text-sm">No offerings found. Create offerings to see satisfaction data.</p>
				</div>
			{:else}
				<div class="bg-surface-elevated border border-neutral-800 overflow-x-auto">
					<table class="w-full text-sm">
						<thead>
							<tr class="border-b border-neutral-800">
								<th class="text-left text-neutral-500 font-medium px-4 py-3">Offering</th>
								<th class="text-right text-neutral-500 font-medium px-4 py-3">Feedback</th>
								<th class="text-right text-neutral-500 font-medium px-4 py-3">Service Matched</th>
								<th class="text-right text-neutral-500 font-medium px-4 py-3">Would Rent Again</th>
								<th class="text-right text-neutral-500 font-medium px-4 py-3">Satisfaction Rate</th>
							</tr>
						</thead>
						<tbody>
							{#each sortedSatisfactionStats as row}
								<tr class="border-b border-neutral-800/50 hover:bg-neutral-800/30 transition-colors">
									<td class="px-4 py-3">
										<div class="font-medium text-neutral-200">{row.offerName}</div>
										<div class="text-neutral-500 font-mono text-xs mt-0.5">{row.offeringId}</div>
									</td>
									<td class="px-4 py-3 text-right text-neutral-300">
										{row.totalFeedback.toLocaleString()}
									</td>
									<td class="px-4 py-3 text-right text-neutral-300">
										{#if row.totalFeedback > 0}
											{row.serviceMatchedYes}/{row.totalFeedback}
										{:else}
											<span class="text-neutral-600">—</span>
										{/if}
									</td>
									<td class="px-4 py-3 text-right text-neutral-300">
										{#if row.totalFeedback > 0}
											{row.wouldRentAgainYes}/{row.totalFeedback}
										{:else}
											<span class="text-neutral-600">—</span>
										{/if}
									</td>
									<td class="px-4 py-3 text-right font-semibold {satisfactionClass(row.satisfactionRatePct)}">
										{#if row.totalFeedback > 0}
											{row.satisfactionRatePct.toFixed(1)}%
										{:else}
											<span class="text-neutral-600 font-normal">No data</span>
										{/if}
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
				<p class="text-xs text-neutral-600">
					Satisfaction rate: composite of "service matched description" and "would rent again" responses. Green &ge;75%, yellow 50–75%, red &lt;50%.
				</p>
			{/if}
		</section>
	{/if}
</div>
