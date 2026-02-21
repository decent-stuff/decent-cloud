<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { page } from '$app/stores';
	import { navigateToLogin } from '$lib/utils/navigation';
	import {
		getProviderContracts,
		getProviderContractHealthSummary,
		getProviderSlaUptimeConfig,
		updateProviderSlaUptimeConfig,
		hexEncode,
		type Contract,
		type ContractHealthSummary,
		type SlaUptimeConfig
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

			// Load SLA config and contracts in parallel
			const [signedContracts] = await Promise.all([
				signRequest(info.identity, 'GET', `/api/v1/providers/${providerHex}/contracts`),
				loadSlaConfig(info.identity, providerHex)
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
			console.debug('SLA config not found, using defaults:', e);
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

	function handleLogin() {
		navigateToLogin($page.url.pathname);
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
		<div class="card p-8 border border-neutral-800 text-center">
			<div class="max-w-md mx-auto space-y-6">
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-neutral-400">
					Login to view SLA monitoring data for your contracts.
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
