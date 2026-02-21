<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { page } from '$app/stores';
	import { navigateToLogin } from '$lib/utils/navigation';
	import {
		getPendingPasswordResets,
		hexEncode,
		type Contract
	} from '$lib/services/api';
	import { signRequest } from '$lib/services/auth-api';
	import { authStore } from '$lib/stores/auth';
	import { Ed25519KeyIdentity } from '@dfinity/identity';
	import { truncateContractHash, formatRelativeTime } from '$lib/utils/contract-format';

	const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || '';

	type SigningIdentity = {
		identity: Ed25519KeyIdentity;
		publicKeyBytes: Uint8Array;
	};

	let pendingResets = $state<Contract[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let isAuthenticated = $state(false);
	let providerHex = $state('');
	let signingIdentityInfo = $state<SigningIdentity | null>(null);
	let lastRefresh = $state<number>(Date.now());
	let sseConnected = $state(false);

	let eventSource: EventSource | null = null;
	let unsubscribeAuth: (() => void) | null = null;

	function connectSSE() {
		if (!isAuthenticated || !providerHex) return;
		closeSSE();
		const url = `${API_BASE_URL}/api/v1/providers/${providerHex}/password-reset-events`;
		eventSource = new EventSource(url);
		eventSource.addEventListener('password-reset-count', () => {
			loadData();
		});
		eventSource.onopen = () => {
			sseConnected = true;
		};
		eventSource.onerror = () => {
			sseConnected = false;
		};
	}

	function closeSSE() {
		if (eventSource) {
			eventSource.close();
			eventSource = null;
			sseConnected = false;
		}
	}

	async function refreshData() {
		if (!isAuthenticated || loading) return;
		try {
			await loadData();
		} catch (e) {
			console.error('[Password Resets] Error refreshing:', e);
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
				error = 'You must be authenticated to view password reset requests';
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
				`/api/v1/providers/${providerHex}/contracts/pending-password-reset`
			);
			pendingResets = await getPendingPasswordResets(providerHex, signed.headers);
			lastRefresh = Date.now();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load pending password resets';
		} finally {
			loading = false;
		}
	}

	function handleLogin() {
		navigateToLogin($page.url.pathname);
	}

	async function copyContractId(contractId: string) {
		await navigator.clipboard.writeText(contractId);
	}

	onMount(() => {
		unsubscribeAuth = authStore.isAuthenticated.subscribe((isAuth) => {
			isAuthenticated = isAuth;
			if (isAuth) {
				loadData().then(() => connectSSE());
			} else {
				loading = false;
				closeSSE();
			}
		});
	});

	onDestroy(() => {
		unsubscribeAuth?.();
		closeSSE();
	});
</script>

<div class="space-y-8">
	<div class="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-4">
		<div>
			<h1 class="text-2xl font-bold text-white tracking-tight mb-2">Password Resets</h1>
			<p class="text-neutral-500">
				Tenant-requested password resets pending dc-agent processing
			</p>
		</div>
		{#if isAuthenticated}
			<div class="flex items-center gap-3">
				<div class="flex items-center gap-2 px-3 py-1.5 text-sm {sseConnected ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/30' : 'bg-surface-elevated text-neutral-500 border border-neutral-800'}">
					<span class="relative flex h-2 w-2">
						{#if sseConnected}
							<span class="animate-ping absolute inline-flex h-full w-full bg-emerald-400 opacity-75"></span>
						{/if}
						<span class="relative inline-flex h-2 w-2 {sseConnected ? 'bg-emerald-400' : 'bg-white/30'}"></span>
					</span>
					{sseConnected ? 'Live' : 'Disconnected'}
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
		<div class="card p-8 border border-neutral-800 text-center">
			<div class="max-w-md mx-auto space-y-6">
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-neutral-400">
					Login to view pending password reset requests from tenants.
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

		<div class="bg-surface-elevated border border-neutral-800 p-4 text-neutral-400 text-sm">
			Password resets are handled automatically by dc-agent. This page shows pending requests that haven't been processed yet.
		</div>

		{#if loading}
			<div class="flex justify-center items-center py-12">
				<div class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-primary-400"></div>
			</div>
		{:else}
			<section class="space-y-4">
				<div class="flex items-center justify-between">
					<h2 class="text-xl font-semibold text-white">Pending Requests</h2>
					<span class="text-neutral-500 text-sm">{pendingResets.length} awaiting dc-agent</span>
				</div>

				{#if pendingResets.length === 0}
					<div class="bg-surface-elevated border border-neutral-800 p-6 text-neutral-400">
						No pending password reset requests.
					</div>
				{:else}
					<div class="space-y-3">
						{#each pendingResets as contract}
							<div class="bg-surface-elevated border border-neutral-800 p-4 flex flex-col sm:flex-row sm:items-center gap-4">
								<div class="flex-1 space-y-1 min-w-0">
									<div class="flex items-center gap-3 flex-wrap">
										<span class="font-mono text-sm text-white">{truncateContractHash(contract.contract_id)}</span>
										<span class="text-xs px-2 py-0.5 bg-amber-500/20 text-amber-300 border border-amber-500/30">
											password reset pending
										</span>
									</div>
									<div class="text-sm text-neutral-400">
										Tenant: <span class="font-mono">{truncateContractHash(contract.requester_pubkey)}</span>
									</div>
									<div class="text-xs text-neutral-500">
										Requested: {formatRelativeTime(contract.status_updated_at_ns ?? contract.created_at_ns)}
									</div>
								</div>
								<button
									onclick={() => copyContractId(contract.contract_id)}
									class="shrink-0 px-3 py-1.5 text-sm bg-surface-elevated text-neutral-400 border border-neutral-800 hover:text-white hover:border-neutral-600 transition-colors"
									title="Copy contract ID to clipboard"
								>
									Copy ID
								</button>
							</div>
						{/each}
					</div>
				{/if}
			</section>
		{/if}
	{/if}
</div>
