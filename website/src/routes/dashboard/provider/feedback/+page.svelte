<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import AuthRequiredCard from '$lib/components/AuthRequiredCard.svelte';
	import {
		getProviderFeedbackList,
		getProviderFeedbackStats,
		hexEncode,
		type ProviderContractFeedback,
		type ProviderFeedbackStats
	} from '$lib/services/api';
	import { signRequest } from '$lib/services/auth-api';
	import { authStore } from '$lib/stores/auth';
	import { Ed25519KeyIdentity } from '@dfinity/identity';

	let feedbackList = $state<ProviderContractFeedback[]>([]);
	let feedbackStats = $state<ProviderFeedbackStats | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let isAuthenticated = $state(false);
	let unsubscribeAuth: (() => void) | null = null;
	let copiedContractId = $state<string | null>(null);

	function formatNsTimestamp(ns: number): string {
		return new Date(ns / 1_000_000).toLocaleString();
	}

	function truncateId(id: string): string {
		return id.length > 12 ? `${id.slice(0, 8)}...` : id;
	}

	async function copyContractId(id: string) {
		await navigator.clipboard.writeText(id);
		copiedContractId = id;
		setTimeout(() => { copiedContractId = null; }, 1500);
	}

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
				error = 'You must be authenticated with a signing identity to view feedback';
				return;
			}

			const providerHex = hexEncode(info.publicKeyBytes);

			const signed = await signRequest(info.identity, 'GET', `/api/v1/providers/${providerHex}/feedback`);

			const [list, stats] = await Promise.all([
				getProviderFeedbackList(providerHex, signed.headers),
				getProviderFeedbackStats(providerHex).catch(() => null)
			]);

			feedbackList = list;
			feedbackStats = stats;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load feedback';
		} finally {
			loading = false;
		}
	}

	onDestroy(() => {
		unsubscribeAuth?.();
	});
</script>

<div class="space-y-8">
	<header>
		<h1 class="text-2xl font-bold text-white tracking-tight">Tenant Feedback</h1>
		<p class="text-neutral-500">Individual feedback submitted by your tenants after contracts</p>
	</header>

	{#if !isAuthenticated}
		<AuthRequiredCard subtext="Login to view tenant feedback for your contracts." />
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
			<!-- Summary stats -->
			{#if feedbackStats && feedbackStats.total_responses > 0}
				<section class="space-y-4">
					<h2 class="text-xl font-semibold text-white">Summary</h2>
					<div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
						<div class="bg-surface-elevated border border-neutral-800 p-6">
							<p class="text-neutral-500 text-sm">Total Responses</p>
							<p class="text-3xl font-bold text-white mt-1">{feedbackStats.total_responses}</p>
						</div>
						<div class="bg-surface-elevated border border-neutral-800 p-6">
							<p class="text-neutral-500 text-sm">Service Match Rate</p>
							<p class="text-3xl font-bold text-emerald-400 mt-1">
								{feedbackStats.service_match_rate_pct.toFixed(0)}%
							</p>
							<p class="text-neutral-600 text-xs mt-1">Said service matched description</p>
						</div>
						<div class="bg-surface-elevated border border-neutral-800 p-6">
							<p class="text-neutral-500 text-sm">Would Rent Again</p>
							<p class="text-3xl font-bold text-primary-400 mt-1">
								{feedbackStats.would_rent_again_rate_pct.toFixed(0)}%
							</p>
							<p class="text-neutral-600 text-xs mt-1">Would rent from you again</p>
						</div>
					</div>
				</section>
			{/if}

			<!-- Feedback table -->
			<section class="space-y-4">
				<h2 class="text-xl font-semibold text-white">All Feedback</h2>
				{#if feedbackList.length === 0}
					<div class="bg-surface-elevated border border-neutral-800 p-12 text-center">
						<p class="text-neutral-500 text-sm">No feedback received yet.</p>
						<p class="text-neutral-600 text-xs mt-2">
							Tenants can submit feedback after their contracts end.
						</p>
					</div>
				{:else}
					<div class="bg-surface-elevated border border-neutral-800 overflow-x-auto">
						<table class="w-full text-sm">
							<thead>
								<tr class="border-b border-neutral-800">
									<th class="text-left text-neutral-500 font-medium px-4 py-3">Date</th>
									<th class="text-left text-neutral-500 font-medium px-4 py-3">Contract</th>
									<th class="text-center text-neutral-500 font-medium px-4 py-3">Service Match</th>
									<th class="text-center text-neutral-500 font-medium px-4 py-3">Would Rent Again</th>
								</tr>
							</thead>
							<tbody>
								{#each feedbackList as entry}
									<tr class="border-b border-neutral-800/50 hover:bg-neutral-800/30 transition-colors">
										<td class="px-4 py-3 text-neutral-400 text-xs whitespace-nowrap">
											{formatNsTimestamp(entry.created_at_ns)}
										</td>
										<td class="px-4 py-3">
											<button
												onclick={() => copyContractId(entry.contract_id)}
												class="font-mono text-neutral-300 text-xs hover:text-primary-400 transition-colors cursor-pointer"
												title="Click to copy full ID: {entry.contract_id}"
											>
												{copiedContractId === entry.contract_id ? 'Copied!' : truncateId(entry.contract_id)}
											</button>
										</td>
										<td class="px-4 py-3 text-center">
											{#if entry.service_matched_description}
												<span class="text-emerald-400 font-bold">&#10003;</span>
											{:else}
												<span class="text-red-400 font-bold">&#10007;</span>
											{/if}
										</td>
										<td class="px-4 py-3 text-center">
											{#if entry.would_rent_again}
												<span class="text-emerald-400 font-bold">&#10003;</span>
											{:else}
												<span class="text-red-400 font-bold">&#10007;</span>
											{/if}
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				{/if}
			</section>
		{/if}
	{/if}
</div>
