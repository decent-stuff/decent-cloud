<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { page } from "$app/stores";
	import { navigateToLogin } from "$lib/utils/navigation";
	import {
		getProviderAgentStatus,
		getProviderAgentDelegations,
		revokeAgentDelegation,
		hexEncode,
		type AgentStatus,
		type AgentDelegation,
	} from "$lib/services/api";
	import { signRequest } from "$lib/services/auth-api";
	import { authStore } from "$lib/stores/auth";
	import { Ed25519KeyIdentity } from "@dfinity/identity";

	let agentStatus = $state<AgentStatus | null>(null);
	let delegations = $state<AgentDelegation[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let actionMessage = $state<string | null>(null);
	let providerHex = $state("");
	let isAuthenticated = $state(false);
	let unsubscribeAuth: (() => void) | null = null;
	let revoking = $state<Record<string, boolean>>({});

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
				error = "You must be authenticated to view agent status";
				return;
			}
			if (!(info.identity instanceof Ed25519KeyIdentity)) {
				error = "Only Ed25519 identities can view agent status";
				return;
			}
			const normalizedIdentity: SigningIdentity = {
				identity: info.identity,
				publicKeyBytes: info.publicKeyBytes,
			};
			signingIdentityInfo = normalizedIdentity;
			providerHex = hexEncode(normalizedIdentity.publicKeyBytes);

			// Fetch agent status (public endpoint)
			agentStatus = await getProviderAgentStatus(providerHex);

			// Fetch delegations (requires auth)
			const delegationsSigned = await signRequest(
				normalizedIdentity.identity,
				"GET",
				`/api/v1/providers/${providerHex}/agent-delegations`,
			);
			delegations = await getProviderAgentDelegations(
				providerHex,
				delegationsSigned.headers,
			);
		} catch (e) {
			error =
				e instanceof Error
					? e.message
					: "Failed to load agent information";
		} finally {
			loading = false;
		}
	}

	function handleLogin() {
		navigateToLogin($page.url.pathname);
	}

	async function handleRevoke(agentPubkey: string) {
		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) {
			error = "Missing signing identity";
			return;
		}
		error = null;
		actionMessage = null;
		revoking = { ...revoking, [agentPubkey]: true };
		try {
			const signed = await signRequest(
				activeIdentity.identity,
				"DELETE",
				`/api/v1/providers/${providerHex}/agent-delegations/${agentPubkey}`,
			);
			await revokeAgentDelegation(providerHex, agentPubkey, signed.headers);
			actionMessage = "Agent delegation revoked";
			await loadData();
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to revoke delegation";
		} finally {
			revoking = { ...revoking, [agentPubkey]: false };
		}
	}

	function formatTimestamp(ns: number | undefined): string {
		if (!ns) return "Never";
		const date = new Date(ns / 1_000_000);
		return date.toLocaleString();
	}

	function formatTimeAgo(ns: number | undefined): string {
		if (!ns) return "Never";
		const now = Date.now();
		const timestampMs = ns / 1_000_000;
		const diffMs = now - timestampMs;

		if (diffMs < 60_000) {
			return `${Math.floor(diffMs / 1000)}s ago`;
		} else if (diffMs < 3600_000) {
			return `${Math.floor(diffMs / 60_000)}m ago`;
		} else if (diffMs < 86400_000) {
			return `${Math.floor(diffMs / 3600_000)}h ago`;
		} else {
			return `${Math.floor(diffMs / 86400_000)}d ago`;
		}
	}

	function truncatePubkey(pubkey: string): string {
		if (pubkey.length <= 16) return pubkey;
		return `${pubkey.slice(0, 8)}...${pubkey.slice(-8)}`;
	}

	const activeDelegations = $derived(delegations.filter((d) => d.active));
	const revokedDelegations = $derived(delegations.filter((d) => !d.active));

	onDestroy(() => {
		unsubscribeAuth?.();
	});
</script>

<div class="space-y-8">
	<header>
		<h1 class="text-4xl font-bold text-white mb-2">DC-Agent Status</h1>
		<p class="text-white/60">
			Monitor your provisioning agents and manage delegations
		</p>
	</header>

	{#if !isAuthenticated}
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-8 border border-white/20 text-center">
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">🤖</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-white/70">
					Create an account or login to view your DC-Agent status and manage delegations.
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
			<!-- Agent Status Card -->
			<section class="space-y-4">
				<h2 class="text-2xl font-semibold text-white">Agent Status</h2>

				{#if agentStatus}
					<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
						<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
							<!-- Online Status -->
							<div class="space-y-2">
								<div class="text-white/60 text-sm">Status</div>
								<div class="flex items-center gap-2">
									<span class="relative flex h-3 w-3">
										{#if agentStatus.online}
											<span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75"></span>
											<span class="relative inline-flex rounded-full h-3 w-3 bg-green-500"></span>
										{:else}
											<span class="relative inline-flex rounded-full h-3 w-3 bg-red-500"></span>
										{/if}
									</span>
									<span class="text-lg font-semibold {agentStatus.online ? 'text-green-400' : 'text-red-400'}">
										{agentStatus.online ? 'Online' : 'Offline'}
									</span>
								</div>
							</div>

							<!-- Last Heartbeat -->
							<div class="space-y-2">
								<div class="text-white/60 text-sm">Last Heartbeat</div>
								<div class="text-white font-medium">
									{formatTimeAgo(agentStatus.lastHeartbeatNs)}
								</div>
								<div class="text-white/40 text-xs">
									{formatTimestamp(agentStatus.lastHeartbeatNs)}
								</div>
							</div>

							<!-- Version -->
							<div class="space-y-2">
								<div class="text-white/60 text-sm">Version</div>
								<div class="text-white font-medium">
									{agentStatus.version ?? 'Unknown'}
								</div>
							</div>

							<!-- Active Contracts -->
							<div class="space-y-2">
								<div class="text-white/60 text-sm">Active Contracts</div>
								<div class="text-white font-medium text-2xl">
									{agentStatus.activeContracts}
								</div>
							</div>
						</div>

						<!-- Additional Info Row -->
						<div class="mt-6 pt-6 border-t border-white/10 grid grid-cols-1 md:grid-cols-2 gap-6">
							<!-- Provisioner Type -->
							<div class="space-y-2">
								<div class="text-white/60 text-sm">Provisioner Type</div>
								<div class="text-white font-medium">
									{#if agentStatus.provisionerType}
										<span class="inline-flex items-center px-3 py-1 rounded-full text-sm bg-blue-500/20 text-blue-300 border border-blue-500/30">
											{agentStatus.provisionerType}
										</span>
									{:else}
										<span class="text-white/40">Not specified</span>
									{/if}
								</div>
							</div>

							<!-- Capabilities -->
							<div class="space-y-2">
								<div class="text-white/60 text-sm">Capabilities</div>
								<div class="flex flex-wrap gap-2">
									{#if agentStatus.capabilities && agentStatus.capabilities.length > 0}
										{#each agentStatus.capabilities as capability}
											<span class="inline-flex items-center px-2 py-1 rounded text-xs bg-purple-500/20 text-purple-300 border border-purple-500/30">
												{capability}
											</span>
										{/each}
									{:else}
										<span class="text-white/40">None reported</span>
									{/if}
								</div>
							</div>
						</div>
					</div>
				{:else}
					<div class="bg-white/5 border border-white/10 rounded-xl p-6 text-white/70">
						<div class="flex items-center gap-4">
							<span class="text-4xl">🤖</span>
							<div>
								<div class="font-medium text-white">No agent registered</div>
								<div class="text-sm">
									Set up a DC-Agent to automate provisioning for your offerings.
									<a href="https://docs.decent-cloud.org/dc-agent" target="_blank" rel="noopener noreferrer" class="text-blue-400 hover:underline ml-1">
										Learn more
									</a>
								</div>
							</div>
						</div>
					</div>
				{/if}
			</section>

			<!-- Active Delegations -->
			<section class="space-y-4">
				<div class="flex items-center justify-between">
					<h2 class="text-2xl font-semibold text-white">Active Delegations</h2>
					<span class="text-white/60 text-sm">{activeDelegations.length} active</span>
				</div>

				{#if activeDelegations.length === 0}
					<div class="bg-white/5 border border-white/10 rounded-xl p-6 text-white/70">
						No active agent delegations.
					</div>
				{:else}
					<div class="space-y-3">
						{#each activeDelegations as delegation}
							<div class="bg-white/10 backdrop-blur-lg rounded-xl p-5 border border-white/20">
								<div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
									<div class="space-y-2 flex-1">
										<div class="flex items-center gap-3">
											<span class="font-mono text-white/90 text-sm" title={delegation.agentPubkey}>
												{truncatePubkey(delegation.agentPubkey)}
											</span>
											{#if delegation.label}
												<span class="px-2 py-0.5 rounded text-xs bg-white/10 text-white/70">
													{delegation.label}
												</span>
											{/if}
										</div>
										<div class="flex flex-wrap gap-2">
											{#each delegation.permissions as permission}
												<span class="inline-flex items-center px-2 py-0.5 rounded text-xs bg-emerald-500/20 text-emerald-300 border border-emerald-500/30">
													{permission}
												</span>
											{/each}
										</div>
										<div class="text-white/40 text-xs">
											Created {formatTimestamp(delegation.createdAtNs)}
											{#if delegation.expiresAtNs}
												<span class="ml-2">
													Expires {formatTimestamp(delegation.expiresAtNs)}
												</span>
											{/if}
										</div>
									</div>
									<button
										onclick={() => handleRevoke(delegation.agentPubkey)}
										disabled={revoking[delegation.agentPubkey]}
										class="px-4 py-2 rounded-lg text-sm font-medium bg-red-500/20 text-red-300 border border-red-500/30 hover:bg-red-500/30 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
									>
										{#if revoking[delegation.agentPubkey]}
											Revoking...
										{:else}
											Revoke
										{/if}
									</button>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</section>

			<!-- Revoked Delegations (collapsed by default) -->
			{#if revokedDelegations.length > 0}
				<section class="space-y-4">
					<details class="group">
						<summary class="flex items-center justify-between cursor-pointer list-none">
							<h2 class="text-xl font-semibold text-white/60">Revoked Delegations</h2>
							<span class="text-white/40 text-sm group-open:hidden">{revokedDelegations.length} revoked - click to expand</span>
						</summary>
						<div class="mt-4 space-y-3">
							{#each revokedDelegations as delegation}
								<div class="bg-white/5 rounded-xl p-4 border border-white/10 opacity-60">
									<div class="space-y-2">
										<div class="flex items-center gap-3">
											<span class="font-mono text-white/70 text-sm line-through" title={delegation.agentPubkey}>
												{truncatePubkey(delegation.agentPubkey)}
											</span>
											{#if delegation.label}
												<span class="px-2 py-0.5 rounded text-xs bg-white/5 text-white/50">
													{delegation.label}
												</span>
											{/if}
											<span class="px-2 py-0.5 rounded text-xs bg-red-500/10 text-red-400">
												Revoked
											</span>
										</div>
										<div class="text-white/30 text-xs">
											Created {formatTimestamp(delegation.createdAtNs)}
										</div>
									</div>
								</div>
							{/each}
						</div>
					</details>
				</section>
			{/if}
		{/if}
	{/if}
</div>
