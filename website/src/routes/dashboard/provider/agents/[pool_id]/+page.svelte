<script lang="ts">
	import { page } from "$app/stores";
	import { onMount, onDestroy } from "svelte";
	import {
		getAgentPoolDetails,
		listAgentsInPool,
		listSetupTokens,
		createSetupToken,
		deleteSetupToken,
		revokeAgentDelegation,
		updateAgentDelegationLabel,
		hexEncode,
		type CreateSetupTokenParams,
		type AgentDelegation,
	} from "$lib/services/api";
	import { signRequest } from "$lib/services/auth-api";
	import { authStore } from "$lib/stores/auth";
	import { Ed25519KeyIdentity } from "@dfinity/identity";
	import type { AgentPoolWithStats } from "$lib/types/generated/AgentPoolWithStats";
	import type { SetupToken } from "$lib/types/generated/SetupToken";
	import SetupTokenDialog from "$lib/components/provider/SetupTokenDialog.svelte";

	let pool = $state<AgentPoolWithStats | null>(null);
	let delegations = $state<AgentDelegation[]>([]);
	let tokens = $state<SetupToken[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	let showTokenDialog = $state(false);

	type SigningIdentity = {
		identity: Ed25519KeyIdentity;
		publicKeyBytes: Uint8Array;
	};

	let signingIdentityInfo = $state<SigningIdentity | null>(null);
	let providerHex = $state("");
	let refreshInterval: ReturnType<typeof setInterval> | null = null;

	// Get pool_id from route params, ensuring it's defined
	let poolId = $derived($page.params.pool_id ?? "");

	onMount(() => {
		const unsubscribe = authStore.isAuthenticated.subscribe((isAuth) => {
			if (isAuth && poolId) {
				loadData();
				// Auto-refresh agent status every 30 seconds
				refreshInterval = setInterval(() => {
					refreshAgentData();
				}, 30000);
			} else {
				loading = false;
			}
		});
		return unsubscribe;
	});

	onDestroy(() => {
		if (refreshInterval) {
			clearInterval(refreshInterval);
		}
	});

	async function loadData() {
		if (!poolId) {
			error = "Pool ID not specified";
			loading = false;
			return;
		}

		try {
			loading = true;
			error = null;
			const info = await authStore.getSigningIdentity();
			if (!info || !(info.identity instanceof Ed25519KeyIdentity)) {
				error = "Authentication failed or invalid identity type.";
				return;
			}
			signingIdentityInfo = {
				identity: info.identity,
				publicKeyBytes: info.publicKeyBytes,
			};
			providerHex = hexEncode(signingIdentityInfo.publicKeyBytes);

			// Each API call needs its own signature (path is part of signed message)
			const [signedPoolDetails, signedAgents, signedTokens] = await Promise.all([
				signRequest(signingIdentityInfo.identity, "GET", `/api/v1/providers/${providerHex}/pools/${poolId}`),
				signRequest(signingIdentityInfo.identity, "GET", `/api/v1/providers/${providerHex}/pools/${poolId}/agents`),
				signRequest(signingIdentityInfo.identity, "GET", `/api/v1/providers/${providerHex}/pools/${poolId}/setup-tokens`)
			]);

			[pool, delegations, tokens] = await Promise.all([
				getAgentPoolDetails(providerHex, poolId, signedPoolDetails.headers),
				listAgentsInPool(providerHex, poolId, signedAgents.headers),
				listSetupTokens(providerHex, poolId, signedTokens.headers)
			]);

		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load pool details.";
		} finally {
			loading = false;
		}
	}

	async function refreshAgentData() {
		// Silently refresh agent data without showing loading spinner
		if (!signingIdentityInfo || !poolId) return;

		try {
			const [signedPoolDetails, signedAgents] = await Promise.all([
				signRequest(signingIdentityInfo.identity, "GET", `/api/v1/providers/${providerHex}/pools/${poolId}`),
				signRequest(signingIdentityInfo.identity, "GET", `/api/v1/providers/${providerHex}/pools/${poolId}/agents`)
			]);

			const [updatedPool, updatedDelegations] = await Promise.all([
				getAgentPoolDetails(providerHex, poolId, signedPoolDetails.headers),
				listAgentsInPool(providerHex, poolId, signedAgents.headers)
			]);

			pool = updatedPool;
			delegations = updatedDelegations;
		} catch (e) {
			// Silently fail refresh - don't interrupt user experience
			console.error("Failed to refresh agent data:", e);
		}
	}

	async function handleCreateToken(label: string, expiresHours: number) {
		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) throw new Error("Not authenticated");

		const params: CreateSetupTokenParams = { label, expiresInHours: expiresHours };
		const signed = await signRequest(
			activeIdentity.identity,
			"POST",
			`/api/v1/providers/${providerHex}/pools/${poolId}/setup-tokens`,
			params
		);
		await createSetupToken(providerHex, poolId, params, signed.headers);
		// Need fresh signature for the list call (different path/method)
		const signedList = await signRequest(
			activeIdentity.identity,
			"GET",
			`/api/v1/providers/${providerHex}/pools/${poolId}/setup-tokens`
		);
		tokens = await listSetupTokens(providerHex, poolId, signedList.headers);
	}

	async function handleDeleteToken(token: string) {
		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) throw new Error("Not authenticated");

		const signed = await signRequest(
			activeIdentity.identity,
			"DELETE",
			`/api/v1/providers/${providerHex}/pools/${poolId}/setup-tokens/${token}`
		);
		await deleteSetupToken(providerHex, poolId, token, signed.headers);
		// Need fresh signature for the list call (different path/method)
		const signedList = await signRequest(
			activeIdentity.identity,
			"GET",
			`/api/v1/providers/${providerHex}/pools/${poolId}/setup-tokens`
		);
		tokens = await listSetupTokens(providerHex, poolId, signedList.headers);
	}

	async function handleRevokeAgent(agentPubkey: string) {
		if (!confirm("Are you sure you want to revoke this agent's access?")) return;

		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) throw new Error("Not authenticated");

		try {
			const signed = await signRequest(
				activeIdentity.identity,
				"DELETE",
				`/api/v1/providers/${providerHex}/agent-delegations/${agentPubkey}`
			);
			await revokeAgentDelegation(providerHex, agentPubkey, signed.headers);

			// Refresh agent data
			await refreshAgentData();
		} catch (e) {
			alert(e instanceof Error ? e.message : "Failed to revoke agent");
		}
	}

	async function handleUpdateLabel(agentPubkey: string) {
		const delegation = delegations.find(d => d.agentPubkey === agentPubkey);
		const currentLabel = delegation?.label || "";
		const newLabel = prompt("Enter new label for this agent:", currentLabel);

		if (newLabel === null || newLabel === currentLabel) return;

		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) throw new Error("Not authenticated");

		try {
			const signed = await signRequest(
				activeIdentity.identity,
				"PUT",
				`/api/v1/providers/${providerHex}/agent-delegations/${agentPubkey}/label`,
				{ label: newLabel }
			);
			await updateAgentDelegationLabel(providerHex, agentPubkey, newLabel, signed.headers);

			// Refresh agent data
			await refreshAgentData();
		} catch (e) {
			alert(e instanceof Error ? e.message : "Failed to update agent label");
		}
	}

	function formatPubkey(hex: string): string {
		if (hex.length <= 16) return hex;
		return hex.slice(0, 8) + "..." + hex.slice(-8);
	}

	function formatTimestamp(ns: number): string {
		const date = new Date(ns / 1_000_000);
		return date.toLocaleDateString();
	}
</script>

<div class="space-y-6">
	{#if loading}
		<div class="text-center py-16">
			<div class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-primary-400 mx-auto"></div>
		</div>
	{:else if error}
		<div class="bg-red-500/20 border border-red-500/30  p-6 text-red-300">
			<h2 class="font-bold mb-2">Error loading pool</h2>
			<p>{error}</p>
		</div>
	{:else if pool}
		<!-- Header -->
		<header>
			<a href="/dashboard/provider/agents" class="text-sm text-primary-400 hover:underline mb-2 block">&larr; Back to all pools</a>
			<div class="flex flex-wrap items-center justify-between gap-4">
				<div>
					<h1 class="text-3xl font-bold text-white">{pool.name}</h1>
					<div class="flex items-center gap-3 text-sm mt-2">
						<span class="px-2 py-0.5 rounded bg-primary-500/20 text-primary-300 border border-primary-500/30">{pool.location}</span>
						<span class="px-2 py-0.5 rounded bg-purple-500/20 text-primary-300 border border-purple-500/30">{pool.provisionerType}</span>
					</div>
				</div>
				<div>
					<button
						onclick={() => showTokenDialog = true}
						class="px-5 py-2.5 bg-gradient-to-r from-emerald-500 to-teal-600  font-semibold text-white hover:brightness-110 transition-all"
					>
						+ Add Agent
					</button>
				</div>
			</div>
		</header>

		<!-- Stats Cards -->
		<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
			<div class="bg-surface-elevated border border-neutral-800  p-5">
				<div class="text-sm text-neutral-500 mb-1">Agents</div>
				<div class="text-2xl font-semibold text-white">{pool.agentCount}</div>
			</div>
			<div class="bg-surface-elevated border border-neutral-800  p-5">
				<div class="text-sm text-neutral-500 mb-1">Online</div>
				<div class="text-2xl font-semibold text-green-400">{pool.onlineCount} / {pool.agentCount}</div>
			</div>
			<div class="bg-surface-elevated border border-neutral-800  p-5">
				<div class="text-sm text-neutral-500 mb-1">Active Contracts</div>
				<div class="text-2xl font-semibold text-white">{pool.activeContracts}</div>
			</div>
		</div>

		<!-- Agent Delegations Table -->
		<div class="bg-surface-elevated border border-neutral-800  overflow-hidden">
			<h3 class="px-6 py-4 text-lg font-medium text-white border-b border-neutral-800">
				Agent Delegations
			</h3>
			<table class="w-full text-sm text-left">
				<thead class="bg-surface-elevated text-xs text-neutral-500 uppercase">
					<tr>
						<th scope="col" class="px-6 py-3">Label</th>
						<th scope="col" class="px-6 py-3">Agent Pubkey</th>
						<th scope="col" class="px-6 py-3">Version</th>
						<th scope="col" class="px-6 py-3">Permissions</th>
						<th scope="col" class="px-6 py-3">Created</th>
						<th scope="col" class="px-6 py-3">Status</th>
						<th scope="col" class="px-6 py-3">Actions</th>
					</tr>
				</thead>
				<tbody>
					{#if delegations.length === 0}
						<tr>
							<td colspan="7" class="text-center py-8 text-neutral-500">
								No agents delegated to this pool yet.
							</td>
						</tr>
					{/if}
					{#each delegations as delegation (delegation.agentPubkey)}
						<tr class="border-b border-neutral-800 last:border-b-0 hover:bg-surface-elevated transition-colors">
							<th scope="row" class="px-6 py-4 font-medium text-white whitespace-nowrap">
								{delegation.label || "No label"}
							</th>
							<td class="px-6 py-4 font-mono text-xs text-neutral-400" title={delegation.agentPubkey}>
								{formatPubkey(delegation.agentPubkey)}
							</td>
							<td class="px-6 py-4 text-neutral-300">
								{delegation.version || "—"}
							</td>
							<td class="px-6 py-4">
								<div class="flex flex-wrap gap-1">
									{#each delegation.permissions as perm}
										<span class="px-1.5 py-0.5 text-xs bg-primary-500/20 text-primary-300 rounded">
											{perm}
										</span>
									{/each}
								</div>
							</td>
							<td class="px-6 py-4 text-neutral-300">
								{formatTimestamp(delegation.createdAtNs)}
							</td>
							<td class="px-6 py-4">
								{#if !delegation.active}
									<span class="flex items-center gap-2 text-red-400">
										<span class="h-2 w-2 rounded-full bg-red-400"></span>
										Revoked
									</span>
								{:else if delegation.online}
									<span class="flex items-center gap-2 text-green-400">
										<span class="h-2 w-2 rounded-full bg-green-400"></span>
										Online
									</span>
								{:else}
									<span class="flex items-center gap-2 text-amber-400">
										<span class="h-2 w-2 rounded-full bg-amber-400"></span>
										Offline
									</span>
								{/if}
							</td>
							<td class="px-6 py-4">
								<div class="flex gap-2">
									<button
										onclick={() => handleUpdateLabel(delegation.agentPubkey)}
										class="px-3 py-1 text-xs bg-primary-500/20 text-primary-300 border border-primary-500/30 rounded hover:bg-primary-500/30 transition-colors"
										title="Edit label"
									>
										Edit
									</button>
									{#if delegation.active}
										<button
											onclick={() => handleRevokeAgent(delegation.agentPubkey)}
											class="px-3 py-1 text-xs bg-red-500/20 text-red-300 border border-red-500/30 rounded hover:bg-red-500/30 transition-colors"
											title="Revoke agent access"
										>
											Revoke
										</button>
									{/if}
								</div>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		<!-- Add Agent Dialog -->
		<SetupTokenDialog
			bind:isOpen={showTokenDialog}
			{pool}
			{tokens}
			onCreate={handleCreateToken}
			onDelete={handleDeleteToken}
			onClose={() => {
				showTokenDialog = false;
				// Refresh agent data in case a token was used to add an agent
				refreshAgentData();
			}}
		/>
	{/if}
</div>
