<script lang="ts">
	import type { ProviderTrustMetrics } from '$lib/services/api';
	import { formatDuration } from '$lib/utils/contract-format';

	interface Props {
		metrics: ProviderTrustMetrics;
	}

	let { metrics }: Props = $props();

	// Trust score color based on value
	function getScoreColor(score: number): string {
		if (score >= 80) return 'text-green-400';
		if (score >= 60) return 'text-yellow-400';
		return 'text-red-400';
	}

	function getScoreBgColor(score: number): string {
		if (score >= 80) return 'bg-green-500/20 border-green-500/50';
		if (score >= 60) return 'bg-yellow-500/20 border-yellow-500/50';
		return 'bg-red-500/20 border-red-500/50';
	}

	function getScoreLabel(score: number): string {
		if (score >= 80) return 'Reliable';
		if (score >= 60) return 'Caution';
		return 'High Risk';
	}

	// Format last active time
	function formatLastActive(ns: number): string {
		if (ns === 0) return 'Never';
		const now = Date.now() * 1_000_000; // Convert to nanoseconds
		const diffNs = now - ns;
		return formatDuration(diffNs) + ' ago';
	}

	// Format currency value
	function formatValue(e9s: number): string {
		const value = e9s / 1_000_000_000;
		if (value >= 1000) return `$${(value / 1000).toFixed(1)}k`;
		return `$${value.toFixed(0)}`;
	}

	// Convert trust_score from bigint to number safely
	const trustScore = $derived(Number(metrics.trust_score));

	// Provider tenure badge helpers
	function getTenureBadgeColor(tenure: string): string {
		if (tenure === 'established') return 'bg-green-500/20 border-green-500/50 text-green-300';
		if (tenure === 'growing') return 'bg-blue-500/20 border-blue-500/50 text-blue-300';
		return 'bg-purple-500/20 border-purple-500/50 text-purple-300';
	}

	function getTenureLabel(tenure: string): string {
		if (tenure === 'established') return 'Established Provider';
		if (tenure === 'growing') return 'Growing Provider';
		return 'New Provider';
	}

	// Format contract duration ratio as descriptive text
	function formatDurationRatio(ratio: number | undefined): string {
		if (ratio === undefined || ratio === null) return 'N/A';
		const percentage = (ratio * 100).toFixed(0);
		return `Contracts run ${percentage}% of expected duration`;
	}

	// Determine if no-response rate is concerning
	function isNoResponseConcerning(rate: number | undefined): boolean {
		return rate !== undefined && rate !== null && rate > 10;
	}
</script>

<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/10">
	<!-- Trust Score Header -->
	<div class="flex items-center justify-between mb-6">
		<h3 class="text-xl font-bold">Trust Score</h3>
		<div
			class="flex items-center gap-3 px-4 py-2 rounded-full border {getScoreBgColor(trustScore)}"
		>
			<span class="text-3xl font-bold {getScoreColor(trustScore)}">{trustScore}</span>
			<span class="text-sm text-white/70">/100</span>
			<span class="text-sm font-medium {getScoreColor(trustScore)}">{getScoreLabel(trustScore)}</span
			>
		</div>
	</div>

	<!-- Core Metrics Grid -->
	<div class="grid grid-cols-2 md:grid-cols-3 gap-4 mb-6">
		<div class="bg-white/5 rounded-lg p-3">
			<div class="text-xs text-white/50 mb-1">Time to Delivery</div>
			<div class="text-lg font-semibold">
				{#if metrics.time_to_delivery_hours}
					{metrics.time_to_delivery_hours < 1
						? `${Math.round(metrics.time_to_delivery_hours * 60)}m`
						: `${metrics.time_to_delivery_hours.toFixed(1)}h`}
				{:else}
					<span class="text-white/40">N/A</span>
				{/if}
			</div>
		</div>

		<div class="bg-white/5 rounded-lg p-3">
			<div class="text-xs text-white/50 mb-1">Completion Rate</div>
			<div class="text-lg font-semibold">{metrics.completion_rate_pct.toFixed(0)}%</div>
		</div>

		<div class="bg-white/5 rounded-lg p-3">
			<div class="text-xs text-white/50 mb-1">Repeat Customers</div>
			<div class="text-lg font-semibold">{metrics.repeat_customer_count}</div>
		</div>

		<div class="bg-white/5 rounded-lg p-3">
			<div class="text-xs text-white/50 mb-1">Total Contracts</div>
			<div class="text-lg font-semibold">{metrics.total_contracts}</div>
		</div>

		<div class="bg-white/5 rounded-lg p-3">
			<div class="text-xs text-white/50 mb-1">Active Value</div>
			<div class="text-lg font-semibold">{formatValue(metrics.active_contract_value_e9s)}</div>
		</div>

		<div class="bg-white/5 rounded-lg p-3">
			<div class="text-xs text-white/50 mb-1">Last Active</div>
			<div class="text-lg font-semibold">{formatLastActive(metrics.last_active_ns)}</div>
		</div>
	</div>

	<!-- Provider Tenure Badge -->
	<div
		class="flex items-center gap-2 px-3 py-2 border rounded-lg mb-4 {getTenureBadgeColor(
			metrics.provider_tenure
		)}"
	>
		<span class="text-sm font-medium">{getTenureLabel(metrics.provider_tenure)}</span>
		{#if metrics.provider_tenure === 'new'}
			<span class="text-xs opacity-70">(&lt;5 contracts)</span>
		{:else if metrics.provider_tenure === 'growing'}
			<span class="text-xs opacity-70">(5-20 contracts)</span>
		{:else}
			<span class="text-xs opacity-70">(&gt;20 contracts)</span>
		{/if}
	</div>

	<!-- Critical Flags Section -->
	{#if metrics.has_critical_flags && metrics.critical_flag_reasons.length > 0}
		<div class="border-t border-white/10 pt-4">
			<h4 class="text-sm font-semibold text-red-400 mb-3 flex items-center gap-2">
				<span>&#x26A0;</span> Red Flags Detected
			</h4>
			<ul class="space-y-2">
				{#each metrics.critical_flag_reasons as reason}
					<li
						class="flex items-start gap-2 text-sm text-red-300/80 bg-red-500/10 rounded px-3 py-2"
					>
						<span class="text-red-400 mt-0.5">&#x2022;</span>
						<span>{reason}</span>
					</li>
				{/each}
			</ul>
		</div>
	{/if}

	<!-- Response Time Details -->
	{#if metrics.avg_response_time_hours}
		<div class="border-t border-white/10 pt-4 mt-4">
			<div class="text-xs text-white/50 mb-1">Average Response Time</div>
			<div class="text-sm">
				{metrics.avg_response_time_hours < 1
					? `${Math.round(metrics.avg_response_time_hours * 60)} minutes`
					: `${metrics.avg_response_time_hours.toFixed(1)} hours`}
			</div>
		</div>
	{/if}

	<!-- Contract Duration Ratio -->
	{#if metrics.avg_contract_duration_ratio !== undefined}
		<div class="border-t border-white/10 pt-4 mt-4">
			<div class="text-xs text-white/50 mb-1">Contract Duration Performance</div>
			<div class="text-sm">{formatDurationRatio(metrics.avg_contract_duration_ratio)}</div>
		</div>
	{/if}

	<!-- No Response Rate with Warning -->
	{#if metrics.no_response_rate_pct !== undefined}
		<div class="border-t border-white/10 pt-4 mt-4">
			<div class="text-xs text-white/50 mb-1">No Response Rate</div>
			<div class="flex items-center gap-2">
				<div class="text-sm">{metrics.no_response_rate_pct.toFixed(1)}%</div>
				{#if isNoResponseConcerning(metrics.no_response_rate_pct)}
					<span
						class="text-xs px-2 py-0.5 bg-yellow-500/20 border border-yellow-500/50 text-yellow-300 rounded"
						>Concern: &gt;10%</span
					>
				{/if}
			</div>
		</div>
	{/if}
</div>
