<script lang="ts">
	import { onMount } from "svelte";
	import PendingRequestCard from "$lib/components/provider/PendingRequestCard.svelte";
	import ManagedContractCard from "$lib/components/provider/ManagedContractCard.svelte";
	import {
		getPendingProviderRequests,
		getProviderContracts,
		respondToRentalRequest,
		updateProvisioningStatus,
		type Contract,
		type ProviderRentalResponseParams,
		type ProvisioningStatusUpdateParams,
		hexEncode,
	} from "$lib/services/api";
	import { signRequest } from "$lib/services/auth-api";
	import { authStore } from "$lib/stores/auth";
	import { Ed25519KeyIdentity } from "@dfinity/identity";

	let pendingRequests = $state<Contract[]>([]),
		managedContracts = $state<Contract[]>([]);
	let loading = $state(true),
		error = $state<string | null>(null),
		actionMessage = $state<string | null>(null),
		providerHex = $state("");
	let memoInputs = $state<Record<string, string>>({}),
		provisioningNotes = $state<Record<string, string>>({});
	let responding = $state<Record<string, boolean>>({}),
		updating = $state<Record<string, boolean>>({});

	type SigningIdentity = {
		identity: Ed25519KeyIdentity;
		publicKeyBytes: Uint8Array;
	};

	let signingIdentityInfo = $state<SigningIdentity | null>(null);

	onMount(loadData);

	async function loadData() {
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
		} catch (e) {
			error =
				e instanceof Error
					? e.message
					: "Failed to load provider rental requests";
		} finally {
			loading = false;
		}
	}

	const memoValue = (contractId: string) => memoInputs[contractId] ?? "";
	const provisioningValue = (contractId: string) =>
		provisioningNotes[contractId] ?? "";

	async function handleResponse(contract: Contract, accept: boolean) {
		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) {
			error = "Missing signing identity";
			return;
		}
		error = null;
		actionMessage = null;
		responding = { ...responding, [contract.contractId]: true };
		try {
			const memo = memoValue(contract.contractId).trim();
			const payload: ProviderRentalResponseParams = {
				accept,
				memo: memo || undefined,
			};
			const path = `/api/v1/provider/rental-requests/${contract.contractId}/respond`;
			const signed = await signRequest(
				activeIdentity.identity,
				"POST",
				path,
				payload,
			);
			await respondToRentalRequest(
				contract.contractId,
				payload,
				signed.headers,
			);
			actionMessage = accept ? "Request accepted" : "Request rejected";
			await loadData();
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to update request";
		} finally {
			responding = { ...responding, [contract.contractId]: false };
		}
	}

	async function handleStatusUpdate(contract: Contract, nextStatus: string) {
		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) {
			error = "Missing signing identity";
			return;
		}
		const details = provisioningValue(contract.contractId).trim();
		if (nextStatus === "provisioned" && !details) {
			error =
				"Instance details are required to mark a contract as provisioned";
			return;
		}
		error = null;
		actionMessage = null;
		updating = { ...updating, [contract.contractId]: true };
		try {
			const payload: ProvisioningStatusUpdateParams = {
				status: nextStatus,
				instanceDetails:
					nextStatus === "provisioned" ? details : undefined,
			};
			const signed = await signRequest(
				activeIdentity.identity,
				"PUT",
				`/api/v1/provider/rental-requests/${contract.contractId}/provisioning`,
				payload,
			);
			await updateProvisioningStatus(
				contract.contractId,
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
			updating = { ...updating, [contract.contractId]: false };
		}
	}
</script>

<div class="space-y-8">
	<header>
		<h1 class="text-4xl font-bold text-white mb-2">Provider Requests</h1>
		<p class="text-white/60">
			Review new rental submissions and keep provisioning progress up to
			date
		</p>
	</header>

	{#if error}<div
			class="bg-red-500/20 border border-red-500/30 rounded-lg p-4 text-red-300"
		>
			{error}
		</div>{/if}
	{#if actionMessage}<div
			class="bg-emerald-500/15 border border-emerald-500/30 rounded-lg p-4 text-emerald-300"
		>
			{actionMessage}
		</div>{/if}

	{#if loading}
		<div class="flex justify-center items-center py-12">
			<div
				class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-400"
			></div>
		</div>
	{:else}
		<section class="space-y-4">
			<div class="flex items-center justify-between">
				<h2 class="text-2xl font-semibold text-white">
					Pending Requests
				</h2>
				<span class="text-white/60 text-sm"
					>{pendingRequests.length} awaiting action</span
				>
			</div>

			{#if pendingRequests.length === 0}
				<div
					class="bg-white/5 border border-white/10 rounded-xl p-6 text-white/70"
				>
					No pending rental requests right now.
				</div>
			{:else}
				<div class="space-y-4">
					{#each pendingRequests as contract}
						<PendingRequestCard
							{contract}
							memo={memoValue(contract.contractId)}
							busy={responding[contract.contractId]}
							onMemoChange={(value) =>
								(memoInputs = {
									...memoInputs,
									[contract.contractId]: value,
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
				<span class="text-white/60 text-sm"
					>{managedContracts.length} in progress</span
				>
			</div>

			{#if managedContracts.length === 0}
				<div
					class="bg-white/5 border border-white/10 rounded-xl p-6 text-white/70"
				>
					No contracts in provisioning stages.
				</div>
			{:else}
				<div class="space-y-4">
					{#each managedContracts as contract}
						<ManagedContractCard
							{contract}
							note={provisioningValue(contract.contractId)}
							busy={updating[contract.contractId]}
							onNoteChange={(value) =>
								(provisioningNotes = {
									...provisioningNotes,
									[contract.contractId]: value,
								})}
							onUpdateStatus={(status) =>
								handleStatusUpdate(contract, status)}
						/>
					{/each}
				</div>
			{/if}
		</section>
	{/if}
</div>
