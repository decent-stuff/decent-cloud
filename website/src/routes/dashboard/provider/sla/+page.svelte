<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { debugLog } from '$lib/utils/debug';
	import AuthRequiredCard from '$lib/components/AuthRequiredCard.svelte';
	import {
		getProviderContracts,
		getProviderContractHealthSummary,
		getProviderSlaUptimeConfig,
		updateProviderSlaUptimeConfig,
		getProviderOfferings,
		getProviderSlaSummary,
		upsertProviderOfferingSliReports,
		hexEncode,
		type Contract,
		type ContractHealthSummary,
		type SlaUptimeConfig,
		type Offering,
		type ProviderSlaSummary
	} from '$lib/services/api';
	import { signRequest } from '$lib/services/auth-api';
	import { authStore } from '$lib/stores/auth';
	import { Ed25519KeyIdentity } from '@dfinity/identity';

	type ContractSlaRow = {
		contract: Contract;
		summary: ContractHealthSummary | null;
		error: string | null;
	};

	let rows = $state<ContractSlaRow[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let isAuthenticated = $state(false);
	let unsubscribeAuth: (() => void) | null = null;

	// Alert configuration state
	let slaConfig = $state<SlaUptimeConfig>({ uptimeThresholdPercent: 95, slaAlertWindowHours: 24 });
	let slaConfigLoading = $state(false);
	let slaConfigSaving = $state(false);
	let slaConfigError = $state<string | null>(null);
	let slaConfigSuccess = $state(false);

	// SLI reporting state
	let offerings = $state<Offering[]>([]);
	let providerSlaSummary = $state<ProviderSlaSummary | null>(null);
	let sliSelectedOfferingId = $state<number | null>(null);
	let sliSlaTarget = $state(99.9);
	let sliDate = $state(new Date().toISOString().slice(0, 10));
	let sliUptime = $state(100.0);
	let sliResponseSli = $state<string>('');
	let sliIncidents = $state(0);
	let sliNotes = $state('');
	let sliSubmitting = $state(false);
	let sliError = $state<string | null>(null);
	let sliSuccess = $state(false);

	const overallUptime = $derived(() => {
		const monitored = rows.filter((r) => r.summary && r.summary.totalChecks > 0);
		if (monitored.length === 0) return null;
		const sum = monitored.reduce((acc, r) => acc + r.summary!.uptimePercent, 0);
		return sum / monitored.length;
	});

	const sortedRows = $derived(
		[...rows].sort((a, b) => {
			const au = a.summary?.uptimePercent ?? 100;
			const bu = b.summary?.uptimePercent ?? 100;
			// Worst uptime first; no-data contracts go last
			if (a.summary === null && b.summary !== null) return 1;
			if (b.summary === null && a.summary !== null) return -1;
			return au - bu;
		})
	);

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
			if (!info || !(info.identity instanceof Ed25519KeyIdentity)) {
				error = 'You must be authenticated with an Ed25519 key to view SLA data';
				return;
			}

			const providerHex = hexEncode(info.publicKeyBytes);

			// Load SLA config, contracts, offerings, and provider SLA summary in parallel
			const [signedContracts] = await Promise.all([
				signRequest(info.identity, 'GET', `/api/v1/providers/${providerHex}/contracts`),
				loadSlaConfig(info.identity, providerHex),
				getProviderOfferings(providerHex).then((o) => { offerings = o; }).catch(() => {}),
				getProviderSlaSummary(providerHex, 30).then((s) => { providerSlaSummary = s; }).catch(() => {})
			]);
			const contracts = await getProviderContracts(signedContracts.headers, providerHex);

			// Fetch health summary for each contract in parallel
			const rowData = await Promise.all(
				contracts.map(async (contract): Promise<ContractSlaRow> => {
					try {
						const signed = await signRequest(
							info.identity as Ed25519KeyIdentity,
							'GET',
							`/api/v1/providers/${providerHex}/contracts/${contract.contract_id}/health`
						);
						const summary = await getProviderContractHealthSummary(
							providerHex,
							contract.contract_id,
							signed.headers
						);
						return { contract, summary, error: null };
					} catch (e) {
						return { contract, summary: null, error: e instanceof Error ? e.message : String(e) };
					}
				})
			);

			rows = rowData;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load SLA data';
		} finally {
			loading = false;
		}
	}

	async function loadSlaConfig(identity: Ed25519KeyIdentity, providerHex: string) {
		try {
			slaConfigLoading = true;
			slaConfigError = null;
			const signed = await signRequest(
				identity,
				'GET',
				`/api/v1/providers/${providerHex}/sla-uptime-config`
			);
			slaConfig = await getProviderSlaUptimeConfig(signed.headers, providerHex);
		} catch (e) {
			// Config not found is not fatal - keep defaults
			debugLog('SLA config not found, using defaults:', e);
		} finally {
			slaConfigLoading = false;
		}
	}

	async function saveSlaConfig() {
		try {
			slaConfigSaving = true;
			slaConfigError = null;
			slaConfigSuccess = false;

			const info = await authStore.getSigningIdentity();
			if (!info || !(info.identity instanceof Ed25519KeyIdentity)) {
				slaConfigError = 'You must be authenticated to update configuration';
				return;
			}

			const providerHex = hexEncode(info.publicKeyBytes);
			const signed = await signRequest(
				info.identity,
				'PUT',
				`/api/v1/providers/${providerHex}/sla-uptime-config`,
				slaConfig
			);
			await updateProviderSlaUptimeConfig(signed.headers, providerHex, slaConfig);
			slaConfigSuccess = true;
			setTimeout(() => { slaConfigSuccess = false; }, 3000);
		} catch (e) {
			slaConfigError = e instanceof Error ? e.message : 'Failed to save configuration';
		} finally {
			slaConfigSaving = false;
		}
	}

	async function submitSliReport() {
		if (!sliSelectedOfferingId) {
			sliError = 'Select an offering';
			return;
		}
		try {
			sliSubmitting = true;
			sliError = null;
			sliSuccess = false;

			const info = await authStore.getSigningIdentity();
			if (!info || !(info.identity instanceof Ed25519KeyIdentity)) {
				sliError = 'Authentication required';
				return;
			}
			const providerHex = hexEncode(info.publicKeyBytes);
			const signed = await signRequest(
				info.identity,
				'PUT',
				`/api/v1/providers/${providerHex}/offerings/${sliSelectedOfferingId}/sli-reports`,
				{
					slaTargetPercent: sliSlaTarget,
					reports: [
						{
							reportDate: sliDate,
							uptimePercent: sliUptime,
							responseSliPercent: sliResponseSli !== '' ? parseFloat(sliResponseSli) : undefined,
							incidentCount: sliIncidents,
							notes: sliNotes || undefined
						}
					]
				}
			);
			await upsertProviderOfferingSliReports(
				providerHex,
				sliSelectedOfferingId,
				sliSlaTarget,
				[
					{
						reportDate: sliDate,
						uptimePercent: sliUptime,
						responseSliPercent: sliResponseSli !== '' ? parseFloat(sliResponseSli) : undefined,
						incidentCount: sliIncidents,
						notes: sliNotes || undefined
					}
				],
				signed.headers
			);
			sliSuccess = true;
			sliNotes = '';
			setTimeout(() => { sliSuccess = false; }, 4000);
			// Refresh provider SLA summary
			getProviderSlaSummary(providerHex, 30).then((s) => { providerSlaSummary = s; }).catch(() => {});
		} catch (e) {
			sliError = e instanceof Error ? e.message : 'Failed to submit SLI report';
		} finally {
			sliSubmitting = false;
		}
	}

	onDestroy(() => {
		unsubscribeAuth?.();
	});

	function formatNsTimestamp(ns: number | null | undefined): string {
		if (ns == null) return '—';
		return new Date(ns / 1_000_000).toLocaleString();
	}

	function uptimeColor(pct: number): string {
		if (pct >= 99) return 'text-emerald-400';
		if (pct >= 95) return 'text-yellow-400';
		return 'text-red-400';
	}

	function uptimeBadgeClass(pct: number): string {
		if (pct >= 99) return 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/30';
		if (pct >= 95) return 'bg-yellow-500/20 text-yellow-400 border border-yellow-500/30';
		return 'bg-red-500/20 text-red-400 border border-red-500/30';
	}
</script>

<div class="space-y-8">
	<header>
		<h1 class="text-2xl font-bold text-white tracking-tight">SLA Monitor</h1>
		<p class="text-neutral-500">Per-contract uptime and health check history</p>
	</header>

	{#if !isAuthenticated}
		<AuthRequiredCard subtext="Login to view SLA monitoring data for your contracts." />
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
			<!-- Provider SLA Summary -->
			{#if providerSlaSummary && (providerSlaSummary.reports30d > 0 || providerSlaSummary.offeringsTracked > 0)}
				<section class="grid grid-cols-2 sm:grid-cols-4 gap-4">
					<div class="bg-surface-elevated border border-neutral-800 p-4">
						<p class="text-neutral-500 text-xs">Offerings Tracked</p>
						<p class="text-2xl font-bold text-white mt-1">{providerSlaSummary.offeringsTracked}</p>
					</div>
					<div class="bg-surface-elevated border border-neutral-800 p-4">
						<p class="text-neutral-500 text-xs">30d Compliance</p>
						<p class="text-2xl font-bold mt-1 {(providerSlaSummary.compliance30dPercent ?? 100) >= 99 ? 'text-emerald-400' : (providerSlaSummary.compliance30dPercent ?? 100) >= 95 ? 'text-yellow-400' : 'text-red-400'}">
							{providerSlaSummary.compliance30dPercent?.toFixed(1) ?? '—'}%
						</p>
					</div>
					<div class="bg-surface-elevated border border-neutral-800 p-4">
						<p class="text-neutral-500 text-xs">Breach Days (30d)</p>
						<p class="text-2xl font-bold mt-1 {providerSlaSummary.breachDays30d > 0 ? 'text-red-400' : 'text-emerald-400'}">{providerSlaSummary.breachDays30d}</p>
					</div>
					<div class="bg-surface-elevated border border-neutral-800 p-4">
						<p class="text-neutral-500 text-xs">Penalty Points</p>
						<p class="text-2xl font-bold mt-1 {providerSlaSummary.penaltyPoints > 10 ? 'text-red-400' : providerSlaSummary.penaltyPoints > 3 ? 'text-yellow-400' : 'text-emerald-400'}"
							title="Deducted from reliability score (0–45 scale). Lower is better."
						>
							{providerSlaSummary.penaltyPoints.toFixed(1)}
						</p>
					</div>
				</section>
			{/if}

			<!-- SLI Report Submission -->
			<section class="bg-surface-elevated border border-neutral-800 p-5">
				<h2 class="text-base font-semibold text-white mb-1">Submit SLI Report</h2>
				<p class="text-neutral-500 text-xs mb-4">
					Report daily uptime and SLI data for your offerings. This data is used to compute your reliability score and is displayed to potential customers.
				</p>
				{#if offerings.length === 0}
					<p class="text-neutral-500 text-sm">No offerings found. <a href="/dashboard/offerings" class="text-primary-400 hover:text-primary-300">Create an offering</a> first.</p>
				{:else}
					<div class="flex flex-wrap gap-4">
						<div class="flex flex-col gap-1 min-w-40">
							<label for="sli-offering" class="text-xs text-neutral-400 font-medium">Offering</label>
							<select
								id="sli-offering"
								bind:value={sliSelectedOfferingId}
								class="bg-neutral-900 border border-neutral-700 text-white text-sm px-3 py-1.5 focus:outline-none focus:border-primary-500"
							>
								<option value={null}>— select —</option>
								{#each offerings as o}
									<option value={o.id}>{o.offer_name} (#{o.id})</option>
								{/each}
							</select>
						</div>
						<div class="flex flex-col gap-1">
							<label for="sli-sla-target" class="text-xs text-neutral-400 font-medium">SLA Target (%)</label>
							<input
								id="sli-sla-target"
								type="number"
								min="1"
								max="100"
								step="0.01"
								bind:value={sliSlaTarget}
								class="w-28 bg-neutral-900 border border-neutral-700 text-white text-sm px-3 py-1.5 focus:outline-none focus:border-primary-500"
							/>
						</div>
						<div class="flex flex-col gap-1">
							<label for="sli-date" class="text-xs text-neutral-400 font-medium">Date</label>
							<input
								id="sli-date"
								type="date"
								bind:value={sliDate}
								class="bg-neutral-900 border border-neutral-700 text-white text-sm px-3 py-1.5 focus:outline-none focus:border-primary-500"
							/>
						</div>
						<div class="flex flex-col gap-1">
							<label for="sli-uptime" class="text-xs text-neutral-400 font-medium">Uptime (%)</label>
							<input
								id="sli-uptime"
								type="number"
								min="0"
								max="100"
								step="0.01"
								bind:value={sliUptime}
								class="w-28 bg-neutral-900 border border-neutral-700 text-white text-sm px-3 py-1.5 focus:outline-none focus:border-primary-500"
							/>
						</div>
						<div class="flex flex-col gap-1">
							<label for="sli-response" class="text-xs text-neutral-400 font-medium">Response SLI (%) <span class="text-neutral-600">optional</span></label>
							<input
								id="sli-response"
								type="number"
								min="0"
								max="100"
								step="0.1"
								bind:value={sliResponseSli}
								placeholder="—"
								class="w-28 bg-neutral-900 border border-neutral-700 text-white text-sm px-3 py-1.5 focus:outline-none focus:border-primary-500"
							/>
						</div>
						<div class="flex flex-col gap-1">
							<label for="sli-incidents" class="text-xs text-neutral-400 font-medium">Incidents</label>
							<input
								id="sli-incidents"
								type="number"
								min="0"
								bind:value={sliIncidents}
								class="w-20 bg-neutral-900 border border-neutral-700 text-white text-sm px-3 py-1.5 focus:outline-none focus:border-primary-500"
							/>
						</div>
						<div class="flex flex-col gap-1 grow">
							<label for="sli-notes" class="text-xs text-neutral-400 font-medium">Notes <span class="text-neutral-600">optional</span></label>
							<input
								id="sli-notes"
								type="text"
								bind:value={sliNotes}
								placeholder="e.g. scheduled maintenance"
								class="bg-neutral-900 border border-neutral-700 text-white text-sm px-3 py-1.5 focus:outline-none focus:border-primary-500"
							/>
						</div>
						<div class="flex flex-col gap-1 pb-5">
							<div class="text-xs text-neutral-400 font-medium">&nbsp;</div>
							<button
								onclick={submitSliReport}
								disabled={sliSubmitting || !sliSelectedOfferingId}
								class="px-4 py-1.5 text-sm bg-primary-600 text-white hover:bg-primary-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
							>
								{sliSubmitting ? 'Submitting...' : 'Submit'}
							</button>
						</div>
					</div>
					{#if sliSuccess}
						<p class="text-emerald-400 text-xs mt-2">SLI report submitted successfully.</p>
					{/if}
					{#if sliError}
						<p class="text-red-400 text-xs mt-2">{sliError}</p>
					{/if}
				{/if}
			</section>

			<!-- Alert Configuration -->
			<section class="bg-surface-elevated border border-neutral-800 p-5">
				<h2 class="text-base font-semibold text-white mb-1">Alert Configuration</h2>
				<p class="text-neutral-500 text-xs mb-4">Configure when SLA alerts are triggered for your contracts</p>
				{#if slaConfigLoading}
					<div class="text-neutral-500 text-sm">Loading configuration...</div>
				{:else}
					<div class="flex flex-wrap items-end gap-4">
						<div class="flex flex-col gap-1">
							<label for="uptime-threshold" class="text-xs text-neutral-400 font-medium">
								Uptime Threshold (%)
							</label>
							<input
								id="uptime-threshold"
								type="number"
								min="1"
								max="100"
								bind:value={slaConfig.uptimeThresholdPercent}
								class="w-28 bg-neutral-900 border border-neutral-700 text-white text-sm px-3 py-1.5 focus:outline-none focus:border-primary-500"
							/>
							<p class="text-neutral-600 text-xs">Alert when uptime drops below this value</p>
						</div>
						<div class="flex flex-col gap-1">
							<label for="alert-window" class="text-xs text-neutral-400 font-medium">
								Alert Window (hours)
							</label>
							<input
								id="alert-window"
								type="number"
								min="1"
								max="168"
								bind:value={slaConfig.slaAlertWindowHours}
								class="w-28 bg-neutral-900 border border-neutral-700 text-white text-sm px-3 py-1.5 focus:outline-none focus:border-primary-500"
							/>
							<p class="text-neutral-600 text-xs">Rolling window for uptime measurement</p>
						</div>
						<div class="flex flex-col gap-1 pb-5">
							<button
								onclick={saveSlaConfig}
								disabled={slaConfigSaving}
								class="px-4 py-1.5 text-sm bg-primary-600 text-white hover:bg-primary-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
							>
								{slaConfigSaving ? 'Saving...' : 'Save'}
							</button>
						</div>
					</div>
					{#if slaConfigSuccess}
						<p class="text-emerald-400 text-xs mt-2">Configuration saved.</p>
					{/if}
					{#if slaConfigError}
						<p class="text-red-400 text-xs mt-2">{slaConfigError}</p>
					{/if}
				{/if}
			</section>

			<!-- Summary header -->
			{#if overallUptime() !== null}
				<section class="grid grid-cols-1 sm:grid-cols-3 gap-4">
					<div class="bg-surface-elevated border border-neutral-800 p-6">
						<p class="text-neutral-500 text-sm">Overall Avg Uptime</p>
						<p class="text-3xl font-bold mt-1 {uptimeColor(overallUptime()!)}">
							{overallUptime()!.toFixed(2)}%
						</p>
						<p class="text-neutral-600 text-xs mt-1">Across all monitored contracts</p>
					</div>
					<div class="bg-surface-elevated border border-neutral-800 p-6">
						<p class="text-neutral-500 text-sm">Monitored Contracts</p>
						<p class="text-3xl font-bold text-white mt-1">
							{rows.filter((r) => r.summary && r.summary.totalChecks > 0).length}
						</p>
						<p class="text-neutral-600 text-xs mt-1">Contracts with health data</p>
					</div>
					<div class="bg-surface-elevated border border-neutral-800 p-6">
						<p class="text-neutral-500 text-sm">Total Contracts</p>
						<p class="text-3xl font-bold text-white mt-1">{rows.length}</p>
					</div>
				</section>
			{/if}

			<!-- Per-contract table -->
			{#if rows.length === 0}
				<div class="bg-surface-elevated border border-neutral-800 p-12 text-center">
					<p class="text-neutral-500">No contracts with health monitoring yet</p>
					<p class="text-neutral-600 text-sm mt-2">Health checks are recorded automatically for active contracts</p>
				</div>
			{:else}
				<section class="space-y-4">
					<h2 class="text-xl font-semibold text-white">Contracts</h2>
					<p class="text-neutral-500 text-sm">Sorted by uptime ascending — worst performing contracts shown first</p>
					<div class="bg-surface-elevated border border-neutral-800 overflow-x-auto">
						<table class="w-full text-sm">
							<thead>
								<tr class="border-b border-neutral-800">
									<th class="text-left text-neutral-500 font-medium px-4 py-3">Contract</th>
									<th class="text-left text-neutral-500 font-medium px-4 py-3">Status</th>
									<th class="text-right text-neutral-500 font-medium px-4 py-3">Uptime</th>
									<th class="text-right text-neutral-500 font-medium px-4 py-3">Total Checks</th>
									<th class="text-right text-neutral-500 font-medium px-4 py-3">Avg Latency</th>
									<th class="text-right text-neutral-500 font-medium px-4 py-3">Last Checked</th>
								</tr>
							</thead>
							<tbody>
								{#each sortedRows as row}
									<tr class="border-b border-neutral-800/50 hover:bg-neutral-800/30 transition-colors">
										<td class="px-4 py-3 font-mono text-xs">
											<a
												href="/dashboard/rentals/{row.contract.contract_id}"
												class="text-primary-400 hover:text-primary-300 transition-colors"
											>
												{row.contract.contract_id.slice(0, 12)}...
											</a>
											{#if row.contract.offering_id}
												<span class="text-neutral-600 ml-1">#{row.contract.offering_id}</span>
											{/if}
										</td>
										<td class="px-4 py-3">
											<span class="px-2 py-0.5 text-xs font-medium
												{row.contract.status === 'active' || row.contract.status === 'provisioned'
													? 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/30'
													: row.contract.status === 'cancelled' || row.contract.status === 'failed'
													? 'bg-red-500/20 text-red-400 border border-red-500/30'
													: 'bg-neutral-700/50 text-neutral-400 border border-neutral-600/30'}">
												{row.contract.status}
											</span>
										</td>
										<td class="px-4 py-3 text-right">
											{#if row.summary && row.summary.totalChecks > 0}
												<span class="px-2 py-0.5 text-xs font-medium {uptimeBadgeClass(row.summary.uptimePercent)}">
													{row.summary.uptimePercent.toFixed(1)}%
												</span>
											{:else if row.error}
												<span class="text-red-400 text-xs" title={row.error}>error</span>
											{:else}
												<span class="text-neutral-600 text-xs">no data</span>
											{/if}
										</td>
										<td class="px-4 py-3 text-right text-neutral-300">
											{row.summary ? row.summary.totalChecks : '—'}
										</td>
										<td class="px-4 py-3 text-right text-neutral-400">
											{row.summary?.avgLatencyMs != null
												? `${row.summary.avgLatencyMs.toFixed(0)} ms`
												: '—'}
										</td>
										<td class="px-4 py-3 text-right text-neutral-400 text-xs">
											{formatNsTimestamp(row.summary?.lastCheckedAt)}
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				</section>
			{/if}
		{/if}
	{/if}
</div>
