<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import AuthRequiredCard from '$lib/components/AuthRequiredCard.svelte';
	import {
		getPendingSshKeyRotations,
		hexEncode,
		type Contract
	} from '$lib/services/api';
	import { signRequest } from '$lib/services/auth-api';
	import { authStore } from '$lib/stores/auth';
	import { Ed25519KeyIdentity } from '@dfinity/identity';
	import { truncateContractHash, formatRelativeTime } from '$lib/utils/contract-format';

	type SigningIdentity = {
		identity: Ed25519KeyIdentity;
		publicKeyBytes: Uint8Array;
	};

	let pendingRotations = $state<Contract[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let isAuthenticated = $state(false);
	let providerHex = $state('');
	let signingIdentityInfo = $state<SigningIdentity | null>(null);
	let lastRefresh = $state<number>(Date.now());
	let refreshInterval: ReturnType<typeof setInterval> | null = null;

	let unsubscribeAuth: (() => void) | null = null;

	function startAutoRefresh() {
		stopAutoRefresh();
		refreshInterval = setInterval(() => {
			refreshData();
		}, 15_000);
	}

	function stopAutoRefresh() {
		if (refreshInterval) {
			clearInterval(refreshInterval);
			refreshInterval = null;
		}
	}

	async function refreshData() {
		if (!isAuthenticated || loading || !signingIdentityInfo || !providerHex) return;
		try {
			const signed = await signRequest(
				signingIdentityInfo.identity,
				'GET',
				`/api/v1/providers/${providerHex}/contracts`
			);
			pendingRotations = await getPendingSshKeyRotations(providerHex, signed.headers);
			lastRefresh = Date.now();
		} catch (e) {
			console.error('[SSH Key Rotations] Error refreshing:', e);
		}
	}

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
				error = 'You must be authenticated to view SSH key rotation requests';
				return;
			}
			if (!(info.identity instanceof Ed25519KeyIdentity)) {
				error = 'Only Ed25519 identities can sign provider actions';
				return;
			}
			const normalizedIdentity: SigningIdentity = {
				identity: info.identity,
				publicKeyBytes: info.publicKeyBytes
			};
			signingIdentityInfo = normalizedIdentity;
			providerHex = hexEncode(normalizedIdentity.publicKeyBytes);

			const signed = await signRequest(
				normalizedIdentity.identity,
				'GET',
				`/api/v1/providers/${providerHex}/contracts`
			);
			pendingRotations = await getPendingSshKeyRotations(providerHex, signed.headers);
			lastRefresh = Date.now();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load pending SSH key rotations';
		} finally {
			loading = false;
		}
	}

	async function copyContractId(contractId: string) {
		await navigator.clipboard.writeText(contractId);
	}

	onMount(() => {
		unsubscribeAuth = authStore.isAuthenticated.subscribe((isAuth) => {
			isAuthenticated = isAuth;
			if (isAuth) {
				loadData().then(() => startAutoRefresh());
			} else {
				loading = false;
				stopAutoRefresh();
			}
		});
	});

	onDestroy(() => {
		unsubscribeAuth?.();
		stopAutoRefresh();
	});
</script>

<div class="space-y-8">
	<div class="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-4">
		<div>
			<h1 class="text-2xl font-bold text-white tracking-tight mb-2">SSH Key Rotations</h1>
			<p class="text-neutral-500">
				Tenant-requested SSH key rotations awaiting dc-agent completion
			</p>
		</div>
		{#if isAuthenticated}
			<div class="flex items-center gap-3">
				<div class="px-3 py-1.5 text-sm bg-surface-elevated text-neutral-500 border border-neutral-800">
					Updated {formatRelativeTime(lastRefresh * 1_000_000)}
				</div>
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
		<AuthRequiredCard subtext="Login to view pending SSH key rotation requests from tenants." />
	{:else}
		{#if error}
			<div class="bg-red-500/20 border border-red-500/30 p-4 text-red-300">
				{error}
			</div>
		{/if}

		<div class="bg-surface-elevated border border-neutral-800 p-4 text-neutral-400 text-sm">
			SSH key rotations are completed by dc-agent after it injects the tenant's new key into the VM. This page shows contracts still waiting for that completion step.
		</div>

		{#if loading}
			<div class="flex justify-center items-center py-12">
				<div class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-primary-400"></div>
			</div>
		{:else}
			<section class="space-y-4">
				<div class="flex items-center justify-between">
					<h2 class="text-xl font-semibold text-white">Pending Requests</h2>
					<span class="text-neutral-500 text-sm">{pendingRotations.length} awaiting dc-agent</span>
				</div>

				{#if pendingRotations.length === 0}
					<div class="bg-surface-elevated border border-neutral-800 p-6 text-neutral-400">
						No pending SSH key rotation requests.
					</div>
				{:else}
					<div class="space-y-3">
						{#each pendingRotations as contract}
							<div class="bg-surface-elevated border border-neutral-800 p-4 flex flex-col sm:flex-row sm:items-center gap-4">
								<div class="flex-1 space-y-1 min-w-0">
									<div class="flex items-center gap-3 flex-wrap">
										<span class="font-mono text-sm text-white">{truncateContractHash(contract.contract_id)}</span>
										<span class="text-xs px-2 py-0.5 bg-primary-500/20 text-primary-300 border border-primary-500/30">
											SSH key rotation pending
										</span>
									</div>
									<div class="text-sm text-neutral-400">
										Tenant: <span class="font-mono">{truncateContractHash(contract.requester_pubkey)}</span>
									</div>
									<div class="text-sm text-neutral-400 truncate">
										Current key: <span class="font-mono">{truncateContractHash(contract.requester_ssh_pubkey)}</span>
									</div>
									<div class="text-xs text-neutral-500">
										Requested: {formatRelativeTime(contract.ssh_key_rotation_requested_at_ns ?? contract.status_updated_at_ns ?? contract.created_at_ns)}
									</div>
								</div>
								<div class="flex items-center gap-2 shrink-0">
									<a
										href={`/dashboard/rentals/${contract.contract_id}`}
										class="px-3 py-1.5 text-sm bg-primary-500/20 text-primary-300 border border-primary-500/30 hover:bg-primary-500/30 transition-colors"
									>
										View Contract
									</a>
									<button
										onclick={() => copyContractId(contract.contract_id)}
										class="px-3 py-1.5 text-sm bg-surface-elevated text-neutral-400 border border-neutral-800 hover:text-white hover:border-neutral-600 transition-colors"
										title="Copy contract ID to clipboard"
									>
										Copy ID
									</button>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</section>
		{/if}
	{/if}
</div>
