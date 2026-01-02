<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { page } from "$app/stores";
	import { navigateToLogin } from "$lib/utils/navigation";
	import {
		listAgentPools,
		createAgentPool,
		updateAgentPool,
		deleteAgentPool,
		hexEncode,
	} from "$lib/services/api";
	import type { AgentPoolWithStats } from "$lib/types/generated/AgentPoolWithStats";
	import { signRequest } from "$lib/services/auth-api";
	import { authStore } from "$lib/stores/auth";
	import { Ed25519KeyIdentity } from "@dfinity/identity";
	import AgentPoolTable from "$lib/components/provider/AgentPoolTable.svelte";

	let pools = $state<AgentPoolWithStats[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let actionMessage = $state<string | null>(null);
	let providerHex = $state("");
	let isAuthenticated = $state(false);
	let unsubscribeAuth: (() => void) | null = null;

	// Form state
	let showCreateForm = $state(false);
	let editingPool = $state<AgentPoolWithStats | null>(null);
	let formName = $state("");
	let formLocation = $state("europe");
	let formProvisionerType = $state("proxmox");
	let formSubmitting = $state(false);

	// Deleting state
	let deleting = $state<Record<string, boolean>>({});

	// Region identifiers matching api/src/database/agent_pools.rs REGIONS constant
	const LOCATIONS: { code: string; name: string }[] = [
		{ code: "europe", name: "Europe" },
		{ code: "na", name: "North America" },
		{ code: "latam", name: "Latin America" },
		{ code: "apac", name: "Asia Pacific" },
		{ code: "mena", name: "Middle East & North Africa" },
		{ code: "ssa", name: "Sub-Saharan Africa" },
		{ code: "cis", name: "CIS (Russia & neighbors)" },
	];
	const PROVISIONER_TYPES = ["proxmox", "script", "manual"];

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
				error = "You must be authenticated to manage agent pools";
				return;
			}
			if (!(info.identity instanceof Ed25519KeyIdentity)) {
				error = "Only Ed25519 identities can manage agent pools";
				return;
			}
			const normalizedIdentity: SigningIdentity = {
				identity: info.identity,
				publicKeyBytes: info.publicKeyBytes,
			};
			signingIdentityInfo = normalizedIdentity;
			providerHex = hexEncode(normalizedIdentity.publicKeyBytes);

			// Fetch pools
			const signed = await signRequest(
				normalizedIdentity.identity,
				"GET",
				`/api/v1/providers/${providerHex}/pools`,
			);
			pools = await listAgentPools(providerHex, signed.headers);
		} catch (e) {
			error =
				e instanceof Error
					? e.message
					: "Failed to load agent pools";
		} finally {
			loading = false;
		}
	}

	function handleLogin() {
		navigateToLogin($page.url.pathname);
	}

	function resetForm() {
		formName = "";
		formLocation = "europe";
		formProvisionerType = "proxmox";
		showCreateForm = false;
		editingPool = null;
	}

	function startEdit(pool: AgentPoolWithStats) {
		editingPool = pool;
		formName = pool.name;
		formLocation = pool.location;
		formProvisionerType = pool.provisionerType;
		showCreateForm = true;
	}

	async function handleSubmit() {
		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) {
			error = "Missing signing identity";
			return;
		}
		if (!formName.trim()) {
			error = "Pool name is required";
			return;
		}

		error = null;
		actionMessage = null;
		formSubmitting = true;

		try {
			if (editingPool) {
				// Update existing pool
				const poolData = {
					name: formName.trim(),
					location: formLocation,
					provisionerType: formProvisionerType,
				};
				const signed = await signRequest(
					activeIdentity.identity,
					"PUT",
					`/api/v1/providers/${providerHex}/pools/${editingPool.poolId}`,
					poolData,
				);
				await updateAgentPool(
					providerHex,
					editingPool.poolId,
					poolData,
					signed.headers,
				);
				actionMessage = `Pool "${formName}" updated`;
			} else {
				// Create new pool
				const poolData = {
					name: formName.trim(),
					location: formLocation,
					provisionerType: formProvisionerType,
				};
				const signed = await signRequest(
					activeIdentity.identity,
					"POST",
					`/api/v1/providers/${providerHex}/pools`,
					poolData,
				);
				await createAgentPool(
					providerHex,
					poolData,
					signed.headers,
				);
				actionMessage = `Pool "${formName}" created`;
			}
			resetForm();
			await loadData();
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to save pool";
		} finally {
			formSubmitting = false;
		}
	}

	onDestroy(() => {
		unsubscribeAuth?.();
	});
</script>

<div class="space-y-8">
	<header class="flex items-center justify-between">
		<div>
			<h1 class="text-2xl font-bold text-white tracking-tight">Agents</h1>
			<p class="text-neutral-500">
				Group agents into pools by location and provisioner type for load distribution.
			</p>
		</div>
		{#if isAuthenticated && !loading && !showCreateForm}
			<button
				onclick={() => { showCreateForm = true; }}
				class="px-4 py-2 bg-gradient-to-r from-primary-500 to-primary-600  font-semibold text-white hover:brightness-110 transition-all"
			>
				+ New Pool
			</button>
		{/if}
	</header>

	{#if !isAuthenticated}
		<div class="card p-8 border border-neutral-800 text-center">
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">🤖</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-neutral-400">
					Create an account or login to manage your agents and pools.
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
		{#if error}
			<div class="bg-red-500/20 border border-red-500/30  p-4 text-red-300">
				{error}
			</div>
		{/if}
		{#if actionMessage}
			<div class="bg-emerald-500/15 border border-emerald-500/30  p-4 text-emerald-300">
				{actionMessage}
			</div>
		{/if}

		{#if loading}
			<div class="flex justify-center items-center py-12">
				<div class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-primary-400"></div>
			</div>
		{:else}
			<!-- Create/Edit Form -->
			{#if showCreateForm}
				<div class="card p-6 border border-neutral-800">
					<h2 class="text-xl font-semibold text-white mb-4">
						{editingPool ? "Edit Pool" : "Create Agent Pool"}
					</h2>
					<form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }} class="space-y-4">
						<div>
							<label for="poolName" class="block text-sm text-neutral-400 mb-1">Pool Name</label>
							<input
								id="poolName"
								type="text"
								bind:value={formName}
								placeholder="e.g., eu-proxmox"
								class="w-full px-4 py-2 bg-surface-elevated border border-neutral-800  text-white placeholder-white/40 focus:outline-none focus:border-primary-400"
							/>
						</div>
						<div class="grid grid-cols-2 gap-4">
							<div>
								<label for="location" class="block text-sm text-neutral-400 mb-1">Location</label>
								<select
									id="location"
									bind:value={formLocation}
									class="w-full px-4 py-2 bg-surface-elevated border border-neutral-800  text-white focus:outline-none focus:border-primary-400"
								>
									{#each LOCATIONS as loc}
										<option value={loc.code}>{loc.name}</option>
									{/each}
								</select>
							</div>
							<div>
								<label for="provisionerType" class="block text-sm text-neutral-400 mb-1">Provisioner Type</label>
								<select
									id="provisionerType"
									bind:value={formProvisionerType}
									class="w-full px-4 py-2 bg-surface-elevated border border-neutral-800  text-white focus:outline-none focus:border-primary-400"
								>
									{#each PROVISIONER_TYPES as ptype}
										<option value={ptype}>{ptype}</option>
									{/each}
								</select>
							</div>
						</div>
						<div class="flex justify-end gap-3 pt-2">
							<button
								type="button"
								onclick={resetForm}
								class="px-4 py-2  text-neutral-400 hover:text-white hover:bg-surface-elevated transition-colors"
							>
								Cancel
							</button>
							<button
								type="submit"
								disabled={formSubmitting}
								class="px-6 py-2 bg-gradient-to-r from-primary-500 to-primary-600  font-semibold text-white hover:brightness-110 transition-all disabled:opacity-50"
							>
								{#if formSubmitting}
									Saving...
								{:else if editingPool}
									Update Pool
								{:else}
									Create Pool
								{/if}
							</button>
						</div>
					</form>
				</div>
			{/if}

			<!-- Pool List -->
			<AgentPoolTable {pools} onEdit={startEdit} />

		{/if}
	{/if}
</div>