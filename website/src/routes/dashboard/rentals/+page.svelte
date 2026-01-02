<script lang="ts">
	import { onMount, onDestroy, tick } from "svelte";
	import { page } from "$app/stores";
	import { goto } from "$app/navigation";
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
	let highlightedContractId = $state<string | null>(null);

	// Auto-refresh state
	let refreshInterval: ReturnType<typeof setInterval> | null = null;
	let autoRefreshEnabled = $state(true);
	let lastRefresh = $state<number>(Date.now());
	const REFRESH_INTERVAL_MS = 15_000; // 15 seconds

	// Lifecycle stages for progress indicator
	const LIFECYCLE_STAGES = [
		{ key: "payment", label: "Payment", icon: "💳" },
		{ key: "provider", label: "Provider Review", icon: "⏳" },
		{ key: "provisioning", label: "Provisioning", icon: "⚙️" },
		{ key: "ready", label: "Ready", icon: "✅" },
	] as const;

	// Get current stage index for a contract (0-3, or -1 for terminal states)
	function getStageIndex(status: string, paymentStatus?: string): number {
		const s = status.toLowerCase();
		const ps = paymentStatus?.toLowerCase() ?? "";

		if (s === "cancelled" || s === "rejected") return -1;
		if (s === "requested" && ps === "pending") return 0; // awaiting payment
		if (s === "requested" && ps === "failed") return 0; // payment failed
		if (s === "requested" || s === "pending") return 1; // waiting for provider
		if (s === "accepted") return 2; // accepted, waiting for provisioning
		if (s === "provisioning") return 2; // actively provisioning
		if (s === "provisioned" || s === "active") return 3; // ready
		return 1; // default
	}

	// Get "what's next" info for a contract
	function getNextStepInfo(status: string, paymentStatus?: string): { text: string; isWaiting: boolean } | null {
		const s = status.toLowerCase();
		const ps = paymentStatus?.toLowerCase() ?? "";

		if (s === "requested" && ps === "pending") {
			return { text: "Complete payment to proceed", isWaiting: false };
		}
		if (s === "requested" && ps === "failed") {
			return { text: "Payment failed. Please try again or contact support.", isWaiting: false };
		}
		if (s === "requested" && ps === "succeeded") {
			return { text: "Waiting for provider to accept your request (typically within a few hours)", isWaiting: true };
		}
		if (s === "pending") {
			return { text: "Waiting for provider response", isWaiting: true };
		}
		if (s === "accepted") {
			return { text: "Provider accepted! Waiting for provisioning to start...", isWaiting: true };
		}
		if (s === "provisioning") {
			return { text: "Provider is setting up your resource (typically 5-15 minutes)", isWaiting: true };
		}
		if (s === "provisioned" || s === "active") {
			return { text: "Your resource is ready! See connection details below.", isWaiting: false };
		}
		if (s === "rejected") {
			return { text: "Provider rejected this request. You can try another provider.", isWaiting: false };
		}
		if (s === "cancelled") {
			return null;
		}
		return null;
	}

	function startAutoRefresh() {
		stopAutoRefresh();
		if (autoRefreshEnabled && isAuthenticated) {
			refreshInterval = setInterval(() => {
				refreshContracts();
			}, REFRESH_INTERVAL_MS);
		}
	}

	function stopAutoRefresh() {
		if (refreshInterval) {
			clearInterval(refreshInterval);
			refreshInterval = null;
		}
	}

	function toggleAutoRefresh() {
		autoRefreshEnabled = !autoRefreshEnabled;
		if (autoRefreshEnabled) {
			startAutoRefresh();
		} else {
			stopAutoRefresh();
		}
	}

	async function refreshContracts() {
		if (!isAuthenticated || loading) return;
		try {
			const signingIdentityInfo = await authStore.getSigningIdentity();
			if (!signingIdentityInfo) return;

			const { headers } = await signRequest(
				signingIdentityInfo.identity as any,
				"GET",
				`/api/v1/users/${hexEncode(signingIdentityInfo.publicKeyBytes)}/contracts`,
			);

			contracts = await getUserContracts(
				headers,
				hexEncode(signingIdentityInfo.publicKeyBytes),
			);
			lastRefresh = Date.now();
		} catch (e) {
			console.error("Error refreshing contracts:", e);
		}
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
			lastRefresh = Date.now();
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load rentals";
			console.error("Error loading rentals:", e);
		} finally {
			loading = false;
		}
	}

	async function scrollToHighlightedContract() {
		if (!highlightedContractId) return;

		await tick(); // Wait for DOM update
		const element = document.getElementById(`contract-${highlightedContractId}`);
		if (element) {
			element.scrollIntoView({ behavior: "smooth", block: "center" });
		}
	}

	onMount(async () => {
		// Read contract ID from URL params for deep-linking
		highlightedContractId = $page.url.searchParams.get("contract");

		unsubscribeAuth = authStore.isAuthenticated.subscribe(async (isAuth) => {
			isAuthenticated = isAuth;
			await loadContracts();
			scrollToHighlightedContract();
			if (isAuth) {
				startAutoRefresh();
			} else {
				stopAutoRefresh();
			}
		});
	});

	function isCancellable(status: string): boolean {
		return ["requested", "pending", "accepted", "provisioning", "provisioned", "active"].includes(
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

			// Refresh contracts list (sign new request for GET)
			const pubkeyHex = hexEncode(signingIdentityInfo.publicKeyBytes);
			const { headers: getHeaders } = await signRequest(
				signingIdentityInfo.identity as any,
				"GET",
				`/api/v1/users/${pubkeyHex}/contracts`,
			);
			contracts = await getUserContracts(getHeaders, pubkeyHex);
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
		stopAutoRefresh();
	});
</script>

<div class="space-y-8">
	<div class="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-4">
		<div>
			<h1 class="text-4xl font-bold text-white mb-2">My Rentals</h1>
			<p class="text-white/60">
				View and manage your resource rental requests
			</p>
		</div>
		{#if isAuthenticated && contracts.length > 0}
			<div class="flex items-center gap-3">
				<button
					onclick={toggleAutoRefresh}
					class="flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm transition-colors {autoRefreshEnabled ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/30' : 'bg-glass/5 text-white/50 border border-glass/10'}"
					title={autoRefreshEnabled ? 'Auto-refresh enabled (15s)' : 'Auto-refresh disabled'}
				>
					<span class="relative flex h-2 w-2">
						{#if autoRefreshEnabled}
							<span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
						{/if}
						<span class="relative inline-flex rounded-full h-2 w-2 {autoRefreshEnabled ? 'bg-emerald-400' : 'bg-white/30'}"></span>
					</span>
					Auto-refresh
				</button>
				<button
					onclick={refreshContracts}
					class="px-3 py-1.5 rounded-lg text-sm bg-glass/5 text-white/70 border border-glass/10 hover:bg-glass/10 transition-colors"
					title="Refresh now"
				>
					↻ Refresh
				</button>
			</div>
		{/if}
	</div>

	{#if !isAuthenticated}
		<!-- Anonymous user view - login prompt -->
		<div
			class="bg-glass/10 backdrop-blur-lg rounded-xl p-8 border border-glass/15 text-center"
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
						class="px-8 py-3 bg-gradient-to-r from-primary-500 to-primary-600 rounded-lg font-semibold text-white hover:brightness-110 hover:scale-105 transition-all"
					>
						Login / Create Account
					</button>
					<a
						href="/dashboard/marketplace"
						class="px-8 py-3 bg-glass/10 rounded-lg font-semibold text-white hover:bg-glass/15 transition-all"
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
				class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-primary-400"
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
				class="inline-block px-6 py-3 bg-gradient-to-r from-primary-500 to-primary-600 rounded-lg font-semibold hover:brightness-110 transition-all"
			>
				Browse Marketplace
			</a>
		</div>
	{:else}
		<div class="space-y-4">
			{#each contracts as contract}
				{@const statusBadge = getStatusBadge(contract.status, contract.payment_status)}
				{@const isHighlighted = highlightedContractId === contract.contract_id}
				{@const stageIndex = getStageIndex(contract.status, contract.payment_status)}
				{@const nextStep = getNextStepInfo(contract.status, contract.payment_status)}
				<a
					href="/dashboard/rentals/{contract.contract_id}"
					id="contract-{contract.contract_id}"
					class="block bg-glass/10 backdrop-blur-lg rounded-xl p-6 border transition-all cursor-pointer {isHighlighted
						? 'border-primary-400 ring-2 ring-primary-400/50'
						: 'border-glass/15 hover:border-primary-400 hover:bg-white/[0.12]'}"
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
										onclick={(e) => {
											e.preventDefault();
											e.stopPropagation();
											handleCancelContract(contract.contract_id, contract.status);
										}}
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
										onclick={(e) => {
											e.preventDefault();
											e.stopPropagation();
											handleDownloadInvoice(contract.contract_id);
										}}
										class="px-2 py-1 text-xs bg-primary-600/80 text-white rounded hover:bg-primary-700 transition-colors flex items-center gap-1"
										title="Download invoice PDF"
									>
										<span>&#8595;</span>
										Invoice
									</button>
								{/if}
								<!-- Invoice download state -->
								{#if downloadingInvoiceContractId === contract.contract_id}
									<div
										class="flex items-center gap-1 text-xs text-primary-400"
									>
										<div
											class="animate-spin rounded-full h-3 w-3 border-t border-b border-primary-400"
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

					<!-- Progress indicator (only for active rental flows) -->
					{#if stageIndex >= 0}
						<div class="mb-4 p-4 bg-glass/5 rounded-lg border border-glass/10">
							<div class="flex items-center justify-between mb-3">
								{#each LIFECYCLE_STAGES as stage, i}
									<div class="flex flex-col items-center flex-1">
										<div class="flex items-center w-full">
											{#if i > 0}
												<div class="flex-1 h-0.5 {i <= stageIndex ? 'bg-emerald-500' : 'bg-glass/15'}"></div>
											{/if}
											<div
												class="w-8 h-8 rounded-full flex items-center justify-center text-sm border-2 transition-all {
													i < stageIndex
														? 'bg-emerald-500/20 border-emerald-500 text-emerald-400'
														: i === stageIndex
															? 'bg-primary-500/20 border-primary-500 text-primary-400 ring-2 ring-primary-500/30'
															: 'bg-glass/5 border-glass/15 text-white/40'
												}"
											>
												{stage.icon}
											</div>
											{#if i < LIFECYCLE_STAGES.length - 1}
												<div class="flex-1 h-0.5 {i < stageIndex ? 'bg-emerald-500' : 'bg-glass/15'}"></div>
											{/if}
										</div>
										<span class="text-xs mt-1 {i <= stageIndex ? 'text-white/80' : 'text-white/40'}">{stage.label}</span>
									</div>
								{/each}
							</div>
							{#if nextStep}
								<div class="flex items-start gap-2 text-sm {nextStep.isWaiting ? 'text-primary-400' : 'text-white/70'}">
									{#if nextStep.isWaiting}
										<div class="animate-pulse mt-0.5">⏳</div>
									{:else}
										<span class="mt-0.5">→</span>
									{/if}
									<div>
										<span>{nextStep.text}</span>
										{#if nextStep.isWaiting}
											<p class="text-white/50 text-xs mt-1">
												You'll receive an email when your resource is ready. Make sure your <button onclick={(e) => { e.preventDefault(); e.stopPropagation(); goto('/dashboard/account/profile'); }} class="text-primary-400 hover:underline">profile</button> has a valid email address.
											</p>
										{/if}
									</div>
								</div>
							{/if}
						</div>
					{/if}

					<div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
						<div
							class="bg-glass/5 rounded-lg p-3 border border-glass/10"
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
								class="bg-glass/5 rounded-lg p-3 border border-glass/10"
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
								class="bg-glass/5 rounded-lg p-3 border border-glass/10"
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
							class="bg-glass/5 rounded-lg p-3 border border-glass/10"
						>
							<div class="text-white/60 text-xs mb-1">
								Provider
							</div>
							<button
								onclick={(e) => {
									e.preventDefault();
									e.stopPropagation();
									goto(`/dashboard/reputation/${contract.provider_pubkey}`);
								}}
								class="text-white text-sm font-mono hover:text-primary-400 transition-colors text-left"
							>
								{truncateHash(contract.provider_pubkey)}
							</button>
						</div>
					</div>

					{#if contract.request_memo}
						<div
							class="bg-glass/5 rounded-lg p-3 border border-glass/10 mb-4"
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
				</a>
			{/each}
		</div>
	{/if}
</div>
