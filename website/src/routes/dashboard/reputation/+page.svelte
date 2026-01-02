<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { authStore } from '$lib/stores/auth';
	import { truncatePubkey } from '$lib/utils/identity';
	import {
		searchReputation,
		type AccountSearchResult
	} from '$lib/services/api-reputation';
	import Icon from '$lib/components/Icons.svelte';

	let searchQuery = $state('');
	let results = $state<AccountSearchResult[]>([]);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let debounceTimeout: ReturnType<typeof setTimeout> | null = null;
	let myUsername = $state<string | null>(null);

	onMount(() => {
		const unsubscribe = authStore.currentIdentity.subscribe((identity) => {
			if (identity?.account?.username) {
				myUsername = identity.account.username;
			}
		});
		return unsubscribe;
	});

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
			<button
				onclick={() => navigateToProfile(myUsername!)}
				class="btn-primary inline-flex items-center gap-2"
			>
				<span>View My Reputation</span>
				<Icon name="arrow-right" size={14} />
			</button>
		</div>
	{/if}

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
	{:else if !searchQuery && !loading}
		<div class="card p-8 text-center">
			<div class="icon-box-accent mx-auto mb-4">
				<Icon name="star" size={20} />
			</div>
			<h2 class="text-lg font-semibold text-white mb-2">Search Reputation</h2>
			<p class="text-neutral-500 text-sm">
				Enter a username, display name, or public key to find accounts
			</p>
			<p class="text-xs text-neutral-600 mt-4">
				All reputation data is public by design to encourage transparency and trust
			</p>
		</div>
	{/if}
</div>
