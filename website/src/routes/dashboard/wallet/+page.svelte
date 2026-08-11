<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { page } from "$app/stores";
	import { authStore } from "$lib/stores/auth";
	import AuthRequiredCard from "$lib/components/AuthRequiredCard.svelte";
	import Icon from "$lib/components/Icons.svelte";
	import Button from "$lib/components/Button.svelte";
	import { signRequest } from "$lib/services/auth-api";
	import {
		getWallet,
		topupWallet,
		formatE9sAsUsd,
		hexEncode,
		type WalletLedgerEntry,
	} from "$lib/services/api";
	import { Ed25519KeyIdentity } from "@dfinity/identity";

	let isAuthenticated = $state(false);
	let unsubscribeAuth: (() => void) | null = null;

	let balanceE9s = $state<number | null>(null);
	let ledger = $state<WalletLedgerEntry[]>([]);
	let loading = $state(true);
	let error = $state("");

	// Top-up form state
	let amountInput = $state("10");
	let submitting = $state(false);
	let formError = $state("");

	// Success/cancel banner driven by query params (?topup=success|cancel).
	const topupStatus = $derived($page.url.searchParams.get("topup"));

	onMount(() => {
		// Subscribe so we re-fire when authStore.initialize() completes (the
		// dashboard layout calls it in its own onMount, which races with ours).
		// Mirrors /dashboard/saved and /dashboard/rentals.
		unsubscribeAuth = authStore.isAuthenticated.subscribe((isAuth) => {
			isAuthenticated = isAuth;
			loadWallet();
		});
	});

	onDestroy(() => {
		unsubscribeAuth?.();
	});

	async function loadWallet() {
		// Distinguish "loading" from "unauthenticated": flip loading off as
		// soon as we know the user is not signed in, so the spinner block
		// (rendered independently of the auth block below) doesn't spin
		// forever for anonymous visitors.
		if (!isAuthenticated) {
			loading = false;
			return;
		}
		const signingIdentityInfo = await authStore.getSigningIdentity();
		if (!signingIdentityInfo || !(signingIdentityInfo.identity instanceof Ed25519KeyIdentity)) {
			loading = false;
			return;
		}
		loading = true;
		error = "";
		try {
			const pubkeyHex = hexEncode(signingIdentityInfo.publicKeyBytes);
			const { headers } = await signRequest(
				signingIdentityInfo.identity,
				"GET",
				`/api/v1/users/${pubkeyHex}/wallet`,
			);
			const wallet = await getWallet(headers, pubkeyHex);
			balanceE9s = wallet.balanceE9s;
			ledger = wallet.recentLedger;
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load wallet";
		} finally {
			loading = false;
		}
	}

	async function handleTopup() {
		const signingIdentityInfo = await authStore.getSigningIdentity();
		if (!signingIdentityInfo || !(signingIdentityInfo.identity instanceof Ed25519KeyIdentity)) return;
		const amount = parseFloat(amountInput);
		if (isNaN(amount) || amount <= 0) {
			formError = "Enter a positive amount";
			return;
		}
		submitting = true;
		formError = "";
		try {
			const pubkeyHex = hexEncode(signingIdentityInfo.publicKeyBytes);
			const { headers } = await signRequest(
				signingIdentityInfo.identity,
				"POST",
				`/api/v1/users/${pubkeyHex}/wallet/topup`,
			);
			const { checkoutUrl } = await topupWallet(headers, pubkeyHex, amount);
			// Redirect to Stripe Checkout; webhook credits the balance on success.
			window.location.href = checkoutUrl;
		} catch (e) {
			formError = e instanceof Error ? e.message : "Failed to start top-up";
			submitting = false;
		}
	}

	const ENTRY_LABELS: Record<string, string> = {
		topup: "Top-up",
		rental_debit: "Rental charge",
		rental_refund: "Refund",
		adjustment: "Adjustment",
	};

	function entryLabel(type: string): string {
		return ENTRY_LABELS[type] ?? type;
	}

	function formatAmount(e9s: number): string {
		const sign = e9s >= 0 ? "+" : "";
		return `${sign}${formatE9sAsUsd(e9s)}`;
	}
</script>

<div class="p-4 md:p-6 max-w-5xl mx-auto">
	<div class="mb-6">
		<h1 class="text-2xl font-bold text-white">Wallet</h1>
		<p class="text-neutral-400 text-sm mt-1">
			Top up your balance to rent cloud resources. Balance is spent on platform services only.
		</p>
	</div>

	{#if !isAuthenticated}
		<AuthRequiredCard />
	{:else if error}
		<div class="card p-4 mb-4 border-l-4 border-error/60 bg-error/10">
			<p class="text-error text-sm">{error}</p>
		</div>
	{/if}

	<!-- Loading and content are a SEPARATE block from the auth/error check above.
	     This is the /dashboard/saved + /dashboard/rentals pattern: the spinner
	     always renders while loading=true, even during the initial window before
	     isAuthenticated settles, so an authed user never sees a bare "Login
	     Required" flash during async identity derivation. -->
	{#if loading}
		<div class="card p-8 text-center">
			<div class="inline-block w-8 h-8 border-2 border-primary-500/30 border-t-primary-500 animate-spin"></div>
			<p class="text-neutral-400 mt-3">Loading wallet…</p>
		</div>
	{:else if !isAuthenticated}
		<!-- AuthRequiredCard already rendered above; nothing here -->
	{:else}
		{#if topupStatus === "success"}
			<div class="card p-4 mb-4 border-l-4 border-success/60 bg-success/10 flex items-center gap-3">
				<Icon name="check" size={20} class="text-success" />
				<p class="text-success text-sm">
					Top-up processed. Your balance has been updated.
				</p>
			</div>
		{:else if topupStatus === "cancel"}
			<div class="card p-4 mb-4 border-l-4 border-warning/60 bg-warning/10 flex items-center gap-3">
				<Icon name="x" size={20} class="text-warning" />
				<p class="text-warning text-sm">Top-up was cancelled. You were not charged.</p>
			</div>
		{/if}

		<!-- Balance + Top-up -->
		<div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
			<div class="card p-6">
				<div class="flex items-center gap-2 mb-3">
					<Icon name="wallet" size={20} class="text-primary-400" />
					<span class="text-neutral-400 text-sm font-medium">Available Balance</span>
				</div>
				<p class="text-3xl font-bold text-white">
					${formatE9sAsUsd(balanceE9s)}
					<span class="text-neutral-500 text-lg font-normal">USD</span>
				</p>
				<p class="text-neutral-500 text-xs mt-2">
					Stored-value credit. Non-withdrawable; refunds go to your original card.
				</p>
			</div>

		<div class="card p-6">
			<h2 class="text-neutral-300 text-sm font-medium mb-3">Add Funds</h2>
			<div class="space-y-3">
				<div>
					<label for="amount" class="block text-neutral-400 text-xs mb-1">Amount (USD)</label>
					<div class="flex items-center gap-2">
						<span class="text-neutral-500">$</span>
						<input
							id="amount"
							type="number"
							min="1"
							step="0.01"
							bind:value={amountInput}
							disabled={submitting}
							onkeydown={(e) => { if (e.key === 'Enter') handleTopup(); }}
							class="input flex-1"
							aria-label="Top-up amount in USD"
						/>
					</div>
				</div>
				{#if formError}
					<p class="text-error text-xs">{formError}</p>
				{/if}
				<Button variant="primary" disabled={submitting} onclick={handleTopup} class="w-full">
					{#if submitting}
						Processing…
					{:else}
						Top Up with Stripe
					{/if}
				</Button>
				<p class="text-neutral-500 text-xs">
					You'll be redirected to Stripe's secure checkout.
				</p>
			</div>
		</div>
		</div>

		<!-- Ledger -->
		<div class="card p-6">
			<h2 class="text-neutral-300 text-sm font-medium mb-4">Recent Transactions</h2>
			{#if ledger.length === 0}
				<p class="text-neutral-500 text-sm py-6 text-center">No transactions yet.</p>
			{:else}
				<div class="overflow-x-auto">
					<table class="w-full text-sm">
						<thead>
							<tr class="text-left text-neutral-500 text-xs border-b border-neutral-800">
								<th class="pb-2 pr-4 font-medium">Type</th>
								<th class="pb-2 pr-4 font-medium text-right">Amount</th>
								<th class="pb-2 pr-4 font-medium text-right">Balance After</th>
								<th class="pb-2 font-medium">Reference</th>
							</tr>
						</thead>
						<tbody>
							{#each ledger as entry (entry.id)}
								<tr class="border-b border-neutral-800/50">
									<td class="py-2 pr-4 text-neutral-300">{entryLabel(entry.entryType)}</td>
									<td class="py-2 pr-4 text-right font-mono {entry.amountE9s >= 0 ? "text-success" : "text-neutral-300"}">
										{formatAmount(entry.amountE9s)}
									</td>
									<td class="py-2 pr-4 text-right font-mono text-neutral-400">
										${formatE9sAsUsd(entry.balanceAfterE9s)}
									</td>
									<td class="py-2 font-mono text-xs text-neutral-500">
										{entry.reference ?? "—"}
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}
		</div>
	{/if}
</div>
