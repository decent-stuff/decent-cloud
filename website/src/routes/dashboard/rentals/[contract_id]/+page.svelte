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
		getContractCredentials,
		getContractRecipeLog,
		requestPasswordReset,
		extendContract,
		getContractExtensions,
		getContractHealthChecks,
		submitContractFeedback,
		getContractFeedback,
		getUserContractBandwidthHistory,
		type Contract,
		type ContractUsage,
		type ContractExtension,
		type ContractHealthCheck,
		type ContractFeedback,
		type BandwidthHistoryResponse,
		hexEncode,
	} from "$lib/services/api";
	import { decryptCredentials } from "$lib/services/credential-crypto";
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

	// Encrypted credentials state
	let decryptedPassword = $state<string | null>(null);
	let credentialsLoading = $state(false);
	let credentialsError = $state<string | null>(null);
	let showPassword = $state(false);

	// Password reset state
	let passwordResetLoading = $state(false);
	let passwordResetSuccess = $state(false);
	let passwordResetError = $state<string | null>(null);

	// Extend contract state
	let showExtendForm = $state(false);
	let extendHours = $state(24);
	let extendMemo = $state("");
	let extending = $state(false);
	let extendSuccess = $state<string | null>(null);
	let extendError = $state<string | null>(null);
	let extensions = $state<ContractExtension[]>([]);
	let healthChecks = $state<ContractHealthCheck[]>([]);

	// Recipe log state
	let recipeLog = $state<string | null>(null);

	// SSH copy state
	let copiedSsh = $state(false);

	// How to Connect guide state
	let showConnectGuide = $state(false);
	let connectGuideTab = $state<'unix' | 'win-terminal' | 'putty'>('unix');

	// Feedback state
	let feedback = $state<ContractFeedback | null>(null);
	let feedbackLoading = $state(false);
	let feedbackSubmitting = $state(false);
	let feedbackError = $state<string | null>(null);
	let feedbackServiceMatched = $state<boolean | null>(null);
	let feedbackWouldRentAgain = $state<boolean | null>(null);

	// Bandwidth history state
	let bandwidthHistory = $state<BandwidthHistoryResponse[]>([]);

	function copySSHCommand(command: string) {
		navigator.clipboard.writeText(command).then(() => {
			copiedSsh = true;
			setTimeout(() => { copiedSsh = false; }, 2000);
		});
	}

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
		if (s === "failed") {
			return { text: "Provisioning failed. You can request a refund or contact support.", isWaiting: false };
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

				// Try to fetch recipe log
				try {
					const logHeaders = (await signRequest(
						signingIdentityInfo.identity as any,
						"GET",
						`/api/v1/contracts/${contractId}/recipe-log`,
					)).headers;
					recipeLog = await getContractRecipeLog(contractId, logHeaders);
				} catch (e) {
					console.debug("No recipe log for contract:", e);
				}

				// Try to fetch and decrypt credentials for provisioned contracts
				if (contract.status === 'provisioned' || contract.status === 'active') {
					await loadCredentials(signingIdentityInfo);
				}
			
				// Load extension history
				await loadExtensions(signingIdentityInfo);
				await loadHealthChecks(signingIdentityInfo);
				await loadFeedback(signingIdentityInfo);
				await loadBandwidthHistory(signingIdentityInfo);
			}
			lastRefresh = Date.now();
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load contract";
			console.error("Error loading contract:", e);
		} finally {
			loading = false;
		}
	}

	async function loadCredentials(signingIdentityInfo: any) {
		try {
			credentialsLoading = true;
			credentialsError = null;

			// Sign request to get encrypted credentials
			const { headers } = await signRequest(
				signingIdentityInfo.identity,
				"GET",
				`/api/v1/contracts/${contractId}/credentials`,
			);

			const encryptedJson = await getContractCredentials(contractId, headers);
			if (!encryptedJson) {
				// No credentials available (either not set or expired)
				return;
			}

			// Get the secret key for decryption
			const keyPair = signingIdentityInfo.identity.getKeyPair();
			const secretKey = keyPair.secretKey;

			// Decrypt the credentials
			decryptedPassword = await decryptCredentials(encryptedJson, secretKey);
		} catch (e) {
			console.debug("No credentials available:", e);
			// Don't show error - credentials may not be set for all contracts
		} finally {
			credentialsLoading = false;
		}
	}

	async function handleRequestPasswordReset() {
		if (!contract) return;

		try {
			passwordResetLoading = true;
			passwordResetSuccess = false;
			passwordResetError = null;

			const signingIdentityInfo = await authStore.getSigningIdentity();
			if (!signingIdentityInfo) {
				passwordResetError = "You must be authenticated";
				return;
			}

			const { headers } = await signRequest(
				signingIdentityInfo.identity as any,
				"POST",
				`/api/v1/contracts/${contractId}/reset-password`,
			);

			await requestPasswordReset(contractId, headers);
			passwordResetSuccess = true;
			decryptedPassword = null; // Clear cached password
		} catch (e) {
			passwordResetError = e instanceof Error ? e.message : String(e);
		} finally {
			passwordResetLoading = false;
		}
	}

	function isTerminalState(status: string): boolean {
		return ['cancelled', 'rejected', 'failed'].includes(status.toLowerCase());
	}

	async function loadFeedback(signingIdentityInfo: any) {
		if (!contract || !isTerminalState(contract.status)) return;
		try {
			const { headers } = await signRequest(
				signingIdentityInfo.identity as any,
				'GET',
				`/api/v1/contracts/${contractId}/feedback`,
			);
			feedback = await getContractFeedback(contractId, headers);
		} catch {
			// Feedback not available is not an error
		}
	}

	async function handleSubmitFeedback() {
		if (!contract || feedbackServiceMatched === null || feedbackWouldRentAgain === null) return;

		try {
			feedbackSubmitting = true;
			feedbackError = null;

			const signingIdentityInfo = await authStore.getSigningIdentity();
			if (!signingIdentityInfo) {
				feedbackError = 'You must be authenticated to submit feedback';
				return;
			}

			const body = {
				service_matched_description: feedbackServiceMatched,
				would_rent_again: feedbackWouldRentAgain,
			};

			const { headers } = await signRequest(
				signingIdentityInfo.identity as any,
				'POST',
				`/api/v1/contracts/${contractId}/feedback`,
				body,
			);

			feedback = await submitContractFeedback(contractId, body, headers);
		} catch (e) {
			feedbackError = e instanceof Error ? e.message : 'Failed to submit feedback';
		} finally {
			feedbackSubmitting = false;
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

	function isExtendable(status: string): boolean {
		return ["provisioned", "active", "accepted"].includes(status.toLowerCase());
	}

	async function handleExtendContract() {
		if (!contract || !isExtendable(contract.status)) return;

		try {
			extending = true;
			extendError = null;
			extendSuccess = null;

			const signingIdentityInfo = await authStore.getSigningIdentity();
			if (!signingIdentityInfo) {
				extendError = "You must be authenticated to extend contracts";
				return;
			}

			const body = { extensionHours: extendHours, memo: extendMemo || undefined };

			const { headers } = await signRequest(
				signingIdentityInfo.identity as any,
				"POST",
				`/api/v1/contracts/${contractId}/extend`,
				body,
			);

			const result = await extendContract(contractId, body, headers);
			extendSuccess = result.message;
			showExtendForm = false;
			extendMemo = "";

			// Reload extensions list and refresh contract
			const extHeaders = (await signRequest(
				signingIdentityInfo.identity as any,
				"GET",
				`/api/v1/contracts/${contractId}/extensions`,
			)).headers;
			extensions = await getContractExtensions(contractId, extHeaders);

			await refreshContract();
		} catch (e) {
			extendError = e instanceof Error ? e.message : "Failed to extend contract";
		} finally {
			extending = false;
		}
	}

	async function loadExtensions(signingIdentityInfo: any) {
		try {
			const { headers } = await signRequest(
				signingIdentityInfo.identity as any,
				"GET",
				`/api/v1/contracts/${contractId}/extensions`,
			);
			extensions = await getContractExtensions(contractId, headers);
		} catch {
			// Extensions not available is not an error
		}
	}

	async function loadHealthChecks(signingIdentityInfo: any) {
		try {
			const { headers } = await signRequest(
				signingIdentityInfo.identity as any,
				"GET",
				`/api/v1/contracts/${contractId}/health`,
			);
			healthChecks = await getContractHealthChecks(contractId, headers);
		} catch {
			// Health checks not available is not an error
		}
	}

	async function loadBandwidthHistory(signingIdentityInfo: any) {
		try {
			const pubkeyHex = hexEncode(signingIdentityInfo.publicKeyBytes);
			const { headers } = await signRequest(
				signingIdentityInfo.identity as any,
				"GET",
				`/api/v1/users/${pubkeyHex}/contracts/${contractId}/bandwidth`,
			);
			bandwidthHistory = await getUserContractBandwidthHistory(pubkeyHex, contractId, headers);
		} catch {
			// Bandwidth history not available is not an error
		}
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

	/**
	 * Open Chatwoot widget with contract context for messaging the provider.
	 * Sets contract_id as custom attribute so the backend can track messages per contract.
	 */
	function contactProvider() {
		// @ts-expect-error - Chatwoot global
		if (typeof window !== 'undefined' && window.$chatwoot) {
			// Set contract context as custom attribute
			// @ts-expect-error - Chatwoot global
			window.$chatwoot.setCustomAttributes({
				contract_id: contractId,
				provider_pubkey: contract?.provider_pubkey || '',
			});
			// Open the chat widget
			// @ts-expect-error - Chatwoot global
			window.$chatwoot.toggle('open');
		} else {
			// Fallback if Chatwoot not loaded - show error
			error = "Chat widget not available. Please refresh the page and try again.";
		}
	}

	onDestroy(() => {
		unsubscribeAuth?.();
		stopAutoRefresh();
	});
</script>

<div class="space-y-8">
	<!-- Breadcrumb -->
	<nav class="text-sm text-neutral-500">
		<a href="/dashboard/rentals" class="hover:text-white transition-colors">My Rentals</a>
		<span class="mx-2">/</span>
		<span class="text-white">{truncateHash(contractId)}</span>
	</nav>

	{#if !isAuthenticated}
		<div class="card p-8 border border-neutral-800 text-center">
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">🔑</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-neutral-400">
					Create an account or login to view contract details.
				</p>
				<button
					onclick={handleLogin}
					class="px-8 py-3 bg-gradient-to-r from-primary-500 to-primary-600  font-semibold text-white hover:brightness-110 hover:scale-105 transition-all"
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
		<div class="bg-red-500/20 border border-red-500/30  p-6 text-center">
			<span class="text-6xl mb-4 block">🔍</span>
			<h2 class="text-2xl font-bold text-red-400 mb-2">Contract Not Found</h2>
			<p class="text-neutral-400 mb-4">{error}</p>
			<a
				href="/dashboard/rentals"
				class="inline-block px-6 py-3 bg-surface-elevated  font-semibold hover:bg-surface-elevated transition-all"
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
				<h1 class="text-2xl font-bold text-white tracking-tight">{contract.offering_id}</h1>
				<p class="text-neutral-500 font-mono text-sm">{contract.contract_id}</p>
			</div>
			<div class="flex items-center gap-3">
				<button
					onclick={contactProvider}
					class="px-3 py-1.5 text-sm bg-primary-600/80 text-white border border-primary-500/30 hover:bg-primary-700 transition-colors"
					title="Message the provider about this contract"
				>
					Contact Provider
				</button>
				<button
					onclick={copyLink}
					class="px-3 py-1.5  text-sm bg-surface-elevated text-neutral-400 border border-neutral-800 hover:bg-surface-elevated transition-colors"
					title="Copy link to this contract"
				>
					🔗 Copy Link
				</button>
				<button
					onclick={toggleAutoRefresh}
					class="flex items-center gap-2 px-3 py-1.5  text-sm transition-colors {autoRefreshEnabled ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/30' : 'bg-surface-elevated text-neutral-500 border border-neutral-800'}"
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
					class="px-3 py-1.5  text-sm bg-surface-elevated text-neutral-400 border border-neutral-800 hover:bg-surface-elevated transition-colors"
					title="Refresh now"
				>
					↻ Refresh
				</button>
			</div>
		</div>

		{#if error}
			<div class="bg-red-500/20 border border-red-500/30  p-4 text-red-400">
				<p class="font-semibold">Error</p>
				<p class="text-sm mt-1">{error}</p>
			</div>
		{/if}

		<!-- Contract card -->
		<div class="card p-6 border border-neutral-800">
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
						<div class="text-neutral-500 text-sm">{contract.duration_hours} hours (one-time)</div>
					{/if}
				</div>
			</div>

			<!-- Failure banner (failed status) -->
			{#if contract.status.toLowerCase() === 'failed'}
				{@const failureDetails = (() => {
					if (!contract.provisioning_instance_details) return null;
					try { return JSON.parse(contract.provisioning_instance_details); } catch { return null; }
				})()}
				<div class="mb-4 p-4 bg-red-500/20 border border-red-500/30 text-red-400">
					<p class="font-semibold mb-1">Provisioning failed</p>
					{#if failureDetails?.error}
						<p class="text-sm text-red-300 mb-2">{failureDetails.error}</p>
					{:else if failureDetails?.message}
						<p class="text-sm text-red-300 mb-2">{failureDetails.message}</p>
					{:else if contract.provisioning_instance_details}
						<pre class="text-xs text-red-300/80 font-mono whitespace-pre-wrap mb-2">{contract.provisioning_instance_details}</pre>
					{/if}
					<p class="text-sm text-red-400/70">You can request a refund or <button onclick={contactProvider} class="underline hover:text-red-300 transition-colors">contact support</button>.</p>
				</div>
			{/if}

			<!-- Progress indicator -->
			{#if stageIndex >= 0}
				<div class="mb-4 p-4 bg-surface-elevated  border border-neutral-800">
					<div class="flex items-center justify-between mb-3">
						{#each LIFECYCLE_STAGES as stage, i}
							<div class="flex flex-col items-center flex-1">
								<div class="flex items-center w-full">
									{#if i > 0}
										<div class="flex-1 h-0.5 {i <= stageIndex ? 'bg-emerald-500' : 'bg-surface-elevated'}"></div>
									{/if}
									<div
										class="w-8 h-8 rounded-full flex items-center justify-center text-sm border-2 transition-all {
											i < stageIndex
												? 'bg-emerald-500/20 border-emerald-500 text-emerald-400'
												: i === stageIndex
													? 'bg-primary-500/20 border-primary-500 text-primary-400 ring-2 ring-primary-500/30'
													: 'bg-surface-elevated border-neutral-800 text-neutral-600'
										}"
									>
										{stage.icon}
									</div>
									{#if i < LIFECYCLE_STAGES.length - 1}
										<div class="flex-1 h-0.5 {i < stageIndex ? 'bg-emerald-500' : 'bg-surface-elevated'}"></div>
									{/if}
								</div>
								<span class="text-xs mt-1 {i <= stageIndex ? 'text-neutral-300' : 'text-neutral-600'}">{stage.label}</span>
							</div>
						{/each}
					</div>
					{#if nextStep}
						<div class="flex items-start gap-2 text-sm {nextStep.isWaiting ? 'text-primary-400' : 'text-neutral-400'}">
							{#if nextStep.isWaiting}
								<div class="animate-pulse mt-0.5">⏳</div>
							{:else}
								<span class="mt-0.5">→</span>
							{/if}
							<div>
								<span>{nextStep.text}</span>
								{#if nextStep.isWaiting}
									<p class="text-neutral-500 text-xs mt-1">
										You'll receive an email when your resource is ready. Make sure your <a href="/dashboard/account/profile" class="text-primary-400 hover:underline">profile</a> has a valid email address.
									</p>
								{/if}
							</div>
						</div>
					{/if}
				</div>
			{/if}

			<!-- Rejected/Cancelled/Failed CTA -->
			{#if contract.status.toLowerCase() === 'rejected' || contract.status.toLowerCase() === 'cancelled' || contract.status.toLowerCase() === 'failed'}
				<div class="mb-4 p-4 bg-surface-elevated border border-neutral-800 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3">
					<p class="text-neutral-400 text-sm">
						{contract.status.toLowerCase() === 'rejected'
							? 'Provider declined this request. You can try another provider.'
							: contract.status.toLowerCase() === 'failed'
								? 'Your resource could not be provisioned. Try a different provider or contact support.'
								: 'This rental has been cancelled.'}
					</p>
					<a
						href="/dashboard/marketplace"
						class="px-4 py-2 text-sm bg-gradient-to-r from-primary-500 to-primary-600 font-semibold text-white hover:brightness-110 transition-all whitespace-nowrap"
					>
						Browse Marketplace
					</a>
				</div>
			{/if}

			<!-- Contract details grid -->
			<div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
				<div class="bg-surface-elevated  p-3 border border-neutral-800">
					<div class="text-neutral-500 text-xs mb-1">Created</div>
					<div class="text-white text-sm">{formatDate(contract.created_at_ns)}</div>
				</div>
				{#if contract.end_timestamp_ns}
					{@const endDate = new Date(contract.end_timestamp_ns / 1_000_000)}
					{@const isExpired = endDate < new Date()}
					<div class="bg-surface-elevated  p-3 border {isExpired ? 'border-red-500/30' : 'border-neutral-800'}">
						<div class="text-neutral-500 text-xs mb-1">{isExpired ? 'Expired' : 'Expires'}</div>
						<div class="text-sm {isExpired ? 'text-red-400' : 'text-white'}">{endDate.toLocaleString()}</div>
					</div>
				{/if}
				{#if contract.region_name}
					<div class="bg-surface-elevated  p-3 border border-neutral-800">
						<div class="text-neutral-500 text-xs mb-1">Region</div>
						<div class="text-white text-sm">{contract.region_name}</div>
					</div>
				{/if}
				{#if contract.requester_ssh_pubkey}
					<div class="bg-surface-elevated  p-3 border border-neutral-800">
						<div class="text-neutral-500 text-xs mb-1">SSH Key</div>
						<div class="text-white text-sm font-mono truncate">
							{truncateHash(contract.requester_ssh_pubkey)}
						</div>
					</div>
				{/if}
				<div class="bg-surface-elevated  p-3 border border-neutral-800">
					<div class="text-neutral-500 text-xs mb-1">Provider</div>
					<a
						href="/dashboard/reputation/{contract.provider_pubkey}"
						class="text-white text-sm font-mono hover:text-primary-400 transition-colors"
					>
						{truncateHash(contract.provider_pubkey)}
					</a>
				</div>
			</div>

			{#if contract.request_memo}
				<div class="bg-surface-elevated  p-3 border border-neutral-800 mb-4">
					<div class="text-neutral-500 text-xs mb-1">Memo</div>
					<div class="text-white text-sm">{contract.request_memo}</div>
				</div>
			{/if}

			{#if contract.provisioning_instance_details}
				{@const instanceDetails = (() => {
					try { return JSON.parse(contract.provisioning_instance_details); } catch { return null; }
				})()}
				<div class="bg-green-500/10 border border-green-500/30  p-4">
					<div class="text-green-400 font-semibold mb-3">Connection Details</div>

					{#if contract.gateway_subdomain && contract.gateway_ssh_port}
						<!-- Gateway-accessible VM -->
						<div class="space-y-3">
							<div class="bg-black/20  p-3">
								<div class="flex items-center justify-between mb-1">
									<div class="text-neutral-500 text-xs">SSH Command</div>
									<button
										onclick={() => copySSHCommand(`ssh -p ${contract!.gateway_ssh_port} root@${contract!.gateway_subdomain}`)}
										class="text-xs px-2 py-0.5 bg-surface-elevated text-neutral-400 border border-neutral-700 hover:text-white transition-colors"
										title="Copy SSH command"
									>
										{copiedSsh ? 'Copied!' : '📋 Copy'}
									</button>
								</div>
								<code class="text-green-300 text-sm font-mono break-all select-all">
									ssh -p {contract.gateway_ssh_port} root@{contract.gateway_subdomain}
								</code>
							</div>
							<!-- How to Connect guide -->
							<div class="bg-black/20 p-3">
								<button
									onclick={() => showConnectGuide = !showConnectGuide}
									class="flex items-center gap-1 text-xs text-neutral-400 hover:text-white transition-colors w-full text-left"
								>
									<span class="inline-block transition-transform duration-200" style="transform: rotate({showConnectGuide ? '180deg' : '0deg'})">&#9660;</span>
									How to Connect
								</button>
								{#if showConnectGuide}
									<div class="mt-3">
										<div class="flex gap-1 mb-3">
											{#each ([['unix', 'Linux / macOS'], ['win-terminal', 'Windows (Terminal)'], ['putty', 'Windows (PuTTY)']] as const) as tab}
												<button
													onclick={() => connectGuideTab = tab[0]}
													class="text-xs px-2 py-1 border transition-colors {connectGuideTab === tab[0] ? 'bg-green-500/20 border-green-500/50 text-green-300' : 'bg-surface-elevated border-neutral-700 text-neutral-400 hover:text-white'}"
												>
													{tab[1]}
												</button>
											{/each}
										</div>
										{#if connectGuideTab === 'unix'}
											<ol class="text-xs text-neutral-300 space-y-2 list-decimal list-inside">
												<li>Open Terminal</li>
												<li>
													Run:
													<div class="flex items-center justify-between mt-1 font-mono text-xs bg-black/30 px-3 py-2 rounded">
														<code class="text-green-300 select-all">ssh -p {contract.gateway_ssh_port} root@{contract.gateway_subdomain}</code>
														<button
															onclick={() => copySSHCommand(`ssh -p ${contract!.gateway_ssh_port} root@${contract!.gateway_subdomain}`)}
															class="ml-2 text-xs px-2 py-0.5 bg-surface-elevated text-neutral-400 border border-neutral-700 hover:text-white transition-colors shrink-0"
														>{copiedSsh ? 'Copied!' : '📋 Copy'}</button>
													</div>
												</li>
												<li>If prompted about host authenticity, type <code class="font-mono text-green-300">yes</code></li>
												<li>You're connected!</li>
											</ol>
										{:else if connectGuideTab === 'win-terminal'}
											<ol class="text-xs text-neutral-300 space-y-2 list-decimal list-inside">
												<li>Open Windows Terminal or PowerShell</li>
												<li>
													Run:
													<div class="flex items-center justify-between mt-1 font-mono text-xs bg-black/30 px-3 py-2 rounded">
														<code class="text-green-300 select-all">ssh -p {contract.gateway_ssh_port} root@{contract.gateway_subdomain}</code>
														<button
															onclick={() => copySSHCommand(`ssh -p ${contract!.gateway_ssh_port} root@${contract!.gateway_subdomain}`)}
															class="ml-2 text-xs px-2 py-0.5 bg-surface-elevated text-neutral-400 border border-neutral-700 hover:text-white transition-colors shrink-0"
														>{copiedSsh ? 'Copied!' : '📋 Copy'}</button>
													</div>
												</li>
												<li>If prompted about host authenticity, type <code class="font-mono text-green-300">yes</code></li>
												<li>You're connected!</li>
											</ol>
										{:else}
											<ol class="text-xs text-neutral-300 space-y-2 list-decimal list-inside">
												<li>Download PuTTY from <a href="https://putty.org" target="_blank" rel="noopener" class="text-green-400 hover:underline">putty.org</a></li>
												<li>Enter host: <code class="font-mono text-green-300">{contract.gateway_subdomain}</code></li>
												<li>Enter port: <code class="font-mono text-green-300">{contract.gateway_ssh_port}</code></li>
												<li>Click <strong>Open</strong></li>
												<li>Login as: <code class="font-mono text-green-300">root</code></li>
											</ol>
										{/if}
									</div>
								{/if}
							</div>
							<div class="bg-black/20  p-3">
								<div class="text-neutral-500 text-xs mb-1">Host</div>
								<code class="text-white text-sm font-mono select-all">{contract.gateway_subdomain}</code>
							</div>
							{#if contract.gateway_ssh_port && contract.gateway_port_range_start && contract.gateway_port_range_end}
								<div class="bg-black/20  p-3">
									<div class="text-neutral-500 text-xs mb-1">Port Forwarding</div>
									<div class="text-xs text-neutral-400 space-y-1 font-mono">
										<div>SSH: External {contract.gateway_ssh_port} → VM:22</div>
										<div>TCP: External {contract.gateway_ssh_port + 1}-{contract.gateway_ssh_port + 4} → VM:10001-10004</div>
										<div>UDP: External {contract.gateway_ssh_port + 5}-{contract.gateway_ssh_port + 9} → VM:10005-10009</div>
									</div>
									<div class="text-neutral-600 text-xs mt-2">
										Run services on VM ports 10001-10009 to expose them externally
									</div>
								</div>
							{/if}
						</div>
					{:else if instanceDetails?.ip_address}
						<!-- Direct IP access VM -->
						<div class="space-y-3">
							<div class="bg-black/20  p-3">
								<div class="flex items-center justify-between mb-1">
									<div class="text-neutral-500 text-xs">SSH Command</div>
									<button
										onclick={() => copySSHCommand(`ssh root@${instanceDetails.ip_address}`)}
										class="text-xs px-2 py-0.5 bg-surface-elevated text-neutral-400 border border-neutral-700 hover:text-white transition-colors"
										title="Copy SSH command"
									>
										{copiedSsh ? 'Copied!' : '📋 Copy'}
									</button>
								</div>
								<code class="text-green-300 text-sm font-mono break-all select-all">
									ssh root@{instanceDetails.ip_address}
								</code>
							</div>
							<div class="bg-black/20  p-3">
								<div class="text-neutral-500 text-xs mb-1">IP Address</div>
								<code class="text-white text-sm font-mono select-all">{instanceDetails.ip_address}</code>
							</div>
							{#if instanceDetails.ipv6_address}
								<div class="bg-black/20  p-3">
									<div class="text-neutral-500 text-xs mb-1">IPv6 Address</div>
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

					{#if decryptedPassword}
						<div class="bg-black/20 p-3 mt-3 border border-amber-500/30">
							<div class="flex items-center justify-between mb-1">
								<div class="text-amber-400 text-xs font-medium">Root Password</div>
								<button
									onclick={() => showPassword = !showPassword}
									class="text-xs text-neutral-400 hover:text-white transition-colors"
								>
									{showPassword ? 'Hide' : 'Show'}
								</button>
							</div>
							{#if showPassword}
								<code class="text-amber-300 text-sm font-mono select-all block">{decryptedPassword}</code>
							{:else}
								<code class="text-neutral-500 text-sm font-mono">••••••••••••</code>
							{/if}
							<div class="text-amber-400/60 text-xs mt-2 space-y-1">
								<div>Save this password now - it can only be decrypted on this device/browser.</div>
								<div>Auto-deletes 7 days after provisioning. You can always use your SSH key to access the VM.</div>
							</div>
						</div>
					{:else if credentialsLoading}
						<div class="text-neutral-500 text-xs mt-3">Loading credentials...</div>
					{/if}

					<!-- Password reset request button -->
					{#if contract.status.toLowerCase() === 'provisioned' || contract.status.toLowerCase() === 'active'}
						<div class="mt-4 pt-3 border-t border-surface-elevated">
							{#if passwordResetSuccess}
								<div class="text-green-400 text-sm">
									Password reset requested. The provider will reset it shortly. Check back in a few minutes.
								</div>
							{:else}
								<button
									onclick={handleRequestPasswordReset}
									disabled={passwordResetLoading}
									class="px-3 py-1.5 text-sm bg-amber-500/20 text-amber-400 border border-amber-500/30 rounded hover:bg-amber-500/30 transition-colors disabled:opacity-50"
								>
									{#if passwordResetLoading}
										Requesting...
									{:else}
										Request Password Reset
									{/if}
								</button>
								{#if passwordResetError}
									<div class="text-red-400 text-xs mt-2">{passwordResetError}</div>
								{/if}
								<p class="text-neutral-500 text-xs mt-2">
									Request a new root password. The provider will reset it and you'll be able to decrypt it here.
								</p>
							{/if}
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
				<div class="bg-purple-500/10 border border-purple-500/30  p-4 mt-4">
					<div class="flex items-center justify-between mb-2">
						<div class="text-purple-400 font-semibold">Subscription</div>
						<span class="px-2 py-0.5 rounded text-xs font-medium {
							contract.subscription_status === 'active' ? 'bg-green-500/20 text-green-400' :
							contract.subscription_status === 'trialing' ? 'bg-primary-500/20 text-primary-400' :
							contract.subscription_status === 'past_due' ? 'bg-amber-500/20 text-amber-400' :
							contract.subscription_status === 'cancelled' ? 'bg-red-500/20 text-red-400' :
							'bg-surface-elevated text-neutral-500'
						}">
							{contract.subscription_status || 'Unknown'}
						</span>
					</div>
					<div class="grid grid-cols-1 sm:grid-cols-2 gap-3 text-sm">
						{#if renewalDate}
							<div>
								<span class="text-neutral-500">{contract.cancel_at_period_end ? 'Ends on:' : 'Renews on:'}</span>
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
				<div class="bg-amber-500/10 border border-amber-500/30  p-4 mt-4">
					<div class="text-amber-400 font-semibold mb-2">Refund Information</div>
					<div class="grid grid-cols-1 sm:grid-cols-2 gap-3 text-sm">
						{#if contract.refund_amount_e9s}
							<div>
								<span class="text-neutral-500">Refund Amount:</span>
								<span class="text-white ml-2 font-medium">{formatPrice(contract.refund_amount_e9s, contract.currency)}</span>
							</div>
						{/if}
						{#if contract.refund_created_at_ns}
							<div>
								<span class="text-neutral-500">Refund Date:</span>
								<span class="text-white ml-2">{formatDate(contract.refund_created_at_ns)}</span>
							</div>
						{/if}
						{#if contract.stripe_refund_id}
							<div>
								<span class="text-neutral-500">Stripe Refund ID:</span>
								<span class="text-neutral-300 ml-2 font-mono text-xs">{contract.stripe_refund_id}</span>
							</div>
						{/if}
						{#if contract.icpay_refund_id}
							<div>
								<span class="text-neutral-500">ICPay Refund ID:</span>
								<span class="text-neutral-300 ml-2 font-mono text-xs">{contract.icpay_refund_id}</span>
							</div>
						{/if}
					</div>
					<p class="text-amber-400/70 text-xs mt-3">
						Refunds typically appear on your original payment method within 5-10 business days.
					</p>
				</div>
			{/if}

			<!-- Recipe execution log (shown when available) -->
			{#if recipeLog}
				<details class="bg-slate-500/10 border border-slate-500/30 p-4 mt-4">
					<summary class="text-slate-400 font-semibold cursor-pointer select-none">Recipe Output</summary>
					<pre class="mt-3 p-3 bg-black/40 rounded text-xs text-neutral-300 font-mono overflow-x-auto max-h-96 overflow-y-auto whitespace-pre-wrap">{recipeLog}</pre>
				</details>
			{/if}

			<!-- Usage information (shown for contracts with usage tracking) -->
			{#if usage}
				{@const billingUnitLabel = usage.billing_unit === 'minute' ? 'minutes' : usage.billing_unit === 'hour' ? 'hours' : usage.billing_unit === 'day' ? 'days' : usage.billing_unit === 'month' ? 'months' : usage.billing_unit}
				<div class="bg-primary-500/10 border border-primary-500/30  p-4 mt-4">
					<div class="text-primary-400 font-semibold mb-2">Current Billing Period Usage</div>
					<div class="grid grid-cols-1 sm:grid-cols-2 gap-3 text-sm">
						<div>
							<span class="text-neutral-500">Billing Period:</span>
							<span class="text-white ml-2">
								{new Date(usage.billing_period_start * 1000).toLocaleDateString()} - {new Date(usage.billing_period_end * 1000).toLocaleDateString()}
							</span>
						</div>
						<div>
							<span class="text-neutral-500">Usage:</span>
							<span class="text-white ml-2 font-medium">{usage.units_used.toFixed(2)} {billingUnitLabel}</span>
							{#if usage.units_included}
								<span class="text-neutral-500">/ {usage.units_included} included</span>
							{/if}
						</div>
						{#if usage.overage_units > 0}
							<div>
								<span class="text-neutral-500">Overage:</span>
								<span class="text-amber-400 ml-2 font-medium">{usage.overage_units.toFixed(2)} {billingUnitLabel}</span>
							</div>
						{/if}
						{#if usage.estimated_charge_cents}
							<div>
								<span class="text-neutral-500">Estimated Charge:</span>
								<span class="text-white ml-2 font-medium">${(usage.estimated_charge_cents / 100).toFixed(2)}</span>
							</div>
						{/if}
					</div>
				</div>
			{/if}
		</div>

	<!-- Bandwidth Usage Chart (gateway contracts with data) -->
	{#if bandwidthHistory.length > 0 && contract.gateway_subdomain}
		{@const points = bandwidthHistory.slice(0, 20).reverse()}
		{@const maxBytes = Math.max(...points.map(p => Math.max(Number(p.bytesIn), Number(p.bytesOut))), 1)}
		{@const chartW = 600}
		{@const chartH = 120}
		{@const padL = 56}
		{@const padR = 8}
		{@const padT = 8}
		{@const padB = 32}
		{@const plotW = chartW - padL - padR}
		{@const plotH = chartH - padT - padB}
		{@const n = points.length}
		{@const xStep = n > 1 ? plotW / (n - 1) : plotW}
		{@const toX = (i: number) => padL + i * xStep}
		{@const toY = (v: number) => padT + plotH - (v / maxBytes) * plotH}
		{@const formatBytes = (b: number) => b >= 1_073_741_824 ? `${(b/1_073_741_824).toFixed(1)}G` : b >= 1_048_576 ? `${(b/1_048_576).toFixed(1)}M` : b >= 1024 ? `${(b/1024).toFixed(0)}K` : `${b}B`}
		{@const formatTime = (ns: number) => { const d = new Date(ns / 1_000_000); return `${d.getHours().toString().padStart(2,'0')}:${d.getMinutes().toString().padStart(2,'0')}`; }}
		<div class="card p-6 border border-neutral-800">
			<h3 class="text-sm font-semibold text-neutral-300 mb-3">Bandwidth Usage</h3>
			<div class="overflow-x-auto">
				<svg viewBox="0 0 {chartW} {chartH}" class="w-full" style="min-width:320px;max-height:160px">
					<!-- Y axis ticks -->
					{#each [0, 0.5, 1] as frac}
						{@const yVal = Math.round(maxBytes * frac)}
						{@const y = toY(yVal)}
						<line x1={padL} y1={y} x2={chartW - padR} y2={y} stroke="#374151" stroke-width="0.5"/>
						<text x={padL - 4} y={y + 4} text-anchor="end" class="text-[9px]" fill="#6b7280" font-size="9">{formatBytes(yVal)}</text>
					{/each}
					<!-- bytes_in filled area -->
					{#if n > 1}
						{@const areaIn = `M${toX(0)},${padT + plotH} ` + points.map((p, i) => `L${toX(i)},${toY(Number(p.bytesIn))}`).join(' ') + ` L${toX(n-1)},${padT + plotH} Z`}
						{@const areaOut = `M${toX(0)},${padT + plotH} ` + points.map((p, i) => `L${toX(i)},${toY(Number(p.bytesOut))}`).join(' ') + ` L${toX(n-1)},${padT + plotH} Z`}
						<path d={areaIn} fill="#3b82f6" fill-opacity="0.15"/>
						<path d={areaOut} fill="#10b981" fill-opacity="0.15"/>
						<polyline points={points.map((p, i) => `${toX(i)},${toY(Number(p.bytesIn))}`).join(' ')} fill="none" stroke="#3b82f6" stroke-width="1.5"/>
						<polyline points={points.map((p, i) => `${toX(i)},${toY(Number(p.bytesOut))}`).join(' ')} fill="none" stroke="#10b981" stroke-width="1.5"/>
					{:else}
						<!-- Single data point: render dots -->
						<circle cx={toX(0)} cy={toY(Number(points[0].bytesIn))} r="3" fill="#3b82f6"/>
						<circle cx={toX(0)} cy={toY(Number(points[0].bytesOut))} r="3" fill="#10b981"/>
					{/if}
					<!-- X axis labels (show at most 5 evenly spaced) -->
					{#each points as p, i}
						{#if n <= 5 || i % Math.ceil((n - 1) / 4) === 0 || i === n - 1}
							<text x={toX(i)} y={chartH - 4} text-anchor="middle" fill="#6b7280" font-size="8">{formatTime(Number(p.recordedAtNs))}</text>
						{/if}
					{/each}
				</svg>
			</div>
			<div class="flex items-center gap-4 mt-2 text-xs text-neutral-400">
				<span class="flex items-center gap-1"><span class="inline-block w-3 h-0.5 bg-blue-500"></span> In</span>
				<span class="flex items-center gap-1"><span class="inline-block w-3 h-0.5 bg-emerald-500"></span> Out</span>
				<span class="text-neutral-600">{points.length} sample{points.length !== 1 ? 's' : ''}</span>
			</div>
		</div>
	{/if}

	<!-- Extension History -->
	{#if extensions.length > 0}
		<div class="card p-6 border border-neutral-800">
			<h3 class="text-sm font-semibold text-neutral-300 mb-3">Extension History ({extensions.length})</h3>
			<div class="space-y-2">
				{#each extensions as ext (ext.id)}
					<div class="bg-surface-elevated p-3 border border-neutral-800">
						<div class="flex items-center justify-between">
							<div class="text-sm text-white">+{ext.extension_hours}h extended</div>
							<div class="text-xs text-neutral-500">{new Date(ext.created_at_ns / 1_000_000).toLocaleString()}</div>
						</div>
						<div class="text-xs text-neutral-500 mt-1">
							New end: {new Date(ext.new_end_timestamp_ns / 1_000_000).toLocaleString()}
						</div>
						{#if ext.extension_memo}
							<div class="text-xs text-neutral-400 mt-1 italic">{ext.extension_memo}</div>
						{/if}
					</div>
				{/each}
			</div>
		</div>
	{/if}

	<!-- Health Status -->
	{#if healthChecks.length > 0}
		<div class="card p-6 border border-neutral-800">
			<h3 class="text-sm font-semibold text-neutral-300 mb-3">Health Status ({healthChecks.length})</h3>
			<div class="flex items-center gap-2 mb-4">
				<span class="text-xs text-neutral-400">Latest:</span>
				{#if healthChecks[0].status === 'healthy'}
					<span class="px-2 py-0.5 rounded text-xs font-medium bg-green-900 text-green-300">healthy</span>
				{:else if healthChecks[0].status === 'unhealthy'}
					<span class="px-2 py-0.5 rounded text-xs font-medium bg-red-900 text-red-300">unhealthy</span>
				{:else}
					<span class="px-2 py-0.5 rounded text-xs font-medium bg-neutral-700 text-neutral-300">unknown</span>
				{/if}
				{#if healthChecks[0].latencyMs !== undefined}
					<span class="text-xs text-neutral-500">{healthChecks[0].latencyMs}ms</span>
				{/if}
			</div>
			<div class="overflow-x-auto">
				<table class="w-full text-xs">
					<thead>
						<tr class="text-neutral-500 border-b border-neutral-800">
							<th class="text-left py-1 pr-4">Time</th>
							<th class="text-left py-1 pr-4">Status</th>
							<th class="text-left py-1 pr-4">Latency</th>
							<th class="text-left py-1">Details</th>
						</tr>
					</thead>
					<tbody>
						{#each healthChecks as hc (hc.id)}
							<tr class="border-b border-neutral-800/50">
								<td class="py-1 pr-4 text-neutral-400 whitespace-nowrap">{new Date(hc.checkedAt / 1_000_000).toLocaleString()}</td>
								<td class="py-1 pr-4">
									{#if hc.status === 'healthy'}
										<span class="px-1.5 py-0.5 rounded text-xs bg-green-900 text-green-300">healthy</span>
									{:else if hc.status === 'unhealthy'}
										<span class="px-1.5 py-0.5 rounded text-xs bg-red-900 text-red-300">unhealthy</span>
									{:else}
										<span class="px-1.5 py-0.5 rounded text-xs bg-neutral-700 text-neutral-300">{hc.status}</span>
									{/if}
								</td>
								<td class="py-1 pr-4 text-neutral-400">{hc.latencyMs !== undefined ? `${hc.latencyMs}ms` : '—'}</td>
								<td class="py-1 text-neutral-400">{hc.details ?? '—'}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</div>
	{/if}

	<!-- Feedback card (shown for terminal-state contracts) -->
	{#if isTerminalState(contract.status)}
		<div class="card p-6 border border-neutral-800" id="feedback">
			<h3 class="text-sm font-semibold text-neutral-300 mb-4">Rate Your Experience</h3>
			{#if feedback}
				<div class="space-y-3 text-sm">
					<div class="text-emerald-400 font-medium mb-3">Thank you for your feedback!</div>
					<div class="flex items-center gap-3">
						<span class="text-neutral-500 w-52">Service matched description:</span>
						<span class="{feedback.service_matched_description ? 'text-emerald-400' : 'text-red-400'} font-medium">
							{feedback.service_matched_description ? 'Yes' : 'No'}
						</span>
					</div>
					<div class="flex items-center gap-3">
						<span class="text-neutral-500 w-52">Would rent again:</span>
						<span class="{feedback.would_rent_again ? 'text-emerald-400' : 'text-red-400'} font-medium">
							{feedback.would_rent_again ? 'Yes' : 'No'}
						</span>
					</div>
				</div>
			{:else if feedbackLoading}
				<div class="text-neutral-500 text-sm">Loading...</div>
			{:else}
				<div class="space-y-5">
					<div>
						<p class="text-neutral-400 text-sm mb-2">Did the service match its description?</p>
						<div class="flex gap-2">
							<button
								onclick={() => { feedbackServiceMatched = true; }}
								class="px-4 py-1.5 text-sm border transition-colors {feedbackServiceMatched === true ? 'bg-emerald-500/20 border-emerald-500/60 text-emerald-300' : 'bg-surface-elevated border-neutral-700 text-neutral-400 hover:border-neutral-500'}"
							>Yes</button>
							<button
								onclick={() => { feedbackServiceMatched = false; }}
								class="px-4 py-1.5 text-sm border transition-colors {feedbackServiceMatched === false ? 'bg-red-500/20 border-red-500/60 text-red-300' : 'bg-surface-elevated border-neutral-700 text-neutral-400 hover:border-neutral-500'}"
							>No</button>
						</div>
					</div>
					<div>
						<p class="text-neutral-400 text-sm mb-2">Would you rent from this provider again?</p>
						<div class="flex gap-2">
							<button
								onclick={() => { feedbackWouldRentAgain = true; }}
								class="px-4 py-1.5 text-sm border transition-colors {feedbackWouldRentAgain === true ? 'bg-emerald-500/20 border-emerald-500/60 text-emerald-300' : 'bg-surface-elevated border-neutral-700 text-neutral-400 hover:border-neutral-500'}"
							>Yes</button>
							<button
								onclick={() => { feedbackWouldRentAgain = false; }}
								class="px-4 py-1.5 text-sm border transition-colors {feedbackWouldRentAgain === false ? 'bg-red-500/20 border-red-500/60 text-red-300' : 'bg-surface-elevated border-neutral-700 text-neutral-400 hover:border-neutral-500'}"
							>No</button>
						</div>
					</div>
					{#if feedbackError}
						<div class="text-red-400 text-sm">{feedbackError}</div>
					{/if}
					<button
						onclick={handleSubmitFeedback}
						disabled={feedbackServiceMatched === null || feedbackWouldRentAgain === null || feedbackSubmitting}
						class="px-5 py-2 text-sm bg-primary-600 text-white hover:bg-primary-700 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
					>
						{#if feedbackSubmitting}Submitting...{:else}Submit Feedback{/if}
					</button>
				</div>
			{/if}
		</div>
	{/if}

		<!-- Back link -->
		<div>
			<a
				href="/dashboard/rentals"
				class="inline-flex items-center gap-2 text-neutral-500 hover:text-white transition-colors"
			>
				← Back to All Rentals
			</a>
		</div>
	{/if}
</div>
