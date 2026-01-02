<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { page } from "$app/stores";
	import { goto } from "$app/navigation";
	import { navigateToLogin } from "$lib/utils/navigation";
	import {
		getUserContracts,
		cancelRentalRequest,
		downloadContractInvoice,
		getContractUsage,
		type Contract,
		type ContractUsage,
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

	const contractId = $page.params.contract_id ?? "";

	let contract = $state<Contract | null>(null);
	let usage = $state<ContractUsage | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let cancelling = $state(false);
	let downloadingInvoice = $state(false);
	let isAuthenticated = $state(false);
	let unsubscribeAuth: (() => void) | null = null;

	// Auto-refresh state
	let refreshInterval: ReturnType<typeof setInterval> | null = null;
	let autoRefreshEnabled = $state(true);
	let lastRefresh = $state<number>(Date.now());
	const REFRESH_INTERVAL_MS = 15_000;

	// Lifecycle stages for progress indicator
	const LIFECYCLE_STAGES = [
		{ key: "payment", label: "Payment", icon: "💳" },
		{ key: "provider", label: "Provider Review", icon: "⏳" },
		{ key: "provisioning", label: "Provisioning", icon: "⚙️" },
		{ key: "ready", label: "Ready", icon: "✅" },
	] as const;

	function getStageIndex(status: string, paymentStatus?: string): number {
		const s = status.toLowerCase();
		const ps = paymentStatus?.toLowerCase() ?? "";

		if (s === "cancelled" || s === "rejected") return -1;
		if (s === "requested" && ps === "pending") return 0;
		if (s === "requested" && ps === "failed") return 0;
		if (s === "requested" || s === "pending") return 1;
		if (s === "accepted") return 2;
		if (s === "provisioning") return 2;
		if (s === "provisioned" || s === "active") return 3;
		return 1;
	}

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
				refreshContract();
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

	async function refreshContract() {
		if (!isAuthenticated || loading) return;
		try {
			const signingIdentityInfo = await authStore.getSigningIdentity();
			if (!signingIdentityInfo) return;

			const { headers } = await signRequest(
				signingIdentityInfo.identity as any,
				"GET",
				`/api/v1/users/${hexEncode(signingIdentityInfo.publicKeyBytes)}/contracts`,
			);

			const contracts = await getUserContracts(
				headers,
				hexEncode(signingIdentityInfo.publicKeyBytes),
			);
			contract = contracts.find((c) => c.contract_id === contractId) ?? null;

			// Refresh usage data
			if (contract) {
				try {
					usage = await getContractUsage(contractId, headers);
				} catch (e) {
					console.debug("No usage data for contract:", e);
				}
			}
			lastRefresh = Date.now();
		} catch (e) {
			console.error("Error refreshing contract:", e);
		}
	}

	async function loadContract() {
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

			const contracts = await getUserContracts(
				headers,
				hexEncode(signingIdentityInfo.publicKeyBytes),
			);
			contract = contracts.find((c) => c.contract_id === contractId) ?? null;

			if (!contract) {
				error = "Contract not found";
			} else {
				// Try to fetch usage data (may not exist for all contracts)
				try {
					usage = await getContractUsage(contractId, headers);
				} catch (e) {
					// Usage not available is not an error
					console.debug("No usage data for contract:", e);
				}
			}
			lastRefresh = Date.now();
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load contract";
			console.error("Error loading contract:", e);
		} finally {
			loading = false;
		}
	}

	onMount(async () => {
		unsubscribeAuth = authStore.isAuthenticated.subscribe(async (isAuth) => {
			isAuthenticated = isAuth;
			await loadContract();
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

	async function handleCancelContract() {
		if (!contract || !isCancellable(contract.status)) return;

		if (!confirm("Are you sure you want to cancel this rental request?")) {
			return;
		}

		try {
			cancelling = true;
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

			await refreshContract();
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to cancel rental request";
			console.error("Error cancelling rental request:", e);
		} finally {
			cancelling = false;
		}
	}

	async function handleDownloadInvoice() {
		if (!contract) return;

		try {
			downloadingInvoice = true;
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
			error = e instanceof Error ? e.message : "Failed to download invoice";
			console.error("Error downloading invoice:", e);
		} finally {
			downloadingInvoice = false;
		}
	}

	function handleLogin() {
		navigateToLogin($page.url.pathname);
	}

	function copyLink() {
		navigator.clipboard.writeText(window.location.href);
	}

	onDestroy(() => {
		unsubscribeAuth?.();
		stopAutoRefresh();
	});
</script>

<div class="space-y-8">
	<!-- Breadcrumb -->
	<nav class="text-sm text-white/60">
		<a href="/dashboard/rentals" class="hover:text-white transition-colors">My Rentals</a>
		<span class="mx-2">/</span>
		<span class="text-white">{truncateHash(contractId)}</span>
	</nav>

	{#if !isAuthenticated}
		<div class="bg-glass/10 backdrop-blur-lg rounded-xl p-8 border border-glass/15 text-center">
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">🔑</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-white/70">
					Create an account or login to view contract details.
				</p>
				<button
					onclick={handleLogin}
					class="px-8 py-3 bg-gradient-to-r from-primary-500 to-primary-600 rounded-lg font-semibold text-white hover:brightness-110 hover:scale-105 transition-all"
				>
					Login / Create Account
				</button>
			</div>
		</div>
	{:else if loading}
		<div class="flex justify-center items-center p-8">
			<div class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-primary-400"></div>
		</div>
	{:else if error && !contract}
		<div class="bg-red-500/20 border border-red-500/30 rounded-lg p-6 text-center">
			<span class="text-6xl mb-4 block">🔍</span>
			<h2 class="text-2xl font-bold text-red-400 mb-2">Contract Not Found</h2>
			<p class="text-white/70 mb-4">{error}</p>
			<a
				href="/dashboard/rentals"
				class="inline-block px-6 py-3 bg-glass/10 rounded-lg font-semibold hover:bg-glass/15 transition-all"
			>
				← Back to My Rentals
			</a>
		</div>
	{:else if contract}
		{@const statusBadge = getStatusBadge(contract.status, contract.payment_status)}
		{@const stageIndex = getStageIndex(contract.status, contract.payment_status)}
		{@const nextStep = getNextStepInfo(contract.status, contract.payment_status)}

		<!-- Header with actions -->
		<div class="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-4">
			<div>
				<h1 class="text-4xl font-bold text-white mb-2">{contract.offering_id}</h1>
				<p class="text-white/60 font-mono text-sm">{contract.contract_id}</p>
			</div>
			<div class="flex items-center gap-3">
				<button
					onclick={copyLink}
					class="px-3 py-1.5 rounded-lg text-sm bg-glass/5 text-white/70 border border-glass/10 hover:bg-glass/10 transition-colors"
					title="Copy link to this contract"
				>
					🔗 Copy Link
				</button>
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
					onclick={refreshContract}
					class="px-3 py-1.5 rounded-lg text-sm bg-glass/5 text-white/70 border border-glass/10 hover:bg-glass/10 transition-colors"
					title="Refresh now"
				>
					↻ Refresh
				</button>
			</div>
		</div>

		{#if error}
			<div class="bg-red-500/20 border border-red-500/30 rounded-lg p-4 text-red-400">
				<p class="font-semibold">Error</p>
				<p class="text-sm mt-1">{error}</p>
			</div>
		{/if}

		<!-- Contract card -->
		<div class="bg-glass/10 backdrop-blur-lg rounded-xl p-6 border border-glass/15">
			<div class="flex items-start justify-between mb-4">
				<div class="flex-1">
					<div class="flex items-center gap-3 mb-2">
						<span
							class="inline-flex items-center gap-1 px-3 py-1 rounded-full text-xs font-medium border {statusBadge.class}"
						>
							<span>{statusBadge.icon}</span>
							{statusBadge.text}
						</span>
						{#if isCancellable(contract.status) && !cancelling}
							<button
								onclick={handleCancelContract}
								class="px-2 py-1 text-xs bg-red-600/80 text-white rounded hover:bg-red-700 transition-colors"
								title="Cancel this rental request"
							>
								Cancel
							</button>
						{/if}
						{#if (contract.payment_status === "succeeded" || contract.payment_status === "refunded" || ["active", "provisioned", "provisioning", "accepted"].includes(contract.status.toLowerCase())) && !downloadingInvoice}
							<button
								onclick={handleDownloadInvoice}
								class="px-2 py-1 text-xs bg-primary-600/80 text-white rounded hover:bg-primary-700 transition-colors flex items-center gap-1"
								title="Download invoice PDF"
							>
								<span>↓</span>
								Invoice
							</button>
						{/if}
						{#if downloadingInvoice}
							<div class="flex items-center gap-1 text-xs text-primary-400">
								<div class="animate-spin rounded-full h-3 w-3 border-t border-b border-primary-400"></div>
								Downloading...
							</div>
						{/if}
						{#if cancelling}
							<div class="flex items-center gap-1 text-xs text-red-400">
								<div class="animate-spin rounded-full h-3 w-3 border-t border-b border-red-400"></div>
								Cancelling...
							</div>
						{/if}
					</div>
				</div>
				<div class="text-right">
					<div class="text-2xl font-bold text-white">
						{formatPrice(contract.payment_amount_e9s, contract.currency)}
					</div>
					{#if contract.stripe_subscription_id}
						<div class="text-purple-400 text-sm flex items-center gap-1 justify-end">
							<span class="text-xs">↻</span> Subscription
						</div>
					{:else if contract.duration_hours}
						<div class="text-white/60 text-sm">{contract.duration_hours} hours (one-time)</div>
					{/if}
				</div>
			</div>

			<!-- Progress indicator -->
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
										You'll receive an email when your resource is ready. Make sure your <a href="/dashboard/account/profile" class="text-primary-400 hover:underline">profile</a> has a valid email address.
									</p>
								{/if}
							</div>
						</div>
					{/if}
				</div>
			{/if}

			<!-- Contract details grid -->
			<div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
				<div class="bg-glass/5 rounded-lg p-3 border border-glass/10">
					<div class="text-white/60 text-xs mb-1">Created</div>
					<div class="text-white text-sm">{formatDate(contract.created_at_ns)}</div>
				</div>
				{#if contract.end_timestamp_ns}
					{@const endDate = new Date(contract.end_timestamp_ns / 1_000_000)}
					{@const isExpired = endDate < new Date()}
					<div class="bg-glass/5 rounded-lg p-3 border {isExpired ? 'border-red-500/30' : 'border-glass/10'}">
						<div class="text-white/60 text-xs mb-1">{isExpired ? 'Expired' : 'Expires'}</div>
						<div class="text-sm {isExpired ? 'text-red-400' : 'text-white'}">{endDate.toLocaleString()}</div>
					</div>
				{/if}
				{#if contract.region_name}
					<div class="bg-glass/5 rounded-lg p-3 border border-glass/10">
						<div class="text-white/60 text-xs mb-1">Region</div>
						<div class="text-white text-sm">{contract.region_name}</div>
					</div>
				{/if}
				{#if contract.requester_ssh_pubkey}
					<div class="bg-glass/5 rounded-lg p-3 border border-glass/10">
						<div class="text-white/60 text-xs mb-1">SSH Key</div>
						<div class="text-white text-sm font-mono truncate">
							{truncateHash(contract.requester_ssh_pubkey)}
						</div>
					</div>
				{/if}
				<div class="bg-glass/5 rounded-lg p-3 border border-glass/10">
					<div class="text-white/60 text-xs mb-1">Provider</div>
					<a
						href="/dashboard/reputation/{contract.provider_pubkey}"
						class="text-white text-sm font-mono hover:text-primary-400 transition-colors"
					>
						{truncateHash(contract.provider_pubkey)}
					</a>
				</div>
			</div>

			{#if contract.request_memo}
				<div class="bg-glass/5 rounded-lg p-3 border border-glass/10 mb-4">
					<div class="text-white/60 text-xs mb-1">Memo</div>
					<div class="text-white text-sm">{contract.request_memo}</div>
				</div>
			{/if}

			{#if contract.provisioning_instance_details}
				{@const instanceDetails = (() => {
					try { return JSON.parse(contract.provisioning_instance_details); } catch { return null; }
				})()}
				<div class="bg-green-500/10 border border-green-500/30 rounded-lg p-4">
					<div class="text-green-400 font-semibold mb-3">Connection Details</div>

					{#if contract.gateway_slug && contract.gateway_ssh_port}
						<!-- Gateway-accessible VM -->
						<div class="space-y-3">
							<div class="bg-black/20 rounded-lg p-3">
								<div class="text-white/60 text-xs mb-1">SSH Command</div>
								<code class="text-green-300 text-sm font-mono break-all select-all">
									ssh -p {contract.gateway_ssh_port} root@{instanceDetails?.gateway_subdomain || `${contract.gateway_slug}.decent-cloud.org`}
								</code>
							</div>
							{#if instanceDetails?.gateway_subdomain}
								<div class="bg-black/20 rounded-lg p-3">
									<div class="text-white/60 text-xs mb-1">Host</div>
									<code class="text-white text-sm font-mono select-all">{instanceDetails.gateway_subdomain}</code>
								</div>
							{/if}
							{#if contract.gateway_port_range_start && contract.gateway_port_range_end}
								<div class="bg-black/20 rounded-lg p-3">
									<div class="text-white/60 text-xs mb-1">Available Ports</div>
									<code class="text-white text-sm font-mono">{contract.gateway_port_range_start} - {contract.gateway_port_range_end}</code>
									<div class="text-white/40 text-xs mt-1">Use these ports for custom services</div>
								</div>
							{/if}
						</div>
					{:else if instanceDetails?.ip_address}
						<!-- Direct IP access VM -->
						<div class="space-y-3">
							<div class="bg-black/20 rounded-lg p-3">
								<div class="text-white/60 text-xs mb-1">SSH Command</div>
								<code class="text-green-300 text-sm font-mono break-all select-all">
									ssh root@{instanceDetails.ip_address}
								</code>
							</div>
							<div class="bg-black/20 rounded-lg p-3">
								<div class="text-white/60 text-xs mb-1">IP Address</div>
								<code class="text-white text-sm font-mono select-all">{instanceDetails.ip_address}</code>
							</div>
							{#if instanceDetails.ipv6_address}
								<div class="bg-black/20 rounded-lg p-3">
									<div class="text-white/60 text-xs mb-1">IPv6 Address</div>
									<code class="text-white text-sm font-mono select-all">{instanceDetails.ipv6_address}</code>
								</div>
							{/if}
						</div>
					{:else}
						<!-- Raw JSON fallback -->
						<div class="text-white text-sm whitespace-pre-wrap font-mono">
							{contract.provisioning_instance_details}
						</div>
					{/if}

					{#if contract.provisioning_completed_at_ns}
						<div class="text-green-400/60 text-xs mt-3">
							Provisioned: {formatDate(contract.provisioning_completed_at_ns)}
						</div>
					{/if}
				</div>
			{/if}

			<!-- Subscription information (shown for subscription-based contracts) -->
			{#if contract.stripe_subscription_id}
				{@const isActive = contract.subscription_status === 'active' || contract.subscription_status === 'trialing'}
				{@const renewalDate = contract.current_period_end_ns ? new Date(contract.current_period_end_ns / 1_000_000) : null}
				<div class="bg-purple-500/10 border border-purple-500/30 rounded-lg p-4 mt-4">
					<div class="flex items-center justify-between mb-2">
						<div class="text-purple-400 font-semibold">Subscription</div>
						<span class="px-2 py-0.5 rounded text-xs font-medium {
							contract.subscription_status === 'active' ? 'bg-green-500/20 text-green-400' :
							contract.subscription_status === 'trialing' ? 'bg-primary-500/20 text-primary-400' :
							contract.subscription_status === 'past_due' ? 'bg-amber-500/20 text-amber-400' :
							contract.subscription_status === 'cancelled' ? 'bg-red-500/20 text-red-400' :
							'bg-glass/10 text-white/60'
						}">
							{contract.subscription_status || 'Unknown'}
						</span>
					</div>
					<div class="grid grid-cols-1 sm:grid-cols-2 gap-3 text-sm">
						{#if renewalDate}
							<div>
								<span class="text-white/60">{contract.cancel_at_period_end ? 'Ends on:' : 'Renews on:'}</span>
								<span class="text-white ml-2">{renewalDate.toLocaleDateString()}</span>
							</div>
						{/if}
						{#if contract.cancel_at_period_end}
							<div class="col-span-full">
								<span class="text-amber-400 text-sm">Subscription will not renew after the current period.</span>
							</div>
						{/if}
					</div>
					{#if isActive && !contract.cancel_at_period_end}
						<p class="text-purple-400/70 text-xs mt-3">
							Your subscription will automatically renew. To cancel, use the Cancel button above.
						</p>
					{/if}
				</div>
			{/if}

			<!-- Refund information (shown when cancelled/refunded) -->
			{#if contract.payment_status === "refunded" || contract.refund_amount_e9s}
				<div class="bg-amber-500/10 border border-amber-500/30 rounded-lg p-4 mt-4">
					<div class="text-amber-400 font-semibold mb-2">Refund Information</div>
					<div class="grid grid-cols-1 sm:grid-cols-2 gap-3 text-sm">
						{#if contract.refund_amount_e9s}
							<div>
								<span class="text-white/60">Refund Amount:</span>
								<span class="text-white ml-2 font-medium">{formatPrice(contract.refund_amount_e9s, contract.currency)}</span>
							</div>
						{/if}
						{#if contract.refund_created_at_ns}
							<div>
								<span class="text-white/60">Refund Date:</span>
								<span class="text-white ml-2">{formatDate(contract.refund_created_at_ns)}</span>
							</div>
						{/if}
						{#if contract.stripe_refund_id}
							<div>
								<span class="text-white/60">Stripe Refund ID:</span>
								<span class="text-white/80 ml-2 font-mono text-xs">{contract.stripe_refund_id}</span>
							</div>
						{/if}
						{#if contract.icpay_refund_id}
							<div>
								<span class="text-white/60">ICPay Refund ID:</span>
								<span class="text-white/80 ml-2 font-mono text-xs">{contract.icpay_refund_id}</span>
							</div>
						{/if}
					</div>
					<p class="text-amber-400/70 text-xs mt-3">
						Refunds typically appear on your original payment method within 5-10 business days.
					</p>
				</div>
			{/if}

			<!-- Usage information (shown for contracts with usage tracking) -->
			{#if usage}
				<div class="bg-primary-500/10 border border-primary-500/30 rounded-lg p-4 mt-4">
					<div class="text-primary-400 font-semibold mb-2">Current Billing Period Usage</div>
					<div class="grid grid-cols-1 sm:grid-cols-2 gap-3 text-sm">
						<div>
							<span class="text-white/60">Billing Period:</span>
							<span class="text-white ml-2">
								{new Date(usage.billing_period_start * 1000).toLocaleDateString()} - {new Date(usage.billing_period_end * 1000).toLocaleDateString()}
							</span>
						</div>
						<div>
							<span class="text-white/60">Usage:</span>
							<span class="text-white ml-2 font-medium">{usage.units_used.toFixed(2)} hours</span>
							{#if usage.units_included}
								<span class="text-white/50">/ {usage.units_included} included</span>
							{/if}
						</div>
						{#if usage.overage_units > 0}
							<div>
								<span class="text-white/60">Overage:</span>
								<span class="text-amber-400 ml-2 font-medium">{usage.overage_units.toFixed(2)} hours</span>
							</div>
						{/if}
						{#if usage.estimated_charge_cents}
							<div>
								<span class="text-white/60">Estimated Charge:</span>
								<span class="text-white ml-2 font-medium">${(usage.estimated_charge_cents / 100).toFixed(2)}</span>
							</div>
						{/if}
					</div>
				</div>
			{/if}
		</div>

		<!-- Back link -->
		<div>
			<a
				href="/dashboard/rentals"
				class="inline-flex items-center gap-2 text-white/60 hover:text-white transition-colors"
			>
				← Back to All Rentals
			</a>
		</div>
	{/if}
</div>
