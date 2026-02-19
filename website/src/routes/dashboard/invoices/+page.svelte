<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { page } from "$app/stores";
	import { navigateToLogin } from "$lib/utils/navigation";
	import {
		getUserContracts,
		downloadContractInvoice,
		type Contract,
		hexEncode,
	} from "$lib/services/api";
	import {
		formatContractDate as formatDate,
		formatContractPrice as formatPrice,
		truncateContractHash as truncateHash,
	} from "$lib/utils/contract-format";
	import { getContractStatusBadge } from "$lib/utils/contract-status";
	import { authStore } from "$lib/stores/auth";
	import { signRequest } from "$lib/services/auth-api";

	let contracts = $state<Contract[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let downloadingContractId = $state<string | null>(null);
	let isAuthenticated = $state(false);
	let unsubscribeAuth: (() => void) | null = null;

	/** Contracts that have a downloadable invoice */
	let invoiceContracts = $derived(
		contracts
			.filter(
				(c) =>
					c.payment_status === "succeeded" ||
					c.payment_status === "refunded" ||
					["active", "provisioned", "provisioning", "accepted"].includes(
						c.status.toLowerCase(),
					),
			)
			.sort((a, b) => b.created_at_ns - a.created_at_ns),
	);

	function formatAmount(e9s: number, currency: string): string {
		return formatPrice(e9s, currency);
	}

	async function loadContracts() {
		if (!isAuthenticated) {
			loading = false;
			return;
		}

		try {
			loading = true;
			error = null;

			const signingIdentityInfo = await authStore.getSigningIdentity();
			if (!signingIdentityInfo) {
				error = "You must be authenticated to view invoices";
				return;
			}

			const pubkeyHex = hexEncode(signingIdentityInfo.publicKeyBytes);
			const { headers } = await signRequest(
				signingIdentityInfo.identity as any,
				"GET",
				`/api/v1/users/${pubkeyHex}/contracts`,
			);

			contracts = await getUserContracts(headers, pubkeyHex);
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load invoices";
			console.error("Error loading invoices:", e);
		} finally {
			loading = false;
		}
	}

	async function handleDownloadInvoice(contractId: string) {
		try {
			downloadingContractId = contractId;
			const signingIdentityInfo = await authStore.getSigningIdentity();
			if (!signingIdentityInfo) {
				error = "You must be authenticated to download invoices";
				return;
			}

			const { headers } = await signRequest(
				signingIdentityInfo.identity as any,
				"GET",
				`/api/v1/contracts/${contractId}/invoice`,
			);

			await downloadContractInvoice(contractId, headers);
		} catch (e) {
			error =
				e instanceof Error ? e.message : "Failed to download invoice";
			console.error("Error downloading invoice:", e);
		} finally {
			downloadingContractId = null;
		}
	}

	function handleLogin() {
		navigateToLogin($page.url.pathname);
	}

	onMount(() => {
		unsubscribeAuth = authStore.isAuthenticated.subscribe((isAuth) => {
			isAuthenticated = isAuth;
			loadContracts();
		});
	});

	onDestroy(() => {
		unsubscribeAuth?.();
	});
</script>

<div class="space-y-8">
	<header>
		<h1 class="text-2xl font-bold text-white tracking-tight">Invoices</h1>
		<p class="text-neutral-500">Download invoices for your rental contracts</p>
	</header>

	{#if !isAuthenticated}
		<div class="card p-8 border border-neutral-800 text-center">
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">🧾</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-neutral-400">
					Create an account or login to view and download your invoices.
				</p>
				<button
					onclick={handleLogin}
					class="px-8 py-3 bg-gradient-to-r from-primary-500 to-primary-600 font-semibold text-white hover:brightness-110 hover:scale-105 transition-all"
				>
					Login / Create Account
				</button>
			</div>
		</div>
	{:else if error}
		<div class="bg-red-500/20 border border-red-500/30 p-4 text-red-400">
			<p class="font-semibold">Error loading invoices</p>
			<p class="text-sm mt-1">{error}</p>
		</div>
	{/if}

	{#if loading}
		<div class="flex justify-center items-center p-8">
			<div class="animate-spin h-12 w-12 border-t-2 border-b-2 border-primary-400"></div>
		</div>
	{:else if isAuthenticated && invoiceContracts.length === 0}
		<div class="text-center py-16">
			<span class="text-6xl mb-4 block">🧾</span>
			<h3 class="text-2xl font-bold text-white mb-2">No Invoices Yet</h3>
			<p class="text-neutral-500 mb-6">
				Invoices will appear here once you have paid rental contracts.
			</p>
			<a
				href="/dashboard/marketplace"
				class="inline-block px-6 py-3 bg-gradient-to-r from-primary-500 to-primary-600 font-semibold hover:brightness-110 transition-all"
			>
				Browse Marketplace
			</a>
		</div>
	{:else if isAuthenticated}
		<!-- Invoice table -->
		<div class="bg-surface-elevated border border-neutral-800/80 overflow-hidden">
			<div class="overflow-x-auto">
				<table class="w-full text-sm">
					<thead>
						<tr class="border-b border-neutral-800/80 text-neutral-500 text-xs uppercase tracking-wider">
							<th class="text-left px-4 py-3 font-medium">Date</th>
							<th class="text-left px-4 py-3 font-medium">Contract</th>
							<th class="text-left px-4 py-3 font-medium">Provider</th>
							<th class="text-right px-4 py-3 font-medium">Amount</th>
							<th class="text-left px-4 py-3 font-medium">Status</th>
							<th class="text-right px-4 py-3 font-medium">Invoice</th>
						</tr>
					</thead>
					<tbody>
						{#each invoiceContracts as contract}
							{@const statusBadge = getContractStatusBadge(contract.status, contract.payment_status)}
							{@const isDownloading = downloadingContractId === contract.contract_id}
							<tr class="border-b border-neutral-800/40 hover:bg-white/[0.03] transition-colors">
								<td class="px-4 py-3 text-neutral-300 whitespace-nowrap">
									{formatDate(contract.created_at_ns)}
								</td>
								<td class="px-4 py-3">
									<a
										href="/dashboard/rentals/{contract.contract_id}"
										class="text-primary-400 hover:text-primary-300 font-mono text-xs transition-colors"
									>
										{truncateHash(contract.contract_id, 8)}
									</a>
								</td>
								<td class="px-4 py-3 text-neutral-400 font-mono text-xs">
									{truncateHash(contract.provider_pubkey)}
								</td>
								<td class="px-4 py-3 text-white text-right font-medium whitespace-nowrap">
									{formatAmount(contract.payment_amount_e9s, contract.currency)}
								</td>
								<td class="px-4 py-3">
									<span
										class="inline-flex items-center gap-1 px-2 py-0.5 text-xs font-medium border {statusBadge.class}"
									>
										<span>{statusBadge.icon}</span>
										{statusBadge.text}
									</span>
								</td>
								<td class="px-4 py-3 text-right">
									{#if isDownloading}
										<div class="inline-flex items-center gap-1 text-xs text-primary-400">
											<div class="animate-spin h-3 w-3 border-t border-b border-primary-400"></div>
											Downloading...
										</div>
									{:else}
										<button
											onclick={() => handleDownloadInvoice(contract.contract_id)}
											class="px-2 py-1 text-xs bg-primary-600/80 text-white hover:bg-primary-700 transition-colors inline-flex items-center gap-1"
											title="Download invoice PDF"
										>
											<span>&#8595;</span>
											PDF
										</button>
									{/if}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</div>
	{/if}
</div>
