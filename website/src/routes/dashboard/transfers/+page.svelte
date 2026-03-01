<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { hexEncode } from "$lib/services/api";
	import AuthRequiredCard from "$lib/components/AuthRequiredCard.svelte";
	import {
		getAccountTransfers,
		getRecentTransfers,
		getAccountBalance,
		type TokenTransfer,
	} from "$lib/services/api-reputation";
	import { authStore } from "$lib/stores/auth";
	import { truncatePubkey } from "$lib/utils/identity";
	import Icon from "$lib/components/Icons.svelte";

	let transfers = $state<TokenTransfer[]>([]);
	let balance = $state<number>(0);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let isAuthenticated = $state(false);
	let userPubkey = $state<string>("");
	let viewMode = $state<"mine" | "all">("mine");
	let unsubscribeAuth: (() => void) | null = null;

	function formatTokens(e9s: number): string {
		return (e9s / 1_000_000_000).toFixed(4);
	}

	function formatDate(ns: number): string {
		return new Date(ns / 1_000_000).toLocaleDateString("en-US", {
			year: "numeric",
			month: "short",
			day: "numeric",
			hour: "2-digit",
			minute: "2-digit",
		});
	}

	function isSent(transfer: TokenTransfer): boolean {
		return transfer.from_account === userPubkey;
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
			if (!info) {
				error = "You must be authenticated to view transfers";
				return;
			}

			userPubkey = hexEncode(info.publicKeyBytes);

			const [accountBalance, accountTransfers] = await Promise.all([
				getAccountBalance(userPubkey),
				viewMode === "mine"
					? getAccountTransfers(userPubkey)
					: getRecentTransfers(),
			]);

			balance = accountBalance;
			transfers = accountTransfers;
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load transfer data";
		} finally {
			loading = false;
		}
	}

	function switchView(mode: "mine" | "all") {
		if (viewMode === mode) return;
		viewMode = mode;
		loadData();
	}

	onDestroy(() => {
		unsubscribeAuth?.();
	});
</script>

<div class="space-y-8">
	<header>
		<h1 class="text-2xl font-bold text-white tracking-tight">Transfers</h1>
		<p class="text-neutral-500">Token balance and transfer history</p>
	</header>

	{#if !isAuthenticated}
		<AuthRequiredCard subtext="Create an account or login to view your token balance and transfer history." />
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
			<!-- Balance Card -->
			<section>
				<div class="bg-surface-elevated border border-neutral-800 p-6">
					<p class="text-neutral-500 text-sm">Token Balance</p>
					<p class="text-3xl font-bold text-white mt-1">
						{formatTokens(balance)}
					</p>
				</div>
			</section>

			<!-- View Toggle -->
			<section class="space-y-4">
				<div class="flex items-center gap-2">
					<button
						onclick={() => switchView("mine")}
						class="px-4 py-2 text-sm font-medium transition-colors {viewMode === 'mine'
							? 'bg-primary-500/20 text-primary-400 border border-primary-500/30'
							: 'bg-surface-elevated text-neutral-400 border border-neutral-800 hover:text-white'}"
					>
						My Transfers
					</button>
					<button
						onclick={() => switchView("all")}
						class="px-4 py-2 text-sm font-medium transition-colors {viewMode === 'all'
							? 'bg-primary-500/20 text-primary-400 border border-primary-500/30'
							: 'bg-surface-elevated text-neutral-400 border border-neutral-800 hover:text-white'}"
					>
						All Recent
					</button>
				</div>

				<!-- Transfer List -->
				{#if transfers.length === 0}
					<div class="text-center py-16">
						<Icon name="activity" size={48} class="mx-auto text-neutral-600 mb-4" />
						<h3 class="text-xl font-bold text-white mb-2">No Transfers</h3>
						<p class="text-neutral-500">
							{viewMode === "mine"
								? "You have no token transfers yet."
								: "No recent platform transfers found."}
						</p>
					</div>
				{:else}
					<div class="space-y-2">
						{#each transfers as transfer}
							{@const sent = isSent(transfer)}
							{@const counterparty = sent ? transfer.to_account : transfer.from_account}
							<div class="bg-surface-elevated border border-neutral-800 p-4 flex items-center gap-4">
								<!-- Direction indicator -->
								<div class="flex-shrink-0">
									{#if sent}
										<div class="w-8 h-8 rounded-full bg-red-500/20 flex items-center justify-center">
											<Icon name="arrow-right" size={16} class="text-red-400" />
										</div>
									{:else}
										<div class="w-8 h-8 rounded-full bg-green-500/20 flex items-center justify-center">
											<Icon name="arrow-left" size={16} class="text-green-400" />
										</div>
									{/if}
								</div>

								<!-- Details -->
								<div class="flex-1 min-w-0">
									<div class="flex items-center gap-2">
										<span class="text-neutral-500 text-xs">
											{sent ? "To" : "From"}
										</span>
										<span class="text-white text-sm font-mono truncate">
											{truncatePubkey(counterparty)}
										</span>
									</div>
									{#if transfer.memo}
										<p class="text-neutral-500 text-xs mt-1 truncate">
											{transfer.memo}
										</p>
									{/if}
									<p class="text-neutral-600 text-xs mt-1">
										{formatDate(transfer.created_at_ns)}
									</p>
								</div>

								<!-- Amount -->
								<div class="flex-shrink-0 text-right">
									<span class="text-sm font-bold {sent ? 'text-red-400' : 'text-green-400'}">
										{sent ? "-" : "+"}{formatTokens(transfer.amount_e9s)}
									</span>
									{#if transfer.fee_e9s > 0}
										<p class="text-neutral-600 text-xs">
											fee: {formatTokens(transfer.fee_e9s)}
										</p>
									{/if}
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</section>
		{/if}
	{/if}
</div>
