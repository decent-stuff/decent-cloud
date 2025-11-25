<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { authStore } from '$lib/stores/auth';
	import { computePubkey } from '$lib/utils/contract-format';
	import {
		searchReputation,
		type AccountSearchResult
	} from '$lib/services/api-reputation';

	let searchQuery = $state('');
	let results = $state<AccountSearchResult[]>([]);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let debounceTimeout: ReturnType<typeof setTimeout> | null = null;
	let myPubkey = $state<string | null>(null);

	onMount(async () => {
		// Get user's own pubkey for "My Reputation" link
		const isAuth = await new Promise<boolean>((resolve) => {
			const unsubscribe = authStore.isAuthenticated.subscribe((value) => {
				resolve(value);
				unsubscribe();
			});
		});

		if (isAuth) {
			const identity = await authStore.getSigningIdentity();
			if (identity?.publicKeyBytes) {
				myPubkey = computePubkey(identity.publicKeyBytes);
			}
		}
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
		// Debounce search
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

	function shortPubkey(fullPubkey: string): string {
		if (fullPubkey.length <= 12) return fullPubkey;
		return `${fullPubkey.slice(0, 6)}...${fullPubkey.slice(-6)}`;
	}

	function navigateToProfile(pubkey: string) {
		goto(`/dashboard/reputation/${pubkey}`);
	}
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">Reputation</h1>
		<p class="text-white/60">
			Search for users and providers by username, display name, or public key
		</p>
	</div>

	<!-- My Reputation Link -->
	{#if myPubkey}
		<div class="flex gap-4">
			<button
				onclick={() => navigateToProfile(myPubkey!)}
				class="px-6 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 hover:scale-105 transition-all"
			>
				View My Reputation
			</button>
		</div>
	{/if}

	<!-- Search Box -->
	<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
		<label for="search" class="block text-sm font-medium text-white/80 mb-2">
			Search Accounts
		</label>
		<input
			id="search"
			type="text"
			bind:value={searchQuery}
			oninput={handleInput}
			placeholder="Enter username, display name, or public key..."
			class="w-full px-4 py-3 bg-white/5 border border-white/20 rounded-lg text-white placeholder-white/40 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
		/>
		{#if loading}
			<div class="mt-4 flex items-center gap-2 text-white/60">
				<div
					class="animate-spin rounded-full h-4 w-4 border-t-2 border-b-2 border-blue-400"
				></div>
				<span class="text-sm">Searching...</span>
			</div>
		{/if}
		{#if error}
			<div class="mt-4 text-red-400 text-sm">{error}</div>
		{/if}
	</div>

	<!-- Search Results -->
	{#if results.length > 0}
		<div class="space-y-4">
			<h2 class="text-2xl font-bold text-white">
				Search Results ({results.length})
			</h2>
			<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
				{#each results as result}
					<button
						onclick={() => navigateToProfile(result.pubkey)}
						class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20 hover:bg-white/15 hover:border-white/30 transition-all text-left"
					>
						<div class="flex items-start justify-between gap-4 mb-3">
							<div class="flex-1">
								<h3 class="text-lg font-bold text-white mb-1">
									{result.display_name || result.username}
								</h3>
								{#if result.display_name}
									<p class="text-sm text-white/60">@{result.username}</p>
								{/if}
								<p class="text-xs text-white/50 font-mono mt-1">
									{shortPubkey(result.pubkey)}
								</p>
							</div>
							<div class="text-right">
								<div class="text-2xl font-bold text-white">
									{formatNumber(result.reputation_score)}
								</div>
								<div class="text-xs text-white/50">Reputation</div>
							</div>
						</div>
						<div class="flex gap-4 text-sm">
							<div>
								<span class="text-white/60">Contracts:</span>
								<span class="text-white font-medium ml-1"
									>{formatNumber(result.contract_count)}</span
								>
							</div>
							<div>
								<span class="text-white/60">Offerings:</span>
								<span class="text-white font-medium ml-1"
									>{formatNumber(result.offering_count)}</span
								>
							</div>
						</div>
					</button>
				{/each}
			</div>
		</div>
	{:else if searchQuery && !loading && !error}
		<div
			class="bg-white/10 backdrop-blur-lg rounded-xl p-8 border border-white/20 text-center"
		>
			<div class="text-6xl mb-4">🔍</div>
			<h2 class="text-2xl font-bold text-white mb-2">No Results Found</h2>
			<p class="text-white/60">
				No accounts match your search query: <span class="font-mono"
					>"{searchQuery}"</span
				>
			</p>
			<p class="text-sm text-white/50 mt-2">
				Try searching by username, display name, or public key
			</p>
		</div>
	{:else if !searchQuery && !loading}
		<div
			class="bg-white/10 backdrop-blur-lg rounded-xl p-8 border border-white/20 text-center"
		>
			<div class="text-6xl mb-4">⭐</div>
			<h2 class="text-2xl font-bold text-white mb-2">Search Reputation</h2>
			<p class="text-white/60">
				Enter a username, display name, or public key to find accounts
			</p>
			<p class="text-sm text-white/50 mt-4">
				All reputation data is public by design to encourage transparency and trust
			</p>
		</div>
	{/if}
</div>
