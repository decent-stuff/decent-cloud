<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { page } from "$app/stores";
	import { navigateToLogin } from "$lib/utils/navigation";
	import PendingRequestCard from "$lib/components/provider/PendingRequestCard.svelte";
	import ManagedContractCard from "$lib/components/provider/ManagedContractCard.svelte";
	import {
		getPendingProviderRequests,
		getProviderContracts,
		respondToRentalRequest,
		updateProvisioningStatus,
		getProviderBandwidthStats,
		type Contract,
		type ProviderRentalResponseParams,
		type ProvisioningStatusUpdateParams,
		type BandwidthStatsResponse,
		hexEncode,
	} from "$lib/services/api";
	import { signRequest } from "$lib/services/auth-api";
	import { getAutoAcceptSetting, updateAutoAcceptSetting } from "$lib/services/notification-api";
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
	let isAuthenticated = $state(false);
	let unsubscribeAuth: (() => void) | null = null;
	let autoAcceptEnabled = $state(false),
		autoAcceptUpdating = $state(false);

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
			} else {
				loading = false;
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
			// Load auto-accept setting
			autoAcceptEnabled = await getAutoAcceptSetting(normalizedIdentity.identity);
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
	});
</script>

<div class="space-y-8">
	<header>
		<h1 class="text-2xl font-bold text-white tracking-tight">Provider Requests</h1>
		<p class="text-neutral-500">
			Review new rental submissions and keep provisioning progress up to
			date
		</p>
	</header>

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

		<section class="space-y-4">
			<div class="flex items-center justify-between">
				<h2 class="text-2xl font-semibold text-white">
					Pending Requests
				</h2>
				<span class="text-neutral-500 text-sm"
					>{pendingRequests.length} awaiting action</span
				>
			</div>

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
			{:else}
				<div class="space-y-4">
					{#each pendingRequests as contract}
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
