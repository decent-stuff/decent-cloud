<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { authStore } from '$lib/stores/auth';
	import { truncatePubkey } from '$lib/utils/identity';
	import { getScoreColor } from '$lib/utils/trust-score';
	import {
		searchReputation,
		getReputationLeaderboard,
		type AccountSearchResult,
		type ReputationLeaderboardEntry
	} from '$lib/services/api-reputation';
	import Icon from '$lib/components/Icons.svelte';
	import Button from '$lib/components/Button.svelte';

	let searchQuery = $state('');
	let results = $state<AccountSearchResult[]>([]);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let debounceTimeout: ReturnType<typeof setTimeout> | null = null;
	let myUsername = $state<string | null>(null);

	let leaderboard = $state<ReputationLeaderboardEntry[]>([]);
	let leaderboardLoading = $state(true);
	let leaderboardError = $state<string | null>(null);

	onMount(() => {
		const unsubscribe = authStore.currentIdentity.subscribe((identity) => {
			if (identity?.account?.username) {
				myUsername = identity.account.username;
			}
		});

		loadLeaderboard();

		return unsubscribe;
	});

	async function loadLeaderboard() {
		leaderboardLoading = true;
		leaderboardError = null;
		try {
			leaderboard = await getReputationLeaderboard(20);
		} catch (e) {
			leaderboardError = e instanceof Error ? e.message : 'Failed to load leaderboard';
			leaderboard = [];
		} finally {
			leaderboardLoading = false;
		}
	}

	async function performSearch() {
		if (!searchQuery || searchQuery.trim().length === 0) {
			results = [];
			error = null;
			return;
		}

		loading = true;
		error = null;

		try {
			results = await searchReputation(searchQuery.trim(), 50);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Search failed';
			results = [];
		} finally {
			loading = false;
		}
	}

	function handleInput() {
		if (debounceTimeout) {
			clearTimeout(debounceTimeout);
		}

		debounceTimeout = setTimeout(() => {
			performSearch();
		}, 300);
	}

	function formatNumber(num: number): string {
		return num.toLocaleString();
	}

	function formatVolume(e9s: number): string {
		const value = e9s / 1_000_000_000;
		if (value >= 1000) return `$${(value / 1000).toFixed(1)}k`;
		return `$${value.toFixed(0)}`;
	}

	function displayName(entry: ReputationLeaderboardEntry): string {
		return entry.display_name || entry.username || entry.provider_name;
	}

	function navigateIdentifier(entry: ReputationLeaderboardEntry): string {
		return entry.username || entry.pubkey;
	}

	function navigateToProfile(identifier: string) {
		goto(`/dashboard/reputation/${identifier}`);
	}
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-2xl font-bold text-white tracking-tight">Reputation</h1>
		<p class="text-neutral-500 text-sm mt-1">
			Search for users and providers by username, display name, or public key
		</p>
	</div>

	<!-- My Reputation Link -->
	{#if myUsername}
		<div class="flex gap-3">
			<Button variant="primary" onclick={() => navigateToProfile(myUsername!)} class="inline-flex items-center gap-2">
				<span>View My Reputation</span>
				<Icon name="arrow-right" size={20} />
			</Button>
		</div>
	{/if}

	<!-- Top Providers Leaderboard -->
	<section class="space-y-4">
		<div class="flex items-center justify-between">
			<h2 class="text-lg font-semibold text-white">Top Providers</h2>
			<span class="text-xs text-neutral-500">
				Ranked by trust score &amp; completed contracts
			</span>
		</div>

		{#if leaderboardLoading}
			<div class="card p-5 flex items-center gap-2 text-neutral-400">
				<div class="w-4 h-4 border-2 border-primary-500/30 border-t-primary-500 animate-spin"></div>
				<span class="text-sm">Loading leaderboard...</span>
			</div>
		{:else if leaderboardError}
			<div class="card p-5 text-danger text-sm">{leaderboardError}</div>
		{:else if leaderboard.length === 0}
			<div class="card p-8 text-center">
				<div class="icon-box-accent mx-auto mb-4">
					<Icon name="star" size={20} />
				</div>
				<h3 class="text-base font-semibold text-white mb-2">No providers with completed contracts yet</h3>
				<p class="text-neutral-500 text-sm">
					The leaderboard fills in as providers complete rentals. Check back soon.
				</p>
			</div>
		{:else}
			<div class="card overflow-hidden">
				<table class="w-full text-sm">
					<thead>
						<tr class="text-left text-[11px] uppercase tracking-label text-neutral-500 border-b border-neutral-800/60">
							<th class="py-3 px-4 w-12">#</th>
							<th class="py-3 px-4">Provider</th>
							<th class="py-3 px-4 text-right">Trust</th>
							<th class="py-3 px-4 text-right">Completed</th>
							<th class="py-3 px-4 text-right">Completion</th>
							<th class="py-3 px-4 text-right">Volume</th>
						</tr>
					</thead>
					<tbody>
						{#each leaderboard as entry, i}
							<tr
								class="border-b border-neutral-800/40 last:border-0 hover:bg-neutral-800/30 cursor-pointer"
								onclick={() => navigateToProfile(navigateIdentifier(entry))}
							>
								<td class="py-3 px-4 text-neutral-500 font-mono">{i + 1}</td>
								<td class="py-3 px-4">
									<div class="font-medium text-white truncate">{displayName(entry)}</div>
									<div class="text-xs text-neutral-600 font-mono mt-0.5">
										{truncatePubkey(entry.pubkey)}
									</div>
								</td>
								<td class="py-3 px-4 text-right font-mono">
									{#if entry.trust_score !== undefined && entry.trust_score !== null}
										<span class="font-semibold {getScoreColor(entry.trust_score)}">
											{entry.trust_score}
										</span>
									{:else}
										<span class="text-neutral-600">—</span>
									{/if}
								</td>
								<td class="py-3 px-4 text-right text-white">
									{formatNumber(entry.completed_contracts)}
								</td>
								<td class="py-3 px-4 text-right text-white">
									{entry.completion_rate_pct.toFixed(0)}%
								</td>
								<td class="py-3 px-4 text-right text-white font-mono">
									{formatVolume(entry.volume_e9s)}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</section>

	<!-- Search Box -->
	<div class="card p-5">
		<label for="search" class="data-label block mb-2">
			Search Accounts
		</label>
		<input
			id="search"
			type="text"
			bind:value={searchQuery}
			oninput={handleInput}
			placeholder="Enter username, display name, or public key..."
			class="input w-full"
		/>
		{#if loading}
			<div class="mt-4 flex items-center gap-2 text-neutral-400">
				<div class="w-4 h-4 border-2 border-primary-500/30 border-t-primary-500 animate-spin"></div>
				<span class="text-sm">Searching...</span>
			</div>
		{/if}
		{#if error}
			<div class="mt-4 text-danger text-sm">{error}</div>
		{/if}
	</div>

	<!-- Search Results -->
	{#if results.length > 0}
		<div class="space-y-4">
			<h2 class="text-lg font-semibold text-white">
				Search Results ({results.length})
			</h2>
			<div class="grid grid-cols-1 md:grid-cols-2 gap-3">
				{#each results as result}
					<button
						onclick={() => navigateToProfile(result.username)}
						class="card card-hover p-5 text-left"
					>
						<div class="flex items-start justify-between gap-4 mb-3">
							<div class="flex-1 min-w-0">
								<h3 class="text-base font-semibold text-white mb-1 truncate">
									{result.display_name || result.username}
								</h3>
								{#if result.display_name}
									<p class="text-sm text-neutral-500">@{result.username}</p>
								{/if}
								<p class="text-xs text-neutral-600 font-mono mt-1">
									{truncatePubkey(result.pubkey)}
								</p>
							</div>
							<div class="text-right shrink-0">
								<div class="text-2xl font-bold text-primary-400 font-mono">
									{formatNumber(result.reputation_score)}
								</div>
								<div class="text-[10px] text-neutral-500 uppercase tracking-label">Reputation</div>
							</div>
						</div>
						<div class="flex gap-4 text-sm border-t border-neutral-800/60 pt-3">
							<div>
								<span class="text-neutral-500">Contracts:</span>
								<span class="text-white font-medium ml-1">{formatNumber(result.contract_count)}</span>
							</div>
							<div>
								<span class="text-neutral-500">Offerings:</span>
								<span class="text-white font-medium ml-1">{formatNumber(result.offering_count)}</span>
							</div>
						</div>
					</button>
				{/each}
			</div>
		</div>
	{:else if searchQuery && !loading && !error}
		<div class="card p-8 text-center">
			<div class="icon-box mx-auto mb-4">
				<Icon name="search" size={20} />
			</div>
			<h2 class="text-lg font-semibold text-white mb-2">No Results Found</h2>
			<p class="text-neutral-500 text-sm">
				No accounts match your search query: <span class="font-mono text-neutral-400">"{searchQuery}"</span>
			</p>
			<p class="text-xs text-neutral-600 mt-2">
				Try searching by username, display name, or public key
			</p>
		</div>
	{/if}
</div>
