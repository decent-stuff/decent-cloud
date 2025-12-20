<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { page } from "$app/stores";
	import { navigateToLogin } from "$lib/utils/navigation";
	import {
		listAgentPools,
		createAgentPool,
		updateAgentPool,
		deleteAgentPool,
		listSetupTokens,
		createSetupToken,
		deleteSetupToken,
		hexEncode,
		type CreatePoolParams,
		type CreateSetupTokenParams,
	} from "$lib/services/api";
	import type { AgentPoolWithStats } from "$lib/types/generated/AgentPoolWithStats";
	import type { SetupToken } from "$lib/types/generated/SetupToken";
	import { signRequest } from "$lib/services/auth-api";
	import { authStore } from "$lib/stores/auth";
	import { Ed25519KeyIdentity } from "@dfinity/identity";

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
	let formLocation = $state("eu");
	let formProvisionerType = $state("proxmox");
	let formSubmitting = $state(false);

	// Setup tokens state
	let expandedPoolId = $state<string | null>(null);
	let setupTokens = $state<Record<string, SetupToken[]>>({});
	let tokensLoading = $state<Record<string, boolean>>({});
	let tokenLabel = $state("");
	let tokenExpiresHours = $state(24);
	let creatingToken = $state(false);

	// Deleting state
	let deleting = $state<Record<string, boolean>>({});
	let deletingToken = $state<Record<string, boolean>>({});

	const LOCATIONS = ["eu", "us", "asia", "default"];
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
		formLocation = "eu";
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
				const signed = await signRequest(
					activeIdentity.identity,
					"PUT",
					`/api/v1/providers/${providerHex}/pools/${editingPool.poolId}`,
				);
				await updateAgentPool(
					providerHex,
					editingPool.poolId,
					{
						name: formName.trim(),
						location: formLocation,
						provisionerType: formProvisionerType,
					},
					signed.headers,
				);
				actionMessage = `Pool "${formName}" updated`;
			} else {
				// Create new pool
				const signed = await signRequest(
					activeIdentity.identity,
					"POST",
					`/api/v1/providers/${providerHex}/pools`,
				);
				await createAgentPool(
					providerHex,
					{
						name: formName.trim(),
						location: formLocation,
						provisionerType: formProvisionerType,
					},
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

	async function handleDelete(pool: AgentPoolWithStats) {
		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) {
			error = "Missing signing identity";
			return;
		}

		if (!confirm(`Delete pool "${pool.name}"? This cannot be undone.`)) {
			return;
		}

		error = null;
		actionMessage = null;
		deleting = { ...deleting, [pool.poolId]: true };

		try {
			const signed = await signRequest(
				activeIdentity.identity,
				"DELETE",
				`/api/v1/providers/${providerHex}/pools/${pool.poolId}`,
			);
			await deleteAgentPool(providerHex, pool.poolId, signed.headers);
			actionMessage = `Pool "${pool.name}" deleted`;
			await loadData();
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to delete pool";
		} finally {
			deleting = { ...deleting, [pool.poolId]: false };
		}
	}

	async function toggleSetupTokens(poolId: string) {
		if (expandedPoolId === poolId) {
			expandedPoolId = null;
			return;
		}
		expandedPoolId = poolId;
		await loadSetupTokens(poolId);
	}

	async function loadSetupTokens(poolId: string) {
		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) return;

		tokensLoading = { ...tokensLoading, [poolId]: true };
		try {
			const signed = await signRequest(
				activeIdentity.identity,
				"GET",
				`/api/v1/providers/${providerHex}/pools/${poolId}/setup-tokens`,
			);
			const tokens = await listSetupTokens(providerHex, poolId, signed.headers);
			setupTokens = { ...setupTokens, [poolId]: tokens };
		} catch (e) {
			console.error("Failed to load setup tokens:", e);
		} finally {
			tokensLoading = { ...tokensLoading, [poolId]: false };
		}
	}

	async function handleCreateToken(poolId: string) {
		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) {
			error = "Missing signing identity";
			return;
		}

		error = null;
		creatingToken = true;

		try {
			const signed = await signRequest(
				activeIdentity.identity,
				"POST",
				`/api/v1/providers/${providerHex}/pools/${poolId}/setup-tokens`,
			);
			const params: CreateSetupTokenParams = {
				expiresInHours: tokenExpiresHours,
			};
			if (tokenLabel.trim()) {
				params.label = tokenLabel.trim();
			}
			await createSetupToken(providerHex, poolId, params, signed.headers);
			tokenLabel = "";
			await loadSetupTokens(poolId);
			actionMessage = "Setup token created";
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to create token";
		} finally {
			creatingToken = false;
		}
	}

	async function handleDeleteToken(poolId: string, token: string) {
		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) {
			error = "Missing signing identity";
			return;
		}

		error = null;
		deletingToken = { ...deletingToken, [token]: true };

		try {
			const signed = await signRequest(
				activeIdentity.identity,
				"DELETE",
				`/api/v1/providers/${providerHex}/pools/${poolId}/setup-tokens/${token}`,
			);
			await deleteSetupToken(providerHex, poolId, token, signed.headers);
			await loadSetupTokens(poolId);
			actionMessage = "Setup token deleted";
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to delete token";
		} finally {
			deletingToken = { ...deletingToken, [token]: false };
		}
	}

	function copyToClipboard(text: string) {
		navigator.clipboard.writeText(text);
		actionMessage = "Copied to clipboard";
	}

	function formatTimestamp(ns: number): string {
		const date = new Date(ns / 1_000_000);
		return date.toLocaleString();
	}

	function isTokenExpired(token: SetupToken): boolean {
		return token.expiresAtNs < Date.now() * 1_000_000;
	}

	onDestroy(() => {
		unsubscribeAuth?.();
	});
</script>

<div class="space-y-8">
	<header class="flex items-center justify-between">
		<div>
			<h1 class="text-4xl font-bold text-white mb-2">Agent Pools</h1>
			<p class="text-white/60">
				Group agents by location and provisioner type for load distribution
			</p>
		</div>
		{#if isAuthenticated && !loading && !showCreateForm}
			<button
				onclick={() => { showCreateForm = true; }}
				class="px-4 py-2 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 transition-all"
			>
				+ New Pool
			</button>
		{/if}
	</header>

	{#if !isAuthenticated}
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-8 border border-white/20 text-center">
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">🎱</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-white/70">
					Create an account or login to manage your agent pools.
				</p>
				<button
					onclick={handleLogin}
					class="px-8 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 hover:scale-105 transition-all"
				>
					Login / Create Account
				</button>
			</div>
		</div>
	{:else}
		{#if error}
			<div class="bg-red-500/20 border border-red-500/30 rounded-lg p-4 text-red-300">
				{error}
			</div>
		{/if}
		{#if actionMessage}
			<div class="bg-emerald-500/15 border border-emerald-500/30 rounded-lg p-4 text-emerald-300">
				{actionMessage}
			</div>
		{/if}

		{#if loading}
			<div class="flex justify-center items-center py-12">
				<div class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-400"></div>
			</div>
		{:else}
			<!-- Create/Edit Form -->
			{#if showCreateForm}
				<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
					<h2 class="text-xl font-semibold text-white mb-4">
						{editingPool ? "Edit Pool" : "Create Agent Pool"}
					</h2>
					<form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }} class="space-y-4">
						<div>
							<label for="poolName" class="block text-sm text-white/70 mb-1">Pool Name</label>
							<input
								id="poolName"
								type="text"
								bind:value={formName}
								placeholder="e.g., eu-proxmox"
								class="w-full px-4 py-2 bg-white/5 border border-white/20 rounded-lg text-white placeholder-white/40 focus:outline-none focus:border-blue-400"
							/>
						</div>
						<div class="grid grid-cols-2 gap-4">
							<div>
								<label for="location" class="block text-sm text-white/70 mb-1">Location</label>
								<select
									id="location"
									bind:value={formLocation}
									class="w-full px-4 py-2 bg-white/5 border border-white/20 rounded-lg text-white focus:outline-none focus:border-blue-400"
								>
									{#each LOCATIONS as loc}
										<option value={loc}>{loc}</option>
									{/each}
								</select>
							</div>
							<div>
								<label for="provisionerType" class="block text-sm text-white/70 mb-1">Provisioner Type</label>
								<select
									id="provisionerType"
									bind:value={formProvisionerType}
									class="w-full px-4 py-2 bg-white/5 border border-white/20 rounded-lg text-white focus:outline-none focus:border-blue-400"
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
								class="px-4 py-2 rounded-lg text-white/70 hover:text-white hover:bg-white/10 transition-colors"
							>
								Cancel
							</button>
							<button
								type="submit"
								disabled={formSubmitting}
								class="px-6 py-2 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 transition-all disabled:opacity-50"
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
			{#if pools.length === 0 && !showCreateForm}
				<div class="bg-white/5 border border-white/10 rounded-xl p-8 text-center">
					<span class="text-4xl block mb-4">🎱</span>
					<p class="text-white/70 mb-4">No agent pools configured yet.</p>
					<p class="text-white/50 text-sm">
						Create a pool to group agents by location and provisioner type.
					</p>
				</div>
			{:else}
				<div class="space-y-4">
					{#each pools as pool}
						<div class="bg-white/10 backdrop-blur-lg rounded-xl border border-white/20 overflow-hidden">
							<!-- Pool Header -->
							<div class="p-5">
								<div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
									<div class="space-y-2 flex-1">
										<div class="flex items-center gap-3">
											<h3 class="text-xl font-semibold text-white">{pool.name}</h3>
											<span class="px-2 py-0.5 rounded text-xs bg-blue-500/20 text-blue-300 border border-blue-500/30">
												{pool.location}
											</span>
											<span class="px-2 py-0.5 rounded text-xs bg-purple-500/20 text-purple-300 border border-purple-500/30">
												{pool.provisionerType}
											</span>
										</div>
										<div class="flex flex-wrap gap-4 text-sm text-white/60">
											<span class="flex items-center gap-1">
												<span class="text-lg">🤖</span>
												{pool.agentCount} agents ({pool.onlineCount} online)
											</span>
											<span class="flex items-center gap-1">
												<span class="text-lg">📋</span>
												{pool.activeContracts} active contracts
											</span>
										</div>
									</div>
									<div class="flex items-center gap-2">
										<button
											onclick={() => toggleSetupTokens(pool.poolId)}
											class="px-3 py-1.5 rounded-lg text-sm font-medium bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 hover:bg-emerald-500/30 transition-colors"
										>
											{expandedPoolId === pool.poolId ? "Hide Tokens" : "Setup Tokens"}
										</button>
										<button
											onclick={() => startEdit(pool)}
											class="px-3 py-1.5 rounded-lg text-sm font-medium bg-white/10 text-white/80 hover:bg-white/20 transition-colors"
										>
											Edit
										</button>
										<button
											onclick={() => handleDelete(pool)}
											disabled={deleting[pool.poolId] || pool.agentCount > 0}
											title={pool.agentCount > 0 ? "Cannot delete pool with active agents" : "Delete pool"}
											class="px-3 py-1.5 rounded-lg text-sm font-medium bg-red-500/20 text-red-300 border border-red-500/30 hover:bg-red-500/30 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
										>
											{deleting[pool.poolId] ? "..." : "Delete"}
										</button>
									</div>
								</div>
							</div>

							<!-- Setup Tokens Section -->
							{#if expandedPoolId === pool.poolId}
								<div class="border-t border-white/10 bg-white/5 p-5">
									<h4 class="text-sm font-semibold text-white/80 mb-3">Setup Tokens</h4>
									<p class="text-xs text-white/50 mb-4">
										Generate one-time tokens to register new agents to this pool. Use with: <code class="bg-white/10 px-1 rounded">dc-agent setup token --token &lt;TOKEN&gt;</code>
									</p>

									<!-- Create Token Form -->
									<div class="flex flex-wrap gap-2 mb-4">
										<input
											type="text"
											bind:value={tokenLabel}
											placeholder="Optional label (e.g., node-3)"
											class="flex-1 min-w-48 px-3 py-1.5 bg-white/5 border border-white/20 rounded-lg text-sm text-white placeholder-white/40 focus:outline-none focus:border-blue-400"
										/>
										<select
											bind:value={tokenExpiresHours}
											class="px-3 py-1.5 bg-white/5 border border-white/20 rounded-lg text-sm text-white focus:outline-none focus:border-blue-400"
										>
											<option value={1}>1 hour</option>
											<option value={6}>6 hours</option>
											<option value={24}>24 hours</option>
											<option value={72}>3 days</option>
											<option value={168}>7 days</option>
										</select>
										<button
											onclick={() => handleCreateToken(pool.poolId)}
											disabled={creatingToken}
											class="px-4 py-1.5 bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 rounded-lg text-sm font-medium hover:bg-emerald-500/30 transition-colors disabled:opacity-50"
										>
											{creatingToken ? "..." : "+ Token"}
										</button>
									</div>

									<!-- Token List -->
									{#if tokensLoading[pool.poolId]}
										<div class="text-center py-4 text-white/50">Loading tokens...</div>
									{:else if !setupTokens[pool.poolId] || setupTokens[pool.poolId].length === 0}
										<div class="text-center py-4 text-white/50 text-sm">No setup tokens</div>
									{:else}
										<div class="space-y-2">
											{#each setupTokens[pool.poolId] as token}
												{@const expired = isTokenExpired(token)}
												{@const used = !!token.usedAtNs}
												<div class="flex items-center gap-3 p-3 bg-white/5 rounded-lg {expired || used ? 'opacity-60' : ''}">
													<div class="flex-1 min-w-0">
														<div class="flex items-center gap-2 mb-1">
															<code class="text-xs font-mono text-white/90 truncate">{token.token}</code>
															{#if used}
																<span class="px-1.5 py-0.5 rounded text-xs bg-blue-500/20 text-blue-300">Used</span>
															{:else if expired}
																<span class="px-1.5 py-0.5 rounded text-xs bg-red-500/20 text-red-300">Expired</span>
															{:else}
																<span class="px-1.5 py-0.5 rounded text-xs bg-emerald-500/20 text-emerald-300">Active</span>
															{/if}
															{#if token.label}
																<span class="text-xs text-white/50">{token.label}</span>
															{/if}
														</div>
														<div class="text-xs text-white/40">
															{#if used}
																Used {formatTimestamp(token.usedAtNs!)}
															{:else}
																Expires {formatTimestamp(token.expiresAtNs)}
															{/if}
														</div>
													</div>
													{#if !used}
														<button
															onclick={() => copyToClipboard(`dc-agent setup token --token ${token.token}`)}
															class="px-2 py-1 rounded text-xs font-medium bg-emerald-500/20 text-emerald-300 hover:bg-emerald-500/30 transition-colors"
															title="Copy setup command"
														>
															Copy cmd
														</button>
														<button
															onclick={() => copyToClipboard(token.token)}
															class="p-1.5 rounded hover:bg-white/10 text-white/60 hover:text-white transition-colors"
															title="Copy token only"
														>
															<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
																<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3" />
															</svg>
														</button>
													{/if}
													<button
														onclick={() => handleDeleteToken(pool.poolId, token.token)}
														disabled={deletingToken[token.token]}
														class="p-1.5 rounded hover:bg-red-500/20 text-white/60 hover:text-red-300 transition-colors disabled:opacity-50"
														title="Delete token"
													>
														<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
															<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
														</svg>
													</button>
												</div>
											{/each}
										</div>
									{/if}
								</div>
							{/if}
						</div>
					{/each}
				</div>
			{/if}
		{/if}
	{/if}
</div>
