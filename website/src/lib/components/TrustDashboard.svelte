<script lang="ts">
	import type { ProviderTrustMetrics, ProviderResponseMetrics, ProviderHealthSummary } from '$lib/services/api';
	import { formatDuration } from '$lib/utils/contract-format';

	interface Props {
		metrics: ProviderTrustMetrics;
		responseMetrics?: ProviderResponseMetrics | null;
		healthSummary?: ProviderHealthSummary | null;
	}

	let { metrics, responseMetrics = null, healthSummary = null }: Props = $props();

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
		if (tenure === 'growing') return 'bg-primary-500/20 border-primary-500/50 text-primary-300';
		return 'bg-purple-500/20 border-purple-500/50 text-primary-300';
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

	// Get abandonment velocity status
	function getVelocityStatus(velocity: number | undefined): 'good' | 'warning' | 'critical' | null {
		if (velocity === undefined || velocity === null) return null;
		if (velocity >= 2.0) return 'critical';
		if (velocity >= 1.5) return 'warning';
		return 'good';
	}

	// SLA compliance color based on percentage
	function getSlaComplianceColor(percent: number): string {
		if (percent >= 95) return 'text-green-400';
		if (percent >= 80) return 'text-yellow-400';
		return 'text-red-400';
	}

	// Format response time
	function formatResponseTime(hours: number | null): string {
		if (hours === null) return 'N/A';
		if (hours < 1) return `${Math.round(hours * 60)}m`;
		return `${hours.toFixed(1)}h`;
	}

	// Uptime percentage color based on value
	function getUptimeColor(uptime: number): string {
		if (uptime >= 99) return 'text-green-400';
		if (uptime >= 95) return 'text-green-300';
		if (uptime >= 90) return 'text-yellow-400';
		return 'text-red-400';
	}

	// Format average latency
	function formatLatency(ms: number | null | undefined): string {
		if (ms === null || ms === undefined) return 'N/A';
		return `${ms.toFixed(0)}ms`;
	}
</script>

<div class="card p-6 border border-neutral-800">
	<!-- Trust Score Header -->
	<div class="flex items-center justify-between mb-6">
		<h3 class="text-xl font-bold">Trust Score</h3>
		<div
			class="flex items-center gap-3 px-4 py-2 rounded-full border {getScoreBgColor(trustScore)}"
		>
			<span class="text-3xl font-bold {getScoreColor(trustScore)}">{trustScore}</span>
			<span class="text-sm text-neutral-400">/100</span>
			<span class="text-sm font-medium {getScoreColor(trustScore)}">{getScoreLabel(trustScore)}</span
			>
		</div>
	</div>

	<!-- Core Metrics Grid -->
	<div class="grid grid-cols-2 md:grid-cols-3 gap-4 mb-6">
		<div class="bg-surface-elevated  p-3">
			<div class="text-xs text-neutral-500 mb-1">Time to Delivery</div>
			<div class="text-lg font-semibold">
				{#if metrics.time_to_delivery_hours}
					{metrics.time_to_delivery_hours < 1
						? `${Math.round(metrics.time_to_delivery_hours * 60)}m`
						: `${metrics.time_to_delivery_hours.toFixed(1)}h`}
				{:else}
					<span class="text-neutral-600">N/A</span>
				{/if}
			</div>
		</div>

		<div class="bg-surface-elevated  p-3">
			<div class="text-xs text-neutral-500 mb-1">Completion Rate</div>
			<div class="text-lg font-semibold">{metrics.completion_rate_pct.toFixed(0)}%</div>
		</div>

		<div class="bg-surface-elevated  p-3">
			<div class="text-xs text-neutral-500 mb-1">Repeat Customers</div>
			<div class="text-lg font-semibold">{metrics.repeat_customer_count}</div>
		</div>

		<div class="bg-surface-elevated  p-3">
			<div class="text-xs text-neutral-500 mb-1">Total Contracts</div>
			<div class="text-lg font-semibold">{metrics.total_contracts}</div>
		</div>

		<div class="bg-surface-elevated  p-3">
			<div class="text-xs text-neutral-500 mb-1">Active Value</div>
			<div class="text-lg font-semibold">{formatValue(metrics.active_contract_value_e9s)}</div>
		</div>

		<div class="bg-surface-elevated  p-3">
			<div class="text-xs text-neutral-500 mb-1">Last Active</div>
			<div class="text-lg font-semibold">{formatLastActive(metrics.last_active_ns)}</div>
		</div>
	</div>

	<!-- Provider Tenure Badge -->
	<div
		class="flex items-center gap-2 px-3 py-2 border  mb-4 {getTenureBadgeColor(
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

	<!-- Renter Feedback -->
	{#if metrics.feedback_count > 0}
		<div class="border-t border-neutral-800 pt-4 mt-4">
			<h4 class="text-sm font-semibold mb-3">Renter Feedback ({metrics.feedback_count} reviews)</h4>
			<div class="grid grid-cols-2 gap-3">
				<div class="bg-surface-elevated p-3">
					<div class="text-xs text-neutral-500 mb-1">Service Matched Description</div>
					<div class="text-lg font-semibold {(metrics.feedback_service_match_rate_pct ?? 0) >= 80 ? 'text-green-400' : (metrics.feedback_service_match_rate_pct ?? 0) >= 50 ? 'text-yellow-400' : 'text-red-400'}">
						{metrics.feedback_service_match_rate_pct?.toFixed(0) ?? 0}%
					</div>
				</div>
				<div class="bg-surface-elevated p-3">
					<div class="text-xs text-neutral-500 mb-1">Would Rent Again</div>
					<div class="text-lg font-semibold {(metrics.feedback_would_rent_again_rate_pct ?? 0) >= 80 ? 'text-green-400' : (metrics.feedback_would_rent_again_rate_pct ?? 0) >= 50 ? 'text-yellow-400' : 'text-red-400'}">
						{metrics.feedback_would_rent_again_rate_pct?.toFixed(0) ?? 0}%
					</div>
				</div>
			</div>
		</div>
	{/if}

	<!-- Uptime Metrics Section -->
	{#if healthSummary}
		<div class="border-t border-neutral-800 pt-4 mt-4">
			<h4 class="text-sm font-semibold mb-3">Infrastructure Uptime</h4>
			<div class="grid grid-cols-2 md:grid-cols-4 gap-3">
				<div class="bg-surface-elevated p-3">
					<div class="text-xs text-neutral-500 mb-1">Uptime (30d)</div>
					<div class="text-lg font-semibold {getUptimeColor(healthSummary.uptimePercent)}">
						{healthSummary.uptimePercent.toFixed(1)}%
					</div>
				</div>

				<div class="bg-surface-elevated p-3">
					<div class="text-xs text-neutral-500 mb-1">Avg Latency</div>
					<div class="text-lg font-semibold">
						{formatLatency(healthSummary.avgLatencyMs)}
					</div>
				</div>

				<div class="bg-surface-elevated p-3">
					<div class="text-xs text-neutral-500 mb-1">Health Checks</div>
					<div class="text-lg font-semibold">{healthSummary.totalChecks}</div>
				</div>

				<div class="bg-surface-elevated p-3">
					<div class="text-xs text-neutral-500 mb-1">Contracts Monitored</div>
					<div class="text-lg font-semibold">{healthSummary.contractsMonitored}</div>
				</div>
			</div>

			<!-- Health Check Breakdown -->
			<div class="mt-3 pt-3 border-t border-white/5">
				<div class="grid grid-cols-3 gap-3">
					<div>
						<div class="text-xs text-neutral-500">Healthy</div>
						<div class="text-sm font-medium text-green-400">
							{healthSummary.healthyChecks}
							<span class="text-neutral-600">({healthSummary.totalChecks > 0 ? ((healthSummary.healthyChecks / healthSummary.totalChecks) * 100).toFixed(1) : 0}%)</span>
						</div>
					</div>
					<div>
						<div class="text-xs text-neutral-500">Unhealthy</div>
						<div class="text-sm font-medium text-red-400">
							{healthSummary.unhealthyChecks}
							<span class="text-neutral-600">({healthSummary.totalChecks > 0 ? ((healthSummary.unhealthyChecks / healthSummary.totalChecks) * 100).toFixed(1) : 0}%)</span>
						</div>
					</div>
					<div>
						<div class="text-xs text-neutral-500">Unknown</div>
						<div class="text-sm font-medium text-neutral-400">
							{healthSummary.unknownChecks}
							<span class="text-neutral-600">({healthSummary.totalChecks > 0 ? ((healthSummary.unknownChecks / healthSummary.totalChecks) * 100).toFixed(1) : 0}%)</span>
						</div>
					</div>
				</div>
			</div>
		</div>
	{/if}

	<!-- Critical Flags Section -->
	{#if metrics.has_critical_flags && metrics.critical_flag_reasons.length > 0}
		<div class="border-t border-neutral-800 pt-4">
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
		<div class="border-t border-neutral-800 pt-4 mt-4">
			<div class="text-xs text-neutral-500 mb-1">Average Response Time</div>
			<div class="text-sm">
				{metrics.avg_response_time_hours < 1
					? `${Math.round(metrics.avg_response_time_hours * 60)} minutes`
					: `${metrics.avg_response_time_hours.toFixed(1)} hours`}
			</div>
		</div>
	{/if}

	<!-- Contract Duration Ratio -->
	{#if metrics.avg_contract_duration_ratio !== undefined}
		<div class="border-t border-neutral-800 pt-4 mt-4">
			<div class="text-xs text-neutral-500 mb-1">Contract Duration Performance</div>
			<div class="text-sm">{formatDurationRatio(metrics.avg_contract_duration_ratio)}</div>
		</div>
	{/if}

	<!-- No Response Rate with Warning -->
	{#if metrics.no_response_rate_pct !== undefined}
		<div class="border-t border-neutral-800 pt-4 mt-4">
			<div class="text-xs text-neutral-500 mb-1">No Response Rate</div>
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

	<!-- Abandonment Velocity -->
	{#if metrics.abandonment_velocity !== undefined}
		<div class="border-t border-neutral-800 pt-4 mt-4">
			<div class="text-xs text-neutral-500 mb-1">Abandonment Velocity</div>
			<div class="text-xs text-neutral-600 mb-2">
				Ratio of recent (30d) to baseline (31-90d) cancellation rate
			</div>
			<div class="flex items-center gap-2">
				<div
					class="text-sm {getVelocityStatus(metrics.abandonment_velocity) === 'critical'
						? 'text-red-400'
						: getVelocityStatus(metrics.abandonment_velocity) === 'warning'
							? 'text-yellow-400'
							: 'text-green-400'}"
				>
					{metrics.abandonment_velocity.toFixed(1)}x
				</div>
				{#if getVelocityStatus(metrics.abandonment_velocity) === 'critical'}
					<span
						class="text-xs px-2 py-0.5 bg-red-500/20 border border-red-500/50 text-red-300 rounded"
						>Critical: &gt;2.0x</span
					>
				{:else if getVelocityStatus(metrics.abandonment_velocity) === 'warning'}
					<span
						class="text-xs px-2 py-0.5 bg-yellow-500/20 border border-yellow-500/50 text-yellow-300 rounded"
						>Warning: &gt;1.5x</span
					>
				{/if}
			</div>
		</div>
	{/if}

	<!-- Support Response Metrics -->
	{#if responseMetrics}
		<div class="border-t border-neutral-800 pt-4 mt-4">
			<h4 class="text-sm font-semibold mb-3">Support Response</h4>
			<div class="grid grid-cols-2 gap-3">
				<div class="bg-surface-elevated  p-3">
					<div class="text-xs text-neutral-500 mb-1">Avg Response Time</div>
					<div class="text-lg font-semibold">
						{formatResponseTime(responseMetrics.avgResponseHours)}
					</div>
				</div>
				<div class="bg-surface-elevated  p-3">
					<div class="text-xs text-neutral-500 mb-1">SLA Compliance</div>
					<div class="text-lg font-semibold {getSlaComplianceColor(responseMetrics.slaCompliancePercent)}">
						{responseMetrics.slaCompliancePercent.toFixed(0)}%
					</div>
				</div>
			</div>
			{#if responseMetrics.breachCount30d > 0}
				<div class="mt-2 text-xs text-yellow-400">
					{responseMetrics.breachCount30d} SLA breach{responseMetrics.breachCount30d > 1 ? 'es' : ''} in last 30 days
				</div>
			{/if}

			<!-- Response Time Distribution -->
			{#if responseMetrics.distribution.totalResponses > 0}
				<div class="mt-4 pt-3 border-t border-white/5">
					<div class="text-xs text-neutral-500 mb-2">Response Time Distribution ({responseMetrics.distribution.totalResponses} responses)</div>
					<div class="space-y-2">
						{#each [
							{ label: '≤1h', pct: responseMetrics.distribution.within1hPct },
							{ label: '≤4h', pct: responseMetrics.distribution.within4hPct },
							{ label: '≤12h', pct: responseMetrics.distribution.within12hPct },
							{ label: '≤24h', pct: responseMetrics.distribution.within24hPct },
							{ label: '≤72h', pct: responseMetrics.distribution.within72hPct }
						] as bucket}
							<div class="flex items-center gap-2">
								<div class="w-10 text-xs text-neutral-500">{bucket.label}</div>
								<div class="flex-1 h-2 bg-surface-elevated rounded-full overflow-hidden">
									<div
										class="h-full rounded-full {bucket.pct >= 80 ? 'bg-green-500' : bucket.pct >= 50 ? 'bg-yellow-500' : 'bg-red-500'}"
										style="width: {bucket.pct}%"
									></div>
								</div>
								<div class="w-12 text-xs text-neutral-400 text-right">{bucket.pct.toFixed(0)}%</div>
							</div>
						{/each}
					</div>
				</div>
			{/if}
		</div>
	{/if}
</div>
