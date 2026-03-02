<script lang="ts">
	import { onMount, onDestroy, tick } from "svelte";
	import { page } from "$app/stores";
	import { goto } from "$app/navigation";
	import AuthRequiredCard from "$lib/components/AuthRequiredCard.svelte";
	import {
		getUserContracts,
		cancelRentalRequest,
		downloadContractInvoice,
		getOffering,
		fetchIcpPrice,
		type Contract,
		hexEncode,
	} from "$lib/services/api";
	import { getContractStatusBadge as getStatusBadge } from "$lib/utils/contract-status";
	import {
		formatContractDate as formatDate,
		formatContractPrice as formatPrice,
		truncateContractHash as truncateHash,
		formatTimeRemaining,
		getProvisioningElapsed,
		isProvisioningStuck,
		calculateSpendingByCurrency,
	} from "$lib/utils/contract-format";
	import { buildDashboardCtaClass } from "$lib/utils/dashboard-cta";
	import { authStore } from "$lib/stores/auth";
	import { signRequest } from "$lib/services/auth-api";
	import { UserApiClient } from "$lib/services/user-api";
	import { buildContractEventsUrl, parseContractEvent } from "$lib/utils/contract-sse";
	import { get } from "svelte/store";
	import type { Ed25519KeyIdentity } from "@dfinity/identity";

	const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || '';

	let contracts = $state<Contract[]>([]);
	let offeringNames = $state<Map<number, string>>(new Map());
	let activeTab = $state<'all' | 'active' | 'pending' | 'cancelled'>('all');
	let searchQuery = $state('');
	let loading = $state(true);
	let error = $state<string | null>(null);
	let cancellingContractId = $state<string | null>(null);
	let downloadingInvoiceContractId = $state<string | null>(null);
	let copiedCommand = $state<string | null>(null);
	let isAuthenticated = $state(false);
	let unsubscribeAuth: (() => void) | null = null;
	let highlightedContractId = $state<string | null>(null);
	let icpPriceUsd = $state<number | null>(null);
	let pendingGuidanceDismissed = $state(
		typeof sessionStorage !== 'undefined' && sessionStorage.getItem('pending_guidance_dismissed') === '1'
	);

	const spendingStats = $derived({
		total: contracts.length,
		active: contracts.filter(c => ['active', 'provisioned'].includes(c.status)).length,
		spentByCurrency: calculateSpendingByCurrency(contracts)
	});

	const pendingCount = $derived(
		contracts.filter(c => ['requested', 'pending'].includes(c.status.toLowerCase())).length
	);

	// SSE state
	let sseConnected = $state(false);
	let eventSource: EventSource | null = null;

	const PENDING_STATUSES = new Set(['requested', 'pending', 'accepted', 'provisioning', 'provisioned']);
	const CANCELLED_STATUSES = new Set(['cancelled', 'rejected', 'failed']);

	let filteredContracts = $derived.by(() => {
		const byTab = activeTab === 'all'
			? contracts
			: activeTab === 'active'
				? contracts.filter((c) => c.status.toLowerCase() === 'active')
				: activeTab === 'pending'
					? contracts.filter((c) => PENDING_STATUSES.has(c.status.toLowerCase()))
					: contracts.filter((c) => CANCELLED_STATUSES.has(c.status.toLowerCase()));
		const q = searchQuery.trim().toLowerCase();
		if (!q) return byTab;
		return byTab.filter((c) =>
			c.contract_id.toLowerCase().includes(q) ||
			(c.offering_name ?? '').toLowerCase().includes(q) ||
			(offeringNames.get(Number(c.offering_id)) ?? '').toLowerCase().includes(q)
		);
	});

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

	async function fetchOfferingNames(contractList: Contract[]) {
		const ids = [...new Set(contractList.map((c) => parseInt(c.offering_id, 10)).filter((id) => !isNaN(id) && !offeringNames.has(id)))];
		if (ids.length === 0) return;
		const results = await Promise.allSettled(ids.map((id) => getOffering(id)));
		const updated = new Map(offeringNames);
		results.forEach((result, i) => {
			if (result.status === 'fulfilled') {
				const o = result.value;
				updated.set(ids[i], o.offer_name);
			}
		});
		offeringNames = updated;
	}

	async function connectSSE(pubkeyHex: string) {
		closeSSE();
		const signingIdentityInfo = await authStore.getSigningIdentity();
		if (!signingIdentityInfo) return;

		const { headers } = await signRequest(
			signingIdentityInfo.identity as any,
			"GET",
			`/api/v1/users/${pubkeyHex}/contract-events`
		);
		const url = buildContractEventsUrl(pubkeyHex, API_BASE_URL, headers);
		eventSource = new EventSource(url);
		eventSource.addEventListener('contract-status', (ev) => {
			try {
				const update = parseContractEvent(ev.data);
				contracts = contracts.map((c) =>
					c.contract_id === update.contract_id
						? { ...c, status: update.status, status_updated_at_ns: update.updated_at_ns }
						: c
				);
			} catch (e) {
				console.error('[Rentals] Failed to parse contract SSE event:', e);
			}
		});
		eventSource.onopen = () => { sseConnected = true; };
		eventSource.onerror = () => { sseConnected = false; };
	}

	function closeSSE() {
		if (eventSource) {
			eventSource.close();
			eventSource = null;
			sseConnected = false;
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

			const updated = await getUserContracts(
				headers,
				hexEncode(signingIdentityInfo.publicKeyBytes),
			);
			contracts = updated;
			await fetchOfferingNames(updated);
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

			const loaded = await getUserContracts(
				headers,
				hexEncode(signingIdentityInfo.publicKeyBytes),
			);
			contracts = loaded;
			await fetchOfferingNames(loaded);
			icpPriceUsd = await fetchIcpPrice();
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

	// Complete a pending SSH key save that was interrupted by Stripe redirect.
	// The RentalRequestDialog stores the key in localStorage before redirecting.
	async function maybeSavePendingSshKey() {
		const pendingSshKey = localStorage.getItem('dc_pending_ssh_save');
		if (!pendingSshKey) return;
		localStorage.removeItem('dc_pending_ssh_save');
		const identity = get(authStore.activeIdentity);
		if (!identity?.account?.username || !identity.identity) return;
		try {
			const client = new UserApiClient(identity.identity as Ed25519KeyIdentity);
			const keyType = pendingSshKey.split(' ')[0];
			await client.addExternalKey(identity.account.username, { keyType, keyData: pendingSshKey });
		} catch (e) {
			console.warn('Failed to save SSH key after Stripe payment:', e);
		}
	}

	onMount(async () => {
		// Read contract ID from URL params for deep-linking
		highlightedContractId = $page.url.searchParams.get("contract");

		// Complete pending SSH key save from Stripe checkout flow (non-blocking)
		maybeSavePendingSshKey();

		unsubscribeAuth = authStore.isAuthenticated.subscribe(async (isAuth) => {
			isAuthenticated = isAuth;
			await loadContracts();
			scrollToHighlightedContract();
			if (isAuth) {
				const signingIdentityInfo = await authStore.getSigningIdentity();
				if (signingIdentityInfo) {
					connectSSE(hexEncode(signingIdentityInfo.publicKeyBytes));
				}
			} else {
				closeSSE();
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
			const afterCancel = await getUserContracts(getHeaders, pubkeyHex);
			contracts = afterCancel;
			await fetchOfferingNames(afterCancel);
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

	/**
	 * Open Chatwoot widget with contract context for messaging the provider.
	 */
	function formatSshKeyDisplay(key: string): string {
		const parts = key.trim().split(/\s+/);
		if (parts.length >= 3) return parts[2]; // comment (email/name)
		if (parts.length === 2) return `...${parts[1].slice(-20)}`; // last 20 of key data
		return `...${key.slice(-20)}`; // no spaces: show tail
	}

	async function copyToClipboard(text: string, key: string) {
		await navigator.clipboard.writeText(text);
		copiedCommand = key;
		setTimeout(() => { copiedCommand = null; }, 2000);
	}

	function contactProvider(contractId: string, providerPubkey: string) {
		// @ts-expect-error - Chatwoot global
		if (typeof window !== 'undefined' && window.$chatwoot) {
			// @ts-expect-error - Chatwoot global
			window.$chatwoot.setCustomAttributes({
				contract_id: contractId,
				provider_pubkey: providerPubkey,
			});
			// @ts-expect-error - Chatwoot global
			window.$chatwoot.toggle('open');
		}
	}

	onDestroy(() => {
		unsubscribeAuth?.();
		closeSSE();
	});
</script>

<div class="space-y-8">
	<div class="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-4">
		<div>
			<h1 class="text-2xl font-bold text-white tracking-tight mb-2">My Rentals</h1>
			<p class="text-neutral-500">
				View and manage your resource rental requests
			</p>
		</div>
		{#if isAuthenticated && contracts.length > 0}
			<div class="flex items-center gap-3">
				<div class="flex items-center gap-2 px-3 py-1.5 text-sm {sseConnected ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/30' : 'bg-surface-elevated text-neutral-500 border border-neutral-800'}">
					<span class="relative flex h-2 w-2">
						{#if sseConnected}
							<span class="animate-ping absolute inline-flex h-full w-full bg-emerald-400 opacity-75"></span>
						{/if}
						<span class="relative inline-flex h-2 w-2 {sseConnected ? 'bg-emerald-400' : 'bg-white/30'}"></span>
					</span>
					{sseConnected ? 'Live' : 'Disconnected'}
				</div>
				<button
					onclick={refreshContracts}
					class="px-3 py-1.5  text-sm bg-surface-elevated text-neutral-400 border border-neutral-800 hover:bg-surface-elevated transition-colors"
					title="Refresh now"
				>
					↻ Refresh
				</button>
			</div>
		{/if}
	</div>

	{#if !isAuthenticated}
		<AuthRequiredCard subtext="Create an account or login to view and manage your rental contracts. See the marketplace to browse available resources." />
	{:else if error}
		<div
			class="bg-red-500/20 border border-red-500/30  p-4 text-red-400"
		>
			<p class="font-semibold">Error loading rentals</p>
			<p class="text-sm mt-1">{error}</p>
		</div>
	{/if}

	{#if loading}
		<div class="flex justify-center items-center p-8">
			<div
				class="animate-spin  h-12 w-12 border-t-2 border-b-2 border-primary-400"
			></div>
		</div>
	{:else if isAuthenticated && contracts.length === 0}
		<div class="text-center py-12">
			<span class="text-5xl mb-4 block">📋</span>
			<h3 class="text-2xl font-bold text-white mb-2">No Rentals Yet</h3>
			<p class="text-neutral-500 mb-6">Get started in three steps</p>
			<div class="flex items-center justify-center gap-3 max-w-2xl mx-auto mb-8">
				<div class="flex-1 bg-surface-elevated border border-neutral-800 p-4 text-center">
					<div class="text-2xl mb-2">🔍</div>
					<div class="text-xs font-bold text-neutral-300 mb-1">1. Browse</div>
					<div class="text-xs text-neutral-500">Find VMs from providers</div>
				</div>
				<div class="text-neutral-600 text-xl shrink-0">→</div>
				<div class="flex-1 bg-surface-elevated border border-neutral-800 p-4 text-center">
					<div class="text-2xl mb-2">💳</div>
					<div class="text-xs font-bold text-neutral-300 mb-1">2. Rent &amp; Pay</div>
					<div class="text-xs text-neutral-500">Pay with ICP or card</div>
				</div>
				<div class="text-neutral-600 text-xl shrink-0">→</div>
				<div class="flex-1 bg-surface-elevated border border-neutral-800 p-4 text-center">
					<div class="text-2xl mb-2">🖥️</div>
					<div class="text-xs font-bold text-neutral-300 mb-1">3. SSH In</div>
					<div class="text-xs text-neutral-500">Ready in 5–15 min</div>
				</div>
			</div>
			<div class="flex flex-col sm:flex-row items-center justify-center gap-3 mb-8">
				<a
					href="/dashboard/marketplace?preset=gpu"
					class="{buildDashboardCtaClass('rentals-empty-state-cta')} bg-purple-500/20 border border-purple-500/40 text-purple-300 hover:bg-purple-500/30"
				>
					<span class="text-lg">🎮</span> Browse GPU Servers
				</a>
				<a
					href="/dashboard/marketplace?preset=budget"
					class="{buildDashboardCtaClass('rentals-empty-state-cta')} bg-emerald-500/20 border border-emerald-500/40 text-emerald-300 hover:bg-emerald-500/30"
				>
					<span class="text-lg">💰</span> Find Budget VMs
				</a>
				<a
					href="/dashboard/marketplace"
					class="{buildDashboardCtaClass('rentals-empty-state-cta')} bg-gradient-to-r from-primary-500 to-primary-600 text-white hover:brightness-110"
				>
					<span class="text-lg">🚀</span> Explore Marketplace
				</a>
			</div>
		</div>
	{:else}
		<!-- Pending guidance banner -->
		{#if pendingCount > 0 && !pendingGuidanceDismissed}
			<div class="bg-primary-500/10 border border-primary-500/30 p-4 flex items-start gap-3">
				<span class="text-xl shrink-0">📋</span>
				<div class="flex-1 min-w-0">
					<p class="text-primary-300 font-semibold text-sm">You have {pendingCount} pending rental request{pendingCount > 1 ? 's' : ''}</p>
					<p class="text-neutral-400 text-xs mt-0.5">Providers typically respond within 1–24 hours. Once accepted, your VM will be provisioned automatically. You'll receive a notification when it's ready.</p>
				</div>
				<div class="flex items-center gap-2 shrink-0">
					<button
						onclick={() => { activeTab = 'pending'; }}
						class={buildDashboardCtaClass('rentals-pending-guidance-view')}
					>View Pending</button>
					<button
						onclick={() => { pendingGuidanceDismissed = true; sessionStorage.setItem('pending_guidance_dismissed', '1'); }}
						class={buildDashboardCtaClass('rentals-pending-guidance-dismiss')}
						aria-label="Dismiss"
					>×</button>
				</div>
			</div>
		{/if}

		<!-- Search -->
		<div class="relative">
			<input
				type="text"
				bind:value={searchQuery}
				placeholder="Search by contract ID or offering name..."
				class="w-full bg-surface-elevated border border-neutral-800 px-4 py-2 text-sm text-white placeholder-neutral-600 focus:outline-none focus:border-neutral-600 transition-colors"
			/>
			{#if searchQuery}
				<button
					onclick={() => { searchQuery = ''; }}
					class="absolute right-3 top-1/2 -translate-y-1/2 text-neutral-500 hover:text-white transition-colors"
					aria-label="Clear search"
				>×</button>
			{/if}
		</div>

		{#if !loading && contracts.length > 0}
		<div class="grid grid-cols-3 gap-3">
			<div class="bg-surface-elevated border border-neutral-800 p-4">
				<p class="text-neutral-500 text-xs">Total Contracts</p>
				<p class="text-xl font-bold text-white mt-1">{spendingStats.total}</p>
			</div>
			<div class="bg-surface-elevated border border-neutral-800 p-4">
				<p class="text-neutral-500 text-xs">Active Now</p>
				<p class="text-xl font-bold text-emerald-400 mt-1">{spendingStats.active}</p>
			</div>
			<div class="bg-surface-elevated border border-neutral-800 p-4">
				<p class="text-neutral-500 text-xs">Total Spent</p>
				<div class="mt-1 space-y-0.5">
					{#if spendingStats.spentByCurrency.size === 0}
						<p class="text-xl font-bold text-primary-400">0.00</p>
					{:else}
						{#each [...spendingStats.spentByCurrency.entries()] as [currency, amount]}
							<p class="text-xl font-bold text-primary-400">
								{amount.toFixed(2)} {currency}
								{#if currency === 'ICP' && icpPriceUsd && amount > 0}
									<span class="text-neutral-600 text-xs ml-1">&#x2248; ${(amount * icpPriceUsd).toFixed(2)}</span>
								{/if}
							</p>
						{/each}
					{/if}
				</div>
			</div>
		</div>
		{/if}

		<!-- Status filter tab bar -->
		<div class="flex gap-1 border-b border-neutral-800 mb-2">
			{#each [
				{ key: 'all', label: 'All', count: contracts.length, title: 'All your rental contracts' },
				{ key: 'active', label: 'Active', count: contracts.filter((c) => c.status.toLowerCase() === 'active').length, title: 'VMs currently running and accessible' },
				{ key: 'pending', label: 'Pending', count: contracts.filter((c) => PENDING_STATUSES.has(c.status.toLowerCase())).length, title: 'Awaiting provider acceptance or VM provisioning' },
				{ key: 'cancelled', label: 'Cancelled / Failed', count: contracts.filter((c) => CANCELLED_STATUSES.has(c.status.toLowerCase())).length, title: 'Terminated or rejected contracts' },
			] as tab}
				<button
					onclick={() => { activeTab = tab.key as typeof activeTab; }}
					title={tab.title}
					class="px-4 py-2 text-sm font-medium transition-colors border-b-2 -mb-px {activeTab === tab.key ? 'border-primary-400 text-white' : 'border-transparent text-neutral-500 hover:text-neutral-300'}"
				>
					{tab.label}
					{#if tab.count > 0}
						<span class="ml-1.5 px-1.5 py-0.5 text-xs rounded-full {activeTab === tab.key ? 'bg-primary-500/30 text-primary-300' : 'bg-neutral-800 text-neutral-500'}">{tab.count}</span>
					{/if}
				</button>
			{/each}
		</div>
		<div class="space-y-4">
			{#each filteredContracts as contract}
				{@const statusBadge = getStatusBadge(contract.status, contract.payment_status)}
				{@const isHighlighted = highlightedContractId === contract.contract_id}
				{@const stageIndex = getStageIndex(contract.status, contract.payment_status)}
				{@const nextStep = getNextStepInfo(contract.status, contract.payment_status)}
				{@const provisioningElapsed = getProvisioningElapsed(contract.status, contract.created_at_ns, contract.status_updated_at_ns)}
				{@const provisioningStuck = isProvisioningStuck(contract.status, contract.created_at_ns, contract.status_updated_at_ns)}
				{@const expiryInfo = (() => {
					const endNs = contract.end_timestamp_ns ?? contract.current_period_end_ns;
					return ['active', 'provisioned'].includes(contract.status.toLowerCase()) ? formatTimeRemaining(endNs) : null;
				})()}
				<a
					href="/dashboard/rentals/{contract.contract_id}"
					id="contract-{contract.contract_id}"
					class="block card p-6 border transition-all cursor-pointer {isHighlighted
						? 'border-primary-400 ring-2 ring-primary-400/50'
						: 'border-neutral-800 hover:border-primary-400 hover:bg-white/[0.12]'}"
				>
					<div class="flex items-start justify-between mb-4">
						<div class="flex-1">
							<div class="flex items-center gap-3 mb-2">
								<h3 class="text-xl font-bold text-white">
									{contract.offering_name ?? offeringNames.get(parseInt(contract.offering_id, 10)) ?? contract.offering_id}
								</h3>
								<span
									class="inline-flex items-center gap-1 px-3 py-1  text-xs font-medium border {provisioningStuck ? 'bg-amber-500/20 text-amber-400 border-amber-500/30' : statusBadge.class}"
								>
									<span>{statusBadge.icon}</span>
									{statusBadge.text}
									{#if provisioningElapsed}
										<span class="opacity-70 font-normal">· {provisioningElapsed}</span>
									{/if}
								</span>
								{#if provisioningStuck}
									<span class="inline-flex items-center gap-1.5 px-2 py-0.5 text-xs bg-amber-500/10 text-amber-400 border border-amber-500/30">
										Taking longer than usual ·
										<button
											onclick={(e) => { e.preventDefault(); e.stopPropagation(); contactProvider(contract.contract_id, contract.provider_pubkey); }}
											class="underline hover:no-underline"
										>Contact provider</button>
									</span>
								{/if}
								{#if expiryInfo}
									<span class="inline-flex items-center gap-1 px-2 py-0.5 text-xs font-medium rounded {
										expiryInfo.urgency === 'critical' ? 'bg-red-500/20 text-red-400 border border-red-500/30' :
										expiryInfo.urgency === 'warning' ? 'bg-amber-500/20 text-amber-400 border border-amber-500/30' :
										'bg-neutral-800 text-neutral-400 border border-neutral-700'
									}">
										&#x23F1; {expiryInfo.text}
									</span>
								{/if}
								<!-- Cancel button for cancelable contracts -->
								{#if isCancellable(contract.status) && cancellingContractId !== contract.contract_id}
									<button
										onclick={(e) => {
											e.preventDefault();
											e.stopPropagation();
											handleCancelContract(contract.contract_id, contract.status);
										}}
										class="{buildDashboardCtaClass('rentals-contract-action-secondary')} bg-red-600/80 hover:bg-red-700"
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
										class="{buildDashboardCtaClass('rentals-contract-action-primary')} bg-primary-600/80 hover:bg-primary-700"
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
											class="animate-spin  h-3 w-3 border-t border-b border-primary-400"
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
											class="animate-spin  h-3 w-3 border-t border-b border-red-400"
										></div>
										Cancelling...
									</div>
								{/if}
								<!-- Renew button for terminal contracts -->
								{#if CANCELLED_STATUSES.has(contract.status.toLowerCase())}
									<button
										type="button"
										onclick={(e) => { e.preventDefault(); e.stopPropagation(); goto(`/dashboard/marketplace/${contract.offering_id}`); }}
										class="{buildDashboardCtaClass('rentals-contract-action-primary')} bg-primary-600/80 hover:bg-primary-700"
										title="Re-rent the same offering"
									>
										&#8635; Renew
									</button>
								{/if}
								<!-- Rate provider button for terminal contracts -->
								{#if CANCELLED_STATUSES.has(contract.status.toLowerCase())}
									<button
										type="button"
										onclick={(e) => { e.preventDefault(); e.stopPropagation(); goto(`/dashboard/rentals/${contract.contract_id}#feedback`); }}
										class="{buildDashboardCtaClass('rentals-contract-action-warning')} bg-amber-500/20 text-amber-400 border-amber-500/30 hover:bg-amber-500/30"
										title="Rate this provider"
									>
										Rate Provider
									</button>
								{/if}
							</div>
							<p class="text-neutral-500 text-sm">
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
								<div class="text-neutral-500 text-sm">
									{contract.duration_hours} hours
								</div>
							{/if}
						</div>
					</div>

					<!-- Progress indicator (only for active rental flows) -->
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
												class="w-8 h-8  flex items-center justify-center text-sm border-2 transition-all {
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
							class="bg-surface-elevated  p-3 border border-neutral-800"
						>
							<div class="text-neutral-500 text-xs mb-1">
								Created
							</div>
							<div class="text-white text-sm">
								{formatDate(contract.created_at_ns)}
							</div>
						</div>
						{#if contract.region_name}
							<div
								class="bg-surface-elevated  p-3 border border-neutral-800"
							>
								<div class="text-neutral-500 text-xs mb-1">
									Region
								</div>
								<div class="text-white text-sm">
									{contract.region_name}
								</div>
							</div>
						{/if}
						{#if contract.requester_ssh_pubkey}
							<div
								class="bg-surface-elevated  p-3 border border-neutral-800"
							>
								<div class="text-neutral-500 text-xs mb-1">
									SSH Key
								</div>
								<div
									class="text-white text-sm font-mono truncate"
								>
									{formatSshKeyDisplay(
										contract.requester_ssh_pubkey,
									)}
								</div>
							</div>
						{/if}
						<div
							class="bg-surface-elevated  p-3 border border-neutral-800"
						>
							<div class="text-neutral-500 text-xs mb-1">
								Provider
							</div>
							<div class="flex items-center gap-2">
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
								<button
									onclick={(e) => {
										e.preventDefault();
										e.stopPropagation();
										contactProvider(contract.contract_id, contract.provider_pubkey);
									}}
									class="text-xs text-primary-400 hover:text-primary-300 transition-colors"
									title="Message the provider"
								>
									Contact
								</button>
							</div>
						</div>
					</div>

					{#if contract.request_memo}
						<div
							class="bg-surface-elevated  p-3 border border-neutral-800 mb-4"
						>
							<div class="text-neutral-500 text-xs mb-1">Memo</div>
							<div class="text-white text-sm">
								{contract.request_memo}
							</div>
						</div>
					{/if}

					{#if contract.gateway_subdomain && contract.gateway_ssh_port || contract.provisioning_instance_details}
						{@const instanceJson = (() => { try { return JSON.parse(contract.provisioning_instance_details ?? ''); } catch { return null; } })()}
						{@const gatewaySshCmd = contract.gateway_subdomain && contract.gateway_ssh_port ? `ssh -p ${contract.gateway_ssh_port} root@${contract.gateway_subdomain}` : null}
						{@const directSshCmd = instanceJson?.ip_address ? `ssh root@${instanceJson.ip_address}` : null}
						<div class="bg-green-500/10 border border-green-500/30 p-4 space-y-3">
							<div class="text-green-400 font-semibold text-sm">Connection Details</div>
							{#if gatewaySshCmd}
								<div>
									<div class="text-neutral-500 text-xs mb-1">Gateway SSH (recommended)</div>
									<div class="flex items-center gap-2">
										<code class="flex-1 bg-black/30 px-3 py-2 text-xs font-mono text-green-300 overflow-x-auto whitespace-nowrap">{gatewaySshCmd}</code>
										<button
											onclick={(e) => { e.preventDefault(); e.stopPropagation(); copyToClipboard(gatewaySshCmd, `gw-${contract.contract_id}`); }}
											class="shrink-0 px-2 py-2 text-xs bg-green-500/20 hover:bg-green-500/30 text-green-400 border border-green-500/30 transition-colors"
											title="Copy SSH command"
										>{copiedCommand === `gw-${contract.contract_id}` ? '✓' : 'Copy'}</button>
									</div>
								</div>
							{/if}
							{#if directSshCmd}
								<div>
									<div class="text-neutral-500 text-xs mb-1">Direct SSH</div>
									<div class="flex items-center gap-2">
										<code class="flex-1 bg-black/30 px-3 py-2 text-xs font-mono text-green-300 overflow-x-auto whitespace-nowrap">{directSshCmd}</code>
										<button
											onclick={(e) => { e.preventDefault(); e.stopPropagation(); copyToClipboard(directSshCmd, `ip-${contract.contract_id}`); }}
											class="shrink-0 px-2 py-2 text-xs bg-green-500/20 hover:bg-green-500/30 text-green-400 border border-green-500/30 transition-colors"
											title="Copy SSH command"
										>{copiedCommand === `ip-${contract.contract_id}` ? '✓' : 'Copy'}</button>
									</div>
								</div>
							{/if}
							{#if !gatewaySshCmd && !directSshCmd && contract.provisioning_instance_details}
								<div class="text-white text-xs font-mono whitespace-pre-wrap">{contract.provisioning_instance_details}</div>
							{/if}
							{#if contract.provisioning_completed_at_ns}
								<div class="text-green-400/60 text-xs">Provisioned: {formatDate(contract.provisioning_completed_at_ns)}</div>
							{/if}
						</div>
					{/if}
				</a>
			{:else}
				<div class="text-center py-10 text-neutral-500">
					{#if searchQuery.trim()}
						<p class="text-lg mb-2">No results for "{searchQuery.trim()}"</p>
						<button onclick={() => { searchQuery = ''; }} class="text-sm text-primary-400 hover:underline">Clear search</button>
					{:else if activeTab === 'active'}
						<p class="text-lg mb-2">No active rentals</p>
						<p class="text-sm">Provisioned resources appear here. <a href="/dashboard/marketplace" class="text-primary-400 hover:underline">Browse the marketplace</a> to rent one.</p>
					{:else if activeTab === 'pending'}
						<p class="text-lg mb-2">No pending requests</p>
						<p class="text-sm">Rental requests awaiting provider review appear here. <a href="/dashboard/marketplace" class="text-primary-400 hover:underline">Create a new request</a>.</p>
					{:else if activeTab === 'cancelled'}
						<p class="text-lg mb-2">No cancelled or failed rentals</p>
					{:else}
						<p class="text-lg mb-2">No rentals found</p>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>
