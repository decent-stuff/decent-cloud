<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { page } from "$app/stores";
	import { navigateToLogin } from "$lib/utils/navigation";
	import PendingRequestCard from "$lib/components/provider/PendingRequestCard.svelte";
	import ManagedContractCard from "$lib/components/provider/ManagedContractCard.svelte";
	import {
		getPendingProviderRequests,
		getProviderContracts,
		getProviderOfferings,
		respondToRentalRequest,
		updateProvisioningStatus,
		getProviderBandwidthStats,
		getProviderOnboarding,
		type Contract,
		type Offering,
		type ProviderRentalResponseParams,
		type ProvisioningStatusUpdateParams,
		type BandwidthStatsResponse,
		hexEncode,
	} from "$lib/services/api";
	import { signRequest } from "$lib/services/auth-api";
	import ProviderSetupBanner from "$lib/components/ProviderSetupBanner.svelte";
	import {
		getAutoAcceptSetting,
		updateAutoAcceptSetting,
		getAutoAcceptRules,
		createAutoAcceptRule,
		deleteAutoAcceptRule,
		updateAutoAcceptRule,
		type AutoAcceptRule,
	} from "$lib/services/notification-api";
	import { authStore } from "$lib/stores/auth";
	import { Ed25519KeyIdentity } from "@dfinity/identity";

	let pendingRequests = $state<Contract[]>([]),
		managedContracts = $state<Contract[]>([]);
	let bandwidthStats = $state<BandwidthStatsResponse[]>([]);
	let loading = $state(true),
		error = $state<string | null>(null),
		actionMessage = $state<string | null>(null),
		providerHex = $state("");
	let memoInputs = $state<Record<string, string>>({}),
		provisioningNotes = $state<Record<string, string>>({});
	let responding = $state<Record<string, boolean>>({}),
		updating = $state<Record<string, boolean>>({});
	let batchProcessing = $state(false),
		batchProgress = $state<string | null>(null);
	let isAuthenticated = $state(false);
	let onboardingCompleted = $state<boolean | null>(null);
	let unsubscribeAuth: (() => void) | null = null;
	let autoAcceptEnabled = $state(false),
		autoAcceptUpdating = $state(false);
	let autoAcceptRules = $state<AutoAcceptRule[]>([]);
	let rulesLoading = $state(false);
	let newRuleOfferingId = $state('');
	let newRuleMinHours = $state<number | null>(null);
	let newRuleMaxHours = $state<number | null>(null);
	let rulesSaving = $state(false);
	let rulesError = $state<string | null>(null);
	let providerOfferings = $state<Offering[]>([]);
	let filterOfferingId = $state('');
	let filterMinDuration = $state<number | null>(null);
	let filterMaxDuration = $state<number | null>(null);

	let filteredPendingRequests = $derived(
		pendingRequests.filter((req) => {
			if (filterOfferingId && String(req.offering_id) !== filterOfferingId) return false;
			if (filterMinDuration !== null && (req.duration_hours ?? 0) < filterMinDuration) return false;
			if (filterMaxDuration !== null && (req.duration_hours ?? Infinity) > filterMaxDuration) return false;
			return true;
		}),
	);

	// Auto-refresh state
	let refreshInterval: ReturnType<typeof setInterval> | null = null;
	let autoRefreshEnabled = $state(true);
	let lastRefresh = $state<number>(Date.now());
	const REFRESH_INTERVAL_MS = 15_000; // 15 seconds

	function startAutoRefresh() {
		stopAutoRefresh();
		if (autoRefreshEnabled && isAuthenticated) {
			refreshInterval = setInterval(() => {
				refreshData();
			}, REFRESH_INTERVAL_MS);
		}
	}

	function stopAutoRefresh() {
		if (refreshInterval) {
			clearInterval(refreshInterval);
			refreshInterval = null;
		}
	}

	function toggleAutoRefresh() {
		autoRefreshEnabled = !autoRefreshEnabled;
		if (autoRefreshEnabled) {
			startAutoRefresh();
		} else {
			stopAutoRefresh();
		}
	}

	async function refreshData() {
		if (!isAuthenticated || loading) return;
		try {
			await loadData();
			lastRefresh = Date.now();
		} catch (e) {
			console.error("Error refreshing provider requests:", e);
		}
	}

	// Format bytes to human readable
	function formatBytes(bytes: number): string {
		if (bytes === 0) return "0 B";
		const k = 1024;
		const sizes = ["B", "KB", "MB", "GB", "TB"];
		const i = Math.floor(Math.log(bytes) / Math.log(k));
		return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
	}

	type SigningIdentity = {
		identity: Ed25519KeyIdentity;
		publicKeyBytes: Uint8Array;
	};

	let signingIdentityInfo = $state<SigningIdentity | null>(null);

	onMount(() => {
		unsubscribeAuth = authStore.isAuthenticated.subscribe((isAuth) => {
			isAuthenticated = isAuth;
			if (isAuth) {
				loadData();
				startAutoRefresh();
			} else {
				loading = false;
				stopAutoRefresh();
			}
		});
	});

	async function loadData() {
		if (!isAuthenticated) {
			loading = false;
			return;
		}

		try {
			loading = true;
			error = null;
			const info = await authStore.getSigningIdentity();
			if (!info) {
				error = "You must be authenticated to manage provider requests";
				return;
			}
			if (!(info.identity instanceof Ed25519KeyIdentity)) {
				error = "Only Ed25519 identities can sign provider actions";
				return;
			}
			const normalizedIdentity: SigningIdentity = {
				identity: info.identity,
				publicKeyBytes: info.publicKeyBytes,
			};
			signingIdentityInfo = normalizedIdentity;
			providerHex = hexEncode(normalizedIdentity.publicKeyBytes);
			getProviderOnboarding(providerHex).catch(() => null).then(o => { onboardingCompleted = !!o?.onboarding_completed_at; });
			getProviderOfferings(providerHex).then(o => { providerOfferings = o; }).catch(() => null);
			console.log(
				"[Provider Requests] Authenticated as provider:",
				providerHex,
			);
			const pendingSigned = await signRequest(
				normalizedIdentity.identity,
				"GET",
				"/api/v1/provider/rental-requests/pending",
			);
			pendingRequests = await getPendingProviderRequests(
				pendingSigned.headers,
			);
			console.log(
				"[Provider Requests] Found pending requests:",
				pendingRequests.length,
			);
			const contractsSigned = await signRequest(
				normalizedIdentity.identity,
				"GET",
				`/api/v1/providers/${providerHex}/contracts`,
			);
			const contracts = await getProviderContracts(
				contractsSigned.headers,
				providerHex,
			);
			managedContracts = (contracts || []).filter((contract) =>
				["accepted", "provisioning", "provisioned", "active"].includes(
					contract.status.toLowerCase(),
				),
			);
			// Load bandwidth stats
			const bandwidthSigned = await signRequest(
				normalizedIdentity.identity,
				"GET",
				`/api/v1/providers/${providerHex}/bandwidth`,
			);
			bandwidthStats = await getProviderBandwidthStats(providerHex, bandwidthSigned.headers);
			// Load auto-accept setting and rules
			autoAcceptEnabled = await getAutoAcceptSetting(normalizedIdentity.identity);
			autoAcceptRules = await getAutoAcceptRules(normalizedIdentity.identity).catch(() => []);
			lastRefresh = Date.now();
		} catch (e) {
			error =
				e instanceof Error
					? e.message
					: "Failed to load provider rental requests";
		} finally {
			loading = false;
		}
	}

	function handleLogin() {
		navigateToLogin($page.url.pathname);
	}

	const memoValue = (contractId: string) => memoInputs[contractId] ?? "";
	const provisioningValue = (contractId: string) =>
		provisioningNotes[contractId] ?? "";

	async function handleAutoAcceptToggle() {
		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) {
			error = "Missing signing identity";
			return;
		}
		error = null;
		actionMessage = null;
		autoAcceptUpdating = true;
		try {
			const newValue = !autoAcceptEnabled;
			await updateAutoAcceptSetting(activeIdentity.identity, newValue);
			autoAcceptEnabled = newValue;
			actionMessage = newValue
				? "Auto-accept enabled - new rentals will be accepted automatically"
				: "Auto-accept disabled - new rentals will require manual approval";
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to update auto-accept setting";
		} finally {
			autoAcceptUpdating = false;
		}
	}

	async function handleCreateRule() {
		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) { rulesError = "Missing signing identity"; return; }
		if (!newRuleOfferingId) { rulesError = "Select an offering"; return; }
		if (newRuleMinHours !== null && newRuleMaxHours !== null && newRuleMinHours > newRuleMaxHours) {
			rulesError = "Min duration must not exceed max duration";
			return;
		}
		rulesError = null;
		rulesSaving = true;
		try {
			await createAutoAcceptRule(activeIdentity.identity, {
				offeringId: newRuleOfferingId,
				minDurationHours: newRuleMinHours,
				maxDurationHours: newRuleMaxHours,
			});
			newRuleOfferingId = '';
			newRuleMinHours = null;
			newRuleMaxHours = null;
			autoAcceptRules = await getAutoAcceptRules(activeIdentity.identity);
		} catch (e) {
			rulesError = e instanceof Error ? e.message : "Failed to create rule";
		} finally {
			rulesSaving = false;
		}
	}

	async function handleToggleRule(rule: AutoAcceptRule) {
		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) { rulesError = "Missing signing identity"; return; }
		rulesError = null;
		try {
			await updateAutoAcceptRule(activeIdentity.identity, rule.id, {
				minDurationHours: rule.minDurationHours,
				maxDurationHours: rule.maxDurationHours,
				enabled: !rule.enabled,
			});
			autoAcceptRules = await getAutoAcceptRules(activeIdentity.identity);
		} catch (e) {
			rulesError = e instanceof Error ? e.message : "Failed to update rule";
		}
	}

	async function handleDeleteRule(ruleId: number) {
		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) { rulesError = "Missing signing identity"; return; }
		rulesError = null;
		try {
			await deleteAutoAcceptRule(activeIdentity.identity, ruleId);
			autoAcceptRules = await getAutoAcceptRules(activeIdentity.identity);
		} catch (e) {
			rulesError = e instanceof Error ? e.message : "Failed to delete rule";
		}
	}

	async function handleResponse(contract: Contract, accept: boolean) {
		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) {
			error = "Missing signing identity";
			return;
		}
		error = null;
		actionMessage = null;
		responding = { ...responding, [contract.contract_id]: true };
		try {
			const memo = memoValue(contract.contract_id).trim();
			const payload: ProviderRentalResponseParams = {
				accept,
				memo: memo || undefined,
			};
			const path = `/api/v1/provider/rental-requests/${contract.contract_id}/respond`;
			const signed = await signRequest(
				activeIdentity.identity,
				"POST",
				path,
				payload,
			);
			await respondToRentalRequest(
				contract.contract_id,
				payload,
				signed.headers,
			);
			actionMessage = accept ? "Request accepted" : "Request rejected";
			await loadData();
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to update request";
		} finally {
			responding = { ...responding, [contract.contract_id]: false };
		}
	}

	async function handleBatchAction(accept: boolean) {
		const action = accept ? "Accept" : "Reject";
		if (!window.confirm(`${action} all ${filteredPendingRequests.length} pending requests?`)) return;
		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) {
			error = "Missing signing identity";
			return;
		}
		error = null;
		actionMessage = null;
		batchProcessing = true;
		const snapshot = [...filteredPendingRequests];
		const errors: string[] = [];
		for (let i = 0; i < snapshot.length; i++) {
			const contract = snapshot[i];
			batchProgress = `${action}ing ${i + 1}/${snapshot.length}...`;
			try {
				const payload: ProviderRentalResponseParams = { accept };
				const path = `/api/v1/provider/rental-requests/${contract.contract_id}/respond`;
				const signed = await signRequest(activeIdentity.identity, "POST", path, payload);
				await respondToRentalRequest(contract.contract_id, payload, signed.headers);
			} catch (e) {
				errors.push(e instanceof Error ? e.message : `Failed on ${contract.contract_id}`);
			}
		}
		batchProcessing = false;
		batchProgress = null;
		if (errors.length > 0) {
			error = `${errors.length} request(s) failed: ${errors.join("; ")}`;
		} else {
			actionMessage = `${action}ed all ${snapshot.length} requests`;
		}
		await loadData();
	}

	async function handleStatusUpdate(contract: Contract, nextStatus: string) {
		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) {
			error = "Missing signing identity";
			return;
		}
		const details = provisioningValue(contract.contract_id).trim();
		if (nextStatus === "provisioned" && !details) {
			error =
				"Instance details are required to mark a contract as provisioned";
			return;
		}
		error = null;
		actionMessage = null;
		updating = { ...updating, [contract.contract_id]: true };
		try {
			const payload: ProvisioningStatusUpdateParams = {
				status: nextStatus,
				instanceDetails:
					nextStatus === "provisioned" ? details : undefined,
			};
			const signed = await signRequest(
				activeIdentity.identity,
				"PUT",
				`/api/v1/provider/rental-requests/${contract.contract_id}/provisioning`,
				payload,
			);
			await updateProvisioningStatus(
				contract.contract_id,
				payload,
				signed.headers,
			);
			actionMessage = `Updated contract status to ${nextStatus}`;
			await loadData();
		} catch (e) {
			error =
				e instanceof Error
					? e.message
					: "Failed to update provisioning status";
		} finally {
			updating = { ...updating, [contract.contract_id]: false };
		}
	}

	onDestroy(() => {
		unsubscribeAuth?.();
		stopAutoRefresh();
	});
</script>

<div class="space-y-8">
	<ProviderSetupBanner completed={onboardingCompleted} />

	<div class="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-4">
		<div>
			<h1 class="text-2xl font-bold text-white tracking-tight mb-2">Provider Requests</h1>
			<p class="text-neutral-500">
				Review new rental submissions and keep provisioning progress up to date
			</p>
		</div>
		{#if isAuthenticated}
			<div class="flex items-center gap-3">
				<button
					onclick={toggleAutoRefresh}
					class="flex items-center gap-2 px-3 py-1.5 text-sm transition-colors {autoRefreshEnabled ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/30' : 'bg-surface-elevated text-neutral-500 border border-neutral-800'}"
					title={autoRefreshEnabled ? 'Auto-refresh enabled (15s)' : 'Auto-refresh disabled'}
				>
					<span class="relative flex h-2 w-2">
						{#if autoRefreshEnabled}
							<span class="animate-ping absolute inline-flex h-full w-full bg-emerald-400 opacity-75"></span>
						{/if}
						<span class="relative inline-flex h-2 w-2 {autoRefreshEnabled ? 'bg-emerald-400' : 'bg-white/30'}"></span>
					</span>
					Auto-refresh
				</button>
				<button
					onclick={refreshData}
					class="px-3 py-1.5 text-sm bg-surface-elevated text-neutral-400 border border-neutral-800 hover:bg-surface-elevated transition-colors"
					title="Refresh now"
				>
					↻ Refresh
				</button>
			</div>
		{/if}
	</div>

	{#if !isAuthenticated}
		<!-- Anonymous user view - login prompt -->
		<div class="card p-8 border border-neutral-800 text-center">
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">🤝</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-neutral-400">
					Create an account or login to manage provider rental requests, respond to pending requests, and update provisioning status.
				</p>
				<button
					onclick={handleLogin}
					class="px-8 py-3 bg-gradient-to-r from-primary-500 to-primary-600  font-semibold text-white hover:brightness-110 hover:scale-105 transition-all"
				>
					Login / Create Account
				</button>
			</div>
		</div>
	{:else}
		{#if error}<div
			class="bg-red-500/20 border border-red-500/30  p-4 text-red-300"
		>
			{error}
		</div>{/if}
	{#if actionMessage}<div
			class="bg-emerald-500/15 border border-emerald-500/30  p-4 text-emerald-300"
		>
			{actionMessage}
		</div>{/if}

	{#if loading}
		<div class="flex justify-center items-center py-12">
			<div
				class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-primary-400"
			></div>
		</div>
	{:else}
		<!-- Auto-Accept Settings Card -->
		<section class="bg-surface-elevated border border-neutral-800  p-6">
			<div class="flex items-center justify-between">
				<div>
					<h3 class="text-lg font-semibold text-white">Auto-Accept Rentals</h3>
					<p class="text-neutral-500 text-sm mt-1">
						When enabled, new rental requests are automatically accepted after payment.
						This enables instant provisioning for your customers.
					</p>
				</div>
				<button
					onclick={handleAutoAcceptToggle}
					disabled={autoAcceptUpdating}
					aria-label={autoAcceptEnabled ? 'Disable auto-accept rentals' : 'Enable auto-accept rentals'}
					class="relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2 focus:ring-offset-gray-900 disabled:opacity-50 disabled:cursor-not-allowed {autoAcceptEnabled ? 'bg-emerald-500' : 'bg-surface-elevated'}"
				>
					<span
						class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {autoAcceptEnabled ? 'translate-x-5' : 'translate-x-0'}"
					></span>
				</button>
			</div>
			{#if autoAcceptEnabled}
				<div class="mt-3 flex items-center gap-2 text-emerald-400 text-sm">
					<svg class="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
						<path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd" />
					</svg>
					<span>New rentals will be accepted automatically after payment</span>
				</div>
			{/if}
		</section>

		{#if autoAcceptEnabled}
		<!-- Auto-Accept Rules Panel -->
		<section class="bg-surface-elevated border border-neutral-800  p-6 space-y-4">
			<div>
				<h3 class="text-base font-semibold text-white">Auto-Accept Rules</h3>
				<p class="text-neutral-500 text-sm mt-1">
					Optionally restrict auto-accept to specific offerings and duration ranges.
					If no rule exists for an offering, <strong class="text-neutral-300">all</strong> requests for that offering are auto-accepted.
				</p>
			</div>

			{#if rulesError}
				<div class="p-3 bg-red-500/15 border border-red-500/30 text-red-300 text-sm">{rulesError}</div>
			{/if}

			<!-- Existing rules list -->
			{#if rulesLoading}
				<div class="text-neutral-500 text-sm">Loading rules...</div>
			{:else if autoAcceptRules.length > 0}
				<div class="space-y-2">
					{#each autoAcceptRules as rule (rule.id)}
						{@const offering = providerOfferings.find(o => String(o.offering_id) === rule.offeringId)}
						<div class="flex items-center justify-between gap-3 p-3 border border-neutral-700  text-sm {rule.enabled ? 'bg-neutral-900' : 'bg-neutral-900/40 opacity-60'}">
							<div class="flex flex-col gap-0.5">
								<span class="text-neutral-200 font-medium">{offering?.offer_name ?? rule.offeringId}</span>
								<span class="text-neutral-500 text-xs">
									{#if rule.minDurationHours !== null && rule.maxDurationHours !== null}
										{rule.minDurationHours}h – {rule.maxDurationHours}h
									{:else if rule.minDurationHours !== null}
										≥ {rule.minDurationHours}h
									{:else if rule.maxDurationHours !== null}
										≤ {rule.maxDurationHours}h
									{:else}
										Any duration
									{/if}
								</span>
							</div>
							<div class="flex items-center gap-2">
								<button
									onclick={() => handleToggleRule(rule)}
									class="text-xs px-2.5 py-1 border transition-colors {rule.enabled ? 'border-emerald-500/40 text-emerald-400 hover:bg-emerald-500/10' : 'border-neutral-700 text-neutral-500 hover:bg-neutral-800'}"
									title={rule.enabled ? 'Disable rule' : 'Enable rule'}
								>
									{rule.enabled ? 'Enabled' : 'Disabled'}
								</button>
								<button
									onclick={() => handleDeleteRule(rule.id)}
									class="text-xs px-2.5 py-1 border border-red-500/30 text-red-400 hover:bg-red-500/10 transition-colors"
									title="Delete rule"
								>
									Delete
								</button>
							</div>
						</div>
					{/each}
				</div>
			{:else}
				<p class="text-neutral-500 text-sm">No rules configured — all requests are auto-accepted after payment.</p>
			{/if}

			<!-- Add new rule form -->
			<div class="pt-2 border-t border-neutral-800">
				<p class="text-sm font-medium text-neutral-300 mb-3">Add rule</p>
				<div class="flex flex-wrap items-end gap-3">
					<div>
						<label for="rule-offering" class="block text-xs text-neutral-500 mb-1">Offering</label>
						<select
							id="rule-offering"
							bind:value={newRuleOfferingId}
							class="px-3 py-1.5 bg-neutral-900 border border-neutral-700  text-sm text-neutral-300 focus:outline-none focus:border-primary-500"
						>
							<option value="">Select offering</option>
							{#each providerOfferings.filter(o => !autoAcceptRules.some(r => r.offeringId === String(o.offering_id))) as offering}
								<option value={String(offering.offering_id)}>{offering.offer_name}</option>
							{/each}
						</select>
					</div>
					<div>
						<label for="rule-min-hours" class="block text-xs text-neutral-500 mb-1">Min hours (optional)</label>
						<input
							id="rule-min-hours"
							type="number"
							min="0"
							placeholder="e.g. 24"
							bind:value={newRuleMinHours}
							class="w-32 px-3 py-1.5 bg-neutral-900 border border-neutral-700  text-sm text-neutral-300 focus:outline-none focus:border-primary-500"
						/>
					</div>
					<div>
						<label for="rule-max-hours" class="block text-xs text-neutral-500 mb-1">Max hours (optional)</label>
						<input
							id="rule-max-hours"
							type="number"
							min="0"
							placeholder="e.g. 720"
							bind:value={newRuleMaxHours}
							class="w-32 px-3 py-1.5 bg-neutral-900 border border-neutral-700  text-sm text-neutral-300 focus:outline-none focus:border-primary-500"
						/>
					</div>
					<button
						onclick={handleCreateRule}
						disabled={rulesSaving || !newRuleOfferingId}
						class="px-4 py-1.5 text-sm bg-primary-500 text-white hover:bg-primary-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
					>
						{rulesSaving ? 'Adding...' : 'Add Rule'}
					</button>
				</div>
			</div>
		</section>
		{/if}

		<section class="space-y-4">
			<div class="flex items-center justify-between">
				<h2 class="text-2xl font-semibold text-white">
					Pending Requests
				</h2>
				<div class="flex items-center gap-3">
					{#if batchProgress}
						<span class="text-sm text-neutral-400">{batchProgress}</span>
					{/if}
					{#if filteredPendingRequests.length > 1}
						<button
							onclick={() => handleBatchAction(true)}
							disabled={batchProcessing || Object.values(responding).some((v) => v)}
							class="text-sm px-3 py-1.5 border border-emerald-500/40 text-emerald-400 hover:bg-emerald-500/10 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
						>
							Accept All
						</button>
						<button
							onclick={() => handleBatchAction(false)}
							disabled={batchProcessing || Object.values(responding).some((v) => v)}
							class="text-sm px-3 py-1.5 border border-red-500/40 text-red-400 hover:bg-red-500/10 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
						>
							Reject All
						</button>
					{:else}
						<span class="text-neutral-500 text-sm">{pendingRequests.length} awaiting action</span>
					{/if}
				</div>
			</div>

			<!-- Filter bar -->
			{#if pendingRequests.length > 0}
			<div class="flex flex-wrap items-center gap-3 p-3 bg-surface-elevated border border-neutral-800 ">
				<select
					bind:value={filterOfferingId}
					class="px-3 py-1.5 bg-neutral-900 border border-neutral-700  text-sm text-neutral-300 focus:outline-none focus:border-primary-500"
				>
					<option value="">All offerings</option>
					{#each providerOfferings as offering}
						<option value={String(offering.offering_id)}>{offering.offer_name}</option>
					{/each}
				</select>
				<div class="flex items-center gap-2">
					<input
						type="number"
						min="0"
						placeholder="Min hours"
						bind:value={filterMinDuration}
						class="w-28 px-3 py-1.5 bg-neutral-900 border border-neutral-700  text-sm text-neutral-300 focus:outline-none focus:border-primary-500"
					/>
					<span class="text-neutral-600 text-sm">–</span>
					<input
						type="number"
						min="0"
						placeholder="Max hours"
						bind:value={filterMaxDuration}
						class="w-28 px-3 py-1.5 bg-neutral-900 border border-neutral-700  text-sm text-neutral-300 focus:outline-none focus:border-primary-500"
					/>
				</div>
				<span class="text-neutral-500 text-sm ml-auto">
					Showing {filteredPendingRequests.length} of {pendingRequests.length}
				</span>
				{#if filterOfferingId || filterMinDuration !== null || filterMaxDuration !== null}
					<button
						onclick={() => { filterOfferingId = ''; filterMinDuration = null; filterMaxDuration = null; }}
						class="px-2.5 py-1 text-xs text-neutral-400 border border-neutral-700  hover:bg-neutral-800 transition-colors"
					>
						Clear filters
					</button>
				{/if}
			</div>
			{/if}

			{#if pendingRequests.length === 0}
				<div
					class="bg-surface-elevated border border-neutral-800  p-6 text-neutral-400"
				>
					{#if autoAcceptEnabled}
						No pending requests - auto-accept is handling new rentals automatically.
					{:else}
						No pending rental requests right now.
					{/if}
				</div>
			{:else if filteredPendingRequests.length === 0}
				<div class="bg-surface-elevated border border-neutral-800  p-6 text-neutral-400">
					No requests match the current filters.
				</div>
			{:else}
				<div class="space-y-4">
					{#each filteredPendingRequests as contract}
						<PendingRequestCard
							{contract}
							memo={memoValue(contract.contract_id)}
							busy={responding[contract.contract_id]}
							onMemoChange={(value) =>
								(memoInputs = {
									...memoInputs,
									[contract.contract_id]: value,
								})}
							onRespond={(accept) =>
								handleResponse(contract, accept)}
						/>
					{/each}
				</div>
			{/if}
		</section>

		<section class="space-y-4">
			<div class="flex items-center justify-between mt-10">
				<h2 class="text-2xl font-semibold text-white">
					Active Contracts
				</h2>
				<span class="text-neutral-500 text-sm"
					>{managedContracts.length} in progress</span
				>
			</div>

			{#if managedContracts.length === 0}
				<div
					class="bg-surface-elevated border border-neutral-800  p-6 text-neutral-400"
				>
					No contracts in provisioning stages.
				</div>
			{:else}
				<div class="space-y-4">
					{#each managedContracts as contract}
						<ManagedContractCard
							{contract}
							note={provisioningValue(contract.contract_id)}
							busy={updating[contract.contract_id]}
							onNoteChange={(value) =>
								(provisioningNotes = {
									...provisioningNotes,
									[contract.contract_id]: value,
								})}
							onUpdateStatus={(status) =>
								handleStatusUpdate(contract, status)}
						/>
					{/each}
				</div>
			{/if}
		</section>

		<!-- Bandwidth Stats Section -->
		{#if bandwidthStats.length > 0}
			<section class="space-y-4">
				<div class="flex items-center justify-between mt-10">
					<h2 class="text-2xl font-semibold text-white">
						Bandwidth Usage
					</h2>
					<span class="text-neutral-500 text-sm"
						>{bandwidthStats.length} contracts with gateway traffic</span
					>
				</div>

				<div class="bg-surface-elevated border border-neutral-800  overflow-hidden">
					<table class="w-full text-sm">
						<thead>
							<tr class="border-b border-neutral-800 text-left">
								<th class="px-4 py-3 text-neutral-500 font-medium">Contract</th>
								<th class="px-4 py-3 text-neutral-500 font-medium">Gateway</th>
								<th class="px-4 py-3 text-neutral-500 font-medium text-right">Inbound</th>
								<th class="px-4 py-3 text-neutral-500 font-medium text-right">Outbound</th>
								<th class="px-4 py-3 text-neutral-500 font-medium text-right">Total</th>
							</tr>
						</thead>
						<tbody>
							{#each bandwidthStats as stat}
								<tr class="border-b border-white/5 hover:bg-surface-elevated transition-colors">
									<td class="px-4 py-3 font-mono text-neutral-300">
										{stat.contractId.substring(0, 12)}...
									</td>
									<td class="px-4 py-3 text-neutral-400">
										{stat.gatewaySlug}
									</td>
									<td class="px-4 py-3 text-right text-emerald-400">
										↓ {formatBytes(stat.bytesIn)}
									</td>
									<td class="px-4 py-3 text-right text-primary-400">
										↑ {formatBytes(stat.bytesOut)}
									</td>
									<td class="px-4 py-3 text-right text-white font-medium">
										{formatBytes(stat.bytesIn + stat.bytesOut)}
									</td>
								</tr>
							{/each}
						</tbody>
						<tfoot>
							<tr class="bg-surface-elevated">
								<td colspan="2" class="px-4 py-3 text-neutral-500 font-medium">Total</td>
								<td class="px-4 py-3 text-right text-emerald-400 font-medium">
									↓ {formatBytes(bandwidthStats.reduce((sum, s) => sum + s.bytesIn, 0))}
								</td>
								<td class="px-4 py-3 text-right text-primary-400 font-medium">
									↑ {formatBytes(bandwidthStats.reduce((sum, s) => sum + s.bytesOut, 0))}
								</td>
								<td class="px-4 py-3 text-right text-white font-bold">
									{formatBytes(bandwidthStats.reduce((sum, s) => sum + s.bytesIn + s.bytesOut, 0))}
								</td>
							</tr>
						</tfoot>
					</table>
				</div>
			</section>
		{/if}
	{/if}
	{/if}
</div>
