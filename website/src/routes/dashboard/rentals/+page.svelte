<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { page } from "$app/stores";
	import { navigateToLogin } from "$lib/utils/navigation";
	import {
		getUserContracts,
		cancelRentalRequest,
		downloadContractInvoice,
		type Contract,
		hexEncode,
	} from "$lib/services/api";
	import { getContractStatusBadge as getStatusBadge } from "$lib/utils/contract-status";
	import {
		formatContractDate as formatDate,
		formatContractPrice as formatPrice,
		truncateContractHash as truncateHash,
	} from "$lib/utils/contract-format";
	import { authStore } from "$lib/stores/auth";
	import { signRequest } from "$lib/services/auth-api";

	let contracts = $state<Contract[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let cancellingContractId = $state<string | null>(null);
	let downloadingInvoiceContractId = $state<string | null>(null);
	let isAuthenticated = $state(false);
	let unsubscribeAuth: (() => void) | null = null;

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
				error = "You must be authenticated to view rentals";
				return;
			}

			const { headers } = await signRequest(
				signingIdentityInfo.identity as any,
				"GET",
				`/api/v1/users/${hexEncode(signingIdentityInfo.publicKeyBytes)}/contracts`,
			);

			contracts = await getUserContracts(
				headers,
				hexEncode(signingIdentityInfo.publicKeyBytes),
			);
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load rentals";
			console.error("Error loading rentals:", e);
		} finally {
			loading = false;
		}
	}

	onMount(async () => {
		unsubscribeAuth = authStore.isAuthenticated.subscribe((isAuth) => {
			isAuthenticated = isAuth;
			loadContracts();
		});
	});

	function isCancellable(status: string): boolean {
		return ["requested", "pending", "accepted", "provisioning"].includes(
			status.toLowerCase(),
		);
	}

	async function handleCancelContract(
		contractId: string,
		contractStatus: string,
	) {
		if (!isCancellable(contractStatus)) {
			return;
		}

		if (!confirm("Are you sure you want to cancel this rental request?")) {
			return;
		}

		try {
			cancellingContractId = contractId;
			const signingIdentityInfo = await authStore.getSigningIdentity();
			if (!signingIdentityInfo) {
				error = "You must be authenticated to cancel rental requests";
				return;
			}

			const { headers } = await signRequest(
				signingIdentityInfo.identity as any,
				"PUT",
				`/api/v1/contracts/${contractId}/cancel`,
				{ memo: "Cancelled by user" },
			);

			await cancelRentalRequest(
				contractId,
				{ memo: "Cancelled by user" },
				headers,
			);

			// Refresh contracts list
			contracts = await getUserContracts(
				headers,
				hexEncode(signingIdentityInfo.publicKeyBytes),
			);
		} catch (e) {
			error =
				e instanceof Error
					? e.message
					: "Failed to cancel rental request";
			console.error("Error cancelling rental request:", e);
		} finally {
			cancellingContractId = null;
		}
	}

	async function handleDownloadInvoice(contractId: string) {
		try {
			downloadingInvoiceContractId = contractId;
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
			downloadingInvoiceContractId = null;
		}
	}

	function handleLogin() {
		navigateToLogin($page.url.pathname);
	}

	onDestroy(() => {
		unsubscribeAuth?.();
	});
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">My Rentals</h1>
		<p class="text-white/60">
			View and manage your resource rental requests
		</p>
	</div>

	{#if !isAuthenticated}
		<!-- Anonymous user view - login prompt -->
		<div
			class="bg-white/10 backdrop-blur-lg rounded-xl p-8 border border-white/20 text-center"
		>
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">🔑</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-white/70">
					Create an account or login to view and manage your rental
					contracts. See the marketplace to browse available
					resources.
				</p>
				<div class="flex flex-col gap-3">
					<button
						onclick={handleLogin}
						class="px-8 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 hover:scale-105 transition-all"
					>
						Login / Create Account
					</button>
					<a
						href="/dashboard/marketplace"
						class="px-8 py-3 bg-white/10 rounded-lg font-semibold text-white hover:bg-white/20 transition-all"
					>
						Browse Marketplace
					</a>
				</div>
			</div>
		</div>
	{:else if error}
		<div
			class="bg-red-500/20 border border-red-500/30 rounded-lg p-4 text-red-400"
		>
			<p class="font-semibold">Error loading rentals</p>
			<p class="text-sm mt-1">{error}</p>
		</div>
	{/if}

	{#if loading}
		<div class="flex justify-center items-center p-8">
			<div
				class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-400"
			></div>
		</div>
	{:else if contracts.length === 0}
		<div class="text-center py-16">
			<span class="text-6xl mb-4 block">📋</span>
			<h3 class="text-2xl font-bold text-white mb-2">No Rentals Yet</h3>
			<p class="text-white/60 mb-6">
				You haven't created any rental requests yet
			</p>
			<a
				href="/dashboard/marketplace"
				class="inline-block px-6 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold hover:brightness-110 transition-all"
			>
				Browse Marketplace
			</a>
		</div>
	{:else}
		<div class="space-y-4">
			{#each contracts as contract}
				{@const statusBadge = getStatusBadge(contract.status, contract.payment_status)}
				<div
					class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20 hover:border-blue-400 transition-all"
				>
					<div class="flex items-start justify-between mb-4">
						<div class="flex-1">
							<div class="flex items-center gap-3 mb-2">
								<h3 class="text-xl font-bold text-white">
									{contract.offering_id}
								</h3>
								<span
									class="inline-flex items-center gap-1 px-3 py-1 rounded-full text-xs font-medium border {statusBadge.class}"
								>
									<span>{statusBadge.icon}</span>
									{statusBadge.text}
								</span>
								<!-- Cancel button for cancelable contracts -->
								{#if isCancellable(contract.status) && cancellingContractId !== contract.contract_id}
									<button
										onclick={() =>
											handleCancelContract(
												contract.contract_id,
												contract.status,
											)}
										class="px-2 py-1 text-xs bg-red-600/80 text-white rounded hover:bg-red-700 transition-colors"
										title="Cancel this rental request"
									>
										Cancel
									</button>
								{/if}
								<!-- Download Invoice button for paid contracts -->
								<!-- Show for: payment succeeded/refunded OR contract progressed past payment (active/provisioned/provisioning/accepted) -->
								{#if (contract.payment_status === "succeeded" || contract.payment_status === "refunded" || ["active", "provisioned", "provisioning", "accepted"].includes(contract.status.toLowerCase())) && downloadingInvoiceContractId !== contract.contract_id}
									<button
										onclick={() =>
											handleDownloadInvoice(
												contract.contract_id,
											)}
										class="px-2 py-1 text-xs bg-blue-600/80 text-white rounded hover:bg-blue-700 transition-colors flex items-center gap-1"
										title="Download invoice PDF"
									>
										<span>&#8595;</span>
										Invoice
									</button>
								{/if}
								<!-- Invoice download state -->
								{#if downloadingInvoiceContractId === contract.contract_id}
									<div
										class="flex items-center gap-1 text-xs text-blue-400"
									>
										<div
											class="animate-spin rounded-full h-3 w-3 border-t border-b border-blue-400"
										></div>
										Downloading...
									</div>
								{/if}
								<!-- Cancellation state -->
								{#if cancellingContractId === contract.contract_id}
									<div
										class="flex items-center gap-1 text-xs text-red-400"
									>
										<div
											class="animate-spin rounded-full h-3 w-3 border-t border-b border-red-400"
										></div>
										Cancelling...
									</div>
								{/if}
							</div>
							<p class="text-white/60 text-sm">
								Contract ID: {truncateHash(
									contract.contract_id,
								)}
							</p>
						</div>
						<div class="text-right">
							<div class="text-2xl font-bold text-white">
								{formatPrice(
									contract.payment_amount_e9s,
									contract.currency,
								)}
							</div>
							{#if contract.duration_hours}
								<div class="text-white/60 text-sm">
									{contract.duration_hours} hours
								</div>
							{/if}
						</div>
					</div>

					<div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
						<div
							class="bg-white/5 rounded-lg p-3 border border-white/10"
						>
							<div class="text-white/60 text-xs mb-1">
								Created
							</div>
							<div class="text-white text-sm">
								{formatDate(contract.created_at_ns)}
							</div>
						</div>
						{#if contract.region_name}
							<div
								class="bg-white/5 rounded-lg p-3 border border-white/10"
							>
								<div class="text-white/60 text-xs mb-1">
									Region
								</div>
								<div class="text-white text-sm">
									{contract.region_name}
								</div>
							</div>
						{/if}
						{#if contract.requester_ssh_pubkey}
							<div
								class="bg-white/5 rounded-lg p-3 border border-white/10"
							>
								<div class="text-white/60 text-xs mb-1">
									SSH Key
								</div>
								<div
									class="text-white text-sm font-mono truncate"
								>
									{truncateHash(
										contract.requester_ssh_pubkey,
									)}
								</div>
							</div>
						{/if}
						<div
							class="bg-white/5 rounded-lg p-3 border border-white/10"
						>
							<div class="text-white/60 text-xs mb-1">
								Provider
							</div>
							<a
								href="/dashboard/reputation/{contract.provider_pubkey}"
								class="text-white text-sm font-mono hover:text-blue-400 transition-colors"
							>
								{truncateHash(contract.provider_pubkey)}
							</a>
						</div>
					</div>

					{#if contract.request_memo}
						<div
							class="bg-white/5 rounded-lg p-3 border border-white/10 mb-4"
						>
							<div class="text-white/60 text-xs mb-1">Memo</div>
							<div class="text-white text-sm">
								{contract.request_memo}
							</div>
						</div>
					{/if}

					{#if contract.provisioning_instance_details}
						<div
							class="bg-green-500/10 border border-green-500/30 rounded-lg p-4"
						>
							<div class="text-green-400 font-semibold mb-2">
								Instance Details
							</div>
							<div class="text-white text-sm whitespace-pre-wrap">
								{contract.provisioning_instance_details}
							</div>
							{#if contract.provisioning_completed_at_ns}
								<div class="text-green-400/60 text-xs mt-2">
									Provisioned: {formatDate(
										contract.provisioning_completed_at_ns,
									)}
								</div>
							{/if}
						</div>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>
