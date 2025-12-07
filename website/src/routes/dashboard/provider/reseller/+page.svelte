<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { page } from "$app/stores";
	import { navigateToLogin } from "$lib/utils/navigation";
	import {
		getExternalProviders,
		getResellerRelationships,
		createResellerRelationship,
		updateResellerRelationship,
		deleteResellerRelationship,
		getResellerOrders,
		fulfillResellerOrder,
		getProviderOfferings,
		hexEncode,
		type ExternalProvider,
		type ResellerRelationship,
		type ResellerOrder,
		type CreateResellerRelationshipParams,
		type UpdateResellerRelationshipParams,
		type FulfillResellerOrderParams,
		type Offering,
	} from "$lib/services/api";
	import { signRequest } from "$lib/services/auth-api";
	import { authStore } from "$lib/stores/auth";
	import { Ed25519KeyIdentity } from "@dfinity/identity";

	let externalProviders = $state<ExternalProvider[]>([]);
	let relationships = $state<ResellerRelationship[]>([]);
	let orders = $state<ResellerOrder[]>([]);
	let offerings = $state<Record<number, Offering>>({});
	let loading = $state(true);
	let error = $state<string | null>(null);
	let successMessage = $state<string | null>(null);
	let isAuthenticated = $state(false);
	let unsubscribeAuth: (() => void) | null = null;
	let providerHex = $state("");

	// Track which provider is being added as reseller
	let creatingRelationship = $state<Record<string, boolean>>({});
	let commissionInputs = $state<Record<string, number>>({});

	// Track editing state
	let editingRelationship = $state<Record<string, boolean>>({});
	let editCommissionInputs = $state<Record<string, number>>({});
	let deletingRelationship = $state<Record<string, boolean>>({});

	// Order fulfillment
	let orderStatusFilter = $state<string>('pending');
	let fulfillModalOpen = $state(false);
	let selectedOrder = $state<ResellerOrder | null>(null);
	let externalOrderId = $state('');
	let externalOrderDetails = $state('');
	let fulfillingOrder = $state(false);

	type SigningIdentity = {
		identity: Ed25519KeyIdentity;
		publicKeyBytes: Uint8Array;
	};

	let signingIdentityInfo = $state<SigningIdentity | null>(null);

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
		if (!isAuthenticated) {
			loading = false;
			return;
		}

		try {
			loading = true;
			error = null;
			const info = await authStore.getSigningIdentity();
			if (!info) {
				error = "You must be authenticated to manage reseller relationships";
				return;
			}
			if (!(info.identity instanceof Ed25519KeyIdentity)) {
				error = "Only Ed25519 identities can sign reseller actions";
				return;
			}
			const normalizedIdentity: SigningIdentity = {
				identity: info.identity,
				publicKeyBytes: info.publicKeyBytes,
			};
			signingIdentityInfo = normalizedIdentity;
			providerHex = hexEncode(normalizedIdentity.publicKeyBytes);

			// Fetch external providers (no auth required - public endpoint)
			externalProviders = await getExternalProviders();

			// Fetch current relationships (requires auth)
			const relationshipsSigned = await signRequest(
				normalizedIdentity.identity,
				"GET",
				"/api/v1/reseller/relationships",
			);
			relationships = await getResellerRelationships(relationshipsSigned.headers);

			// Fetch orders
			await loadOrders();
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load reseller data";
		} finally {
			loading = false;
		}
	}

	async function loadOrders() {
		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) return;

		try {
			const signed = await signRequest(
				activeIdentity.identity,
				"GET",
				orderStatusFilter === 'all'
					? "/api/v1/reseller/orders"
					: `/api/v1/reseller/orders?status=${orderStatusFilter}`,
			);
			orders = await getResellerOrders(signed.headers, orderStatusFilter === 'all' ? undefined : orderStatusFilter);

			// Fetch offering details for all unique offering IDs
			const offeringIds = [...new Set(orders.map(o => o.offering_id))];
			const offeringsMap: Record<number, Offering> = {};

			for (const offeringId of offeringIds) {
				// Find the external provider pubkey for this offering
				const order = orders.find(o => o.offering_id === offeringId);
				if (!order) continue;

				try {
					const providerOfferings = await getProviderOfferings(order.external_provider_pubkey);
					const offering = providerOfferings.find(o => o.id === offeringId);
					if (offering) {
						offeringsMap[offeringId] = offering;
					}
				} catch (e) {
					console.error(`Failed to load offering ${offeringId}:`, e);
				}
			}

			offerings = offeringsMap;
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load orders";
		}
	}

	function handleLogin() {
		navigateToLogin($page.url.pathname);
	}

	function getCommissionValue(pubkey: string): number {
		return commissionInputs[pubkey] ?? 10; // Default 10%
	}

	function getEditCommissionValue(pubkey: string): number {
		return editCommissionInputs[pubkey] ?? 10;
	}

	function isAlreadyReseller(providerPubkey: string): boolean {
		return relationships.some(
			(r) => r.external_provider_pubkey === providerPubkey && r.status === "active"
		);
	}

	async function handleBecomeReseller(provider: ExternalProvider) {
		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) {
			error = "Missing signing identity";
			return;
		}

		const commission = getCommissionValue(provider.pubkey);
		if (commission < 0 || commission > 50) {
			error = "Commission must be between 0 and 50%";
			return;
		}

		error = null;
		successMessage = null;
		creatingRelationship = { ...creatingRelationship, [provider.pubkey]: true };

		try {
			const payload: CreateResellerRelationshipParams = {
				external_provider_pubkey: provider.pubkey,
				commission_percent: commission,
			};

			const signed = await signRequest(
				activeIdentity.identity,
				"POST",
				"/api/v1/reseller/relationships",
				payload,
			);

			await createResellerRelationship(payload, signed.headers);
			successMessage = `Successfully became reseller for ${provider.name} with ${commission}% commission`;
			await loadData();
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to create reseller relationship";
		} finally {
			creatingRelationship = { ...creatingRelationship, [provider.pubkey]: false };
		}
	}

	function startEdit(relationship: ResellerRelationship) {
		editingRelationship = { ...editingRelationship, [relationship.external_provider_pubkey]: true };
		editCommissionInputs = { ...editCommissionInputs, [relationship.external_provider_pubkey]: relationship.commission_percent };
	}

	function cancelEdit(providerPubkey: string) {
		editingRelationship = { ...editingRelationship, [providerPubkey]: false };
	}

	async function handleUpdateRelationship(relationship: ResellerRelationship) {
		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) {
			error = "Missing signing identity";
			return;
		}

		const newCommission = getEditCommissionValue(relationship.external_provider_pubkey);
		if (newCommission < 0 || newCommission > 50) {
			error = "Commission must be between 0 and 50%";
			return;
		}

		error = null;
		successMessage = null;

		try {
			const payload: UpdateResellerRelationshipParams = {
				commission_percent: newCommission,
			};

			const signed = await signRequest(
				activeIdentity.identity,
				"PUT",
				`/api/v1/reseller/relationships/${relationship.external_provider_pubkey}`,
				payload,
			);

			await updateResellerRelationship(relationship.external_provider_pubkey, payload, signed.headers);
			successMessage = `Updated commission to ${newCommission}%`;
			editingRelationship = { ...editingRelationship, [relationship.external_provider_pubkey]: false };
			await loadData();
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to update relationship";
		}
	}

	async function handleDeleteRelationship(relationship: ResellerRelationship) {
		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) {
			error = "Missing signing identity";
			return;
		}

		if (!confirm(`Are you sure you want to stop reselling for this provider?`)) {
			return;
		}

		error = null;
		successMessage = null;
		deletingRelationship = { ...deletingRelationship, [relationship.external_provider_pubkey]: true };

		try {
			const signed = await signRequest(
				activeIdentity.identity,
				"DELETE",
				`/api/v1/reseller/relationships/${relationship.external_provider_pubkey}`,
			);

			await deleteResellerRelationship(relationship.external_provider_pubkey, signed.headers);
			successMessage = "Reseller relationship deleted";
			await loadData();
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to delete relationship";
		} finally {
			deletingRelationship = { ...deletingRelationship, [relationship.external_provider_pubkey]: false };
		}
	}

	function getProviderName(providerPubkey: string): string {
		const provider = externalProviders.find(p => p.pubkey === providerPubkey);
		return provider?.name ?? providerPubkey.substring(0, 8);
	}

	function openFulfillModal(order: ResellerOrder) {
		selectedOrder = order;
		externalOrderId = '';
		externalOrderDetails = '';
		fulfillModalOpen = true;
	}

	function closeFulfillModal() {
		fulfillModalOpen = false;
		selectedOrder = null;
		externalOrderId = '';
		externalOrderDetails = '';
	}

	async function handleFulfillOrder() {
		if (!selectedOrder || !signingIdentityInfo) return;

		if (!externalOrderId.trim()) {
			error = "External order ID is required";
			return;
		}

		fulfillingOrder = true;
		error = null;
		successMessage = null;

		try {
			const params: FulfillResellerOrderParams = {
				external_order_id: externalOrderId.trim(),
				external_order_details: externalOrderDetails.trim() || undefined,
			};

			const signed = await signRequest(
				signingIdentityInfo.identity,
				"POST",
				`/api/v1/reseller/orders/${selectedOrder.contract_id}/fulfill`,
				params,
			);

			await fulfillResellerOrder(selectedOrder.contract_id, params, signed.headers);
			successMessage = "Order fulfilled successfully";
			closeFulfillModal();
			await loadOrders();
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to fulfill order";
		} finally {
			fulfillingOrder = false;
		}
	}

	async function handleStatusFilterChange(newStatus: string) {
		orderStatusFilter = newStatus;
		await loadOrders();
	}

	function formatPrice(priceE9s: number): string {
		return (priceE9s / 1e9).toFixed(2);
	}

	function formatDate(timestampNs?: number): string {
		if (!timestampNs) return 'N/A';
		return new Date(timestampNs / 1000000).toLocaleString();
	}

	onDestroy(() => {
		unsubscribeAuth?.();
	});
</script>

<div class="space-y-8">
	<header>
		<h1 class="text-4xl font-bold text-white mb-2">Reseller Program</h1>
		<p class="text-white/60">
			Become a reseller for external providers and earn commission on each order
		</p>
	</header>

	{#if !isAuthenticated}
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-8 border border-white/20 text-center">
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">💼</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-white/70">
					Create an account or login to become a reseller and manage your reseller relationships.
				</p>
				<button
					onclick={handleLogin}
					class="px-8 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 hover:scale-105 transition-all"
				>
					Login / Create Account
				</button>
			</div>
		</div>
	{:else}
		{#if error}
			<div class="bg-red-500/20 border border-red-500/30 rounded-lg p-4 text-red-300">
				{error}
			</div>
		{/if}
		{#if successMessage}
			<div class="bg-emerald-500/15 border border-emerald-500/30 rounded-lg p-4 text-emerald-300">
				{successMessage}
			</div>
		{/if}

		{#if loading}
			<div class="flex justify-center items-center py-12">
				<div class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-400"></div>
			</div>
		{:else}
			<!-- Your Reseller Relationships -->
			<section class="space-y-4">
				<div class="flex items-center justify-between">
					<h2 class="text-2xl font-semibold text-white">Your Reseller Relationships</h2>
					<span class="text-white/60 text-sm">{relationships.length} active</span>
				</div>

				{#if relationships.length === 0}
					<div class="bg-white/5 border border-white/10 rounded-xl p-6 text-white/70">
						You are not reselling for any providers yet. Browse the available providers below to get started.
					</div>
				{:else}
					<div class="space-y-3">
						{#each relationships as relationship}
							<div class="bg-white/5 border border-white/10 rounded-xl p-6 hover:bg-white/10 transition-colors">
								<div class="flex items-center justify-between">
									<div class="flex-1">
										<h3 class="text-lg font-semibold text-white">{getProviderName(relationship.external_provider_pubkey)}</h3>
										{#if editingRelationship[relationship.external_provider_pubkey]}
											<div class="mt-2 flex items-center gap-2">
												<input
													type="number"
													min="0"
													max="50"
													bind:value={editCommissionInputs[relationship.external_provider_pubkey]}
													class="w-24 px-3 py-1.5 bg-white/10 border border-white/20 rounded-lg text-white focus:outline-none focus:border-blue-400"
												/>
												<span class="text-white/70">% commission</span>
											</div>
										{:else}
											<p class="text-white/60 mt-1">{relationship.commission_percent}% commission · {relationship.status}</p>
										{/if}
									</div>
									<div class="flex items-center gap-2">
										{#if editingRelationship[relationship.external_provider_pubkey]}
											<button
												onclick={() => handleUpdateRelationship(relationship)}
												class="px-4 py-2 bg-emerald-500/20 border border-emerald-500/30 rounded-lg text-emerald-300 hover:bg-emerald-500/30 transition-colors"
											>
												Save
											</button>
											<button
												onclick={() => cancelEdit(relationship.external_provider_pubkey)}
												class="px-4 py-2 bg-white/10 border border-white/20 rounded-lg text-white/70 hover:bg-white/20 transition-colors"
											>
												Cancel
											</button>
										{:else}
											<button
												onclick={() => startEdit(relationship)}
												class="px-4 py-2 bg-blue-500/20 border border-blue-500/30 rounded-lg text-blue-300 hover:bg-blue-500/30 transition-colors"
											>
												Edit
											</button>
											<button
												onclick={() => handleDeleteRelationship(relationship)}
												disabled={deletingRelationship[relationship.external_provider_pubkey]}
												class="px-4 py-2 bg-red-500/20 border border-red-500/30 rounded-lg text-red-300 hover:bg-red-500/30 transition-colors disabled:opacity-50"
											>
												{deletingRelationship[relationship.external_provider_pubkey] ? "Deleting..." : "Delete"}
											</button>
										{/if}
									</div>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</section>

			<!-- Pending Orders -->
			<section class="space-y-4">
				<div class="flex items-center justify-between">
					<h2 class="text-2xl font-semibold text-white">Reseller Orders</h2>
					<div class="flex items-center gap-2">
						<span class="text-white/60 text-sm">{orders.length} orders</span>
						<select
							bind:value={orderStatusFilter}
							onchange={() => handleStatusFilterChange(orderStatusFilter)}
							class="px-3 py-1.5 bg-white/10 border border-white/20 rounded-lg text-white text-sm focus:outline-none focus:border-blue-400"
						>
							<option value="pending">Pending</option>
							<option value="fulfilled">Fulfilled</option>
							<option value="all">All</option>
						</select>
					</div>
				</div>

				{#if orders.length === 0}
					<div class="bg-white/5 border border-white/10 rounded-xl p-6 text-white/70">
						No {orderStatusFilter === 'all' ? '' : orderStatusFilter} orders found.
					</div>
				{:else}
					<div class="space-y-3">
						{#each orders as order}
							{@const offering = offerings[order.offering_id]}
							{@const providerName = getProviderName(order.external_provider_pubkey)}
							<div class="bg-white/5 border border-white/10 rounded-xl p-6 hover:bg-white/10 transition-colors">
								<div class="flex items-start justify-between">
									<div class="flex-1 space-y-2">
										<div class="flex items-center gap-2">
											<h3 class="text-lg font-semibold text-white">
												Order: {order.contract_id.substring(0, 12)}...
											</h3>
											<span class="px-2 py-1 text-xs rounded {order.status === 'fulfilled' ? 'bg-emerald-500/20 text-emerald-300' : 'bg-yellow-500/20 text-yellow-300'}">
												{order.status}
											</span>
										</div>
										<p class="text-white/70">
											{offering ? offering.offer_name : `Offering ID: ${order.offering_id}`} ({providerName})
										</p>
										<div class="flex items-center gap-4 text-sm text-white/60">
											<span>Base: ${formatPrice(order.base_price_e9s)}</span>
											<span>Commission: ${formatPrice(order.commission_e9s)} ({Math.round(order.commission_e9s / order.base_price_e9s * 100)}%)</span>
											<span class="font-semibold text-white">Total: ${formatPrice(order.total_paid_e9s)}</span>
										</div>
										{#if order.external_order_id}
											<p class="text-white/60 text-sm">External Order: {order.external_order_id}</p>
										{/if}
										{#if order.fulfilled_at_ns}
											<p class="text-white/50 text-xs">Fulfilled: {formatDate(order.fulfilled_at_ns)}</p>
										{/if}
									</div>
									<div class="ml-4">
										{#if order.status === 'pending'}
											<button
												onclick={() => openFulfillModal(order)}
												class="px-4 py-2 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg text-white font-semibold hover:brightness-110 transition-all"
											>
												Fulfill Order
											</button>
										{:else if order.external_order_details}
											<button
												onclick={() => {
													selectedOrder = order;
													externalOrderDetails = order.external_order_details || '';
													fulfillModalOpen = true;
												}}
												class="px-4 py-2 bg-white/10 border border-white/20 rounded-lg text-white/70 hover:bg-white/20 transition-colors"
											>
												View Details
											</button>
										{/if}
									</div>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</section>

			<!-- Available External Providers -->
			<section class="space-y-4">
				<div class="flex items-center justify-between">
					<h2 class="text-2xl font-semibold text-white">Available External Providers</h2>
					<span class="text-white/60 text-sm">{externalProviders.length} providers</span>
				</div>

				{#if externalProviders.length === 0}
					<div class="bg-white/5 border border-white/10 rounded-xl p-6 text-white/70">
						No external providers available at this time.
					</div>
				{:else}
					<div class="space-y-3">
						{#each externalProviders as provider}
							{@const alreadyReseller = isAlreadyReseller(provider.pubkey)}
							<div class="bg-white/5 border border-white/10 rounded-xl p-6 hover:bg-white/10 transition-colors">
								<div class="flex items-center justify-between">
									<div class="flex-1">
										<div class="flex items-center gap-3">
											<h3 class="text-lg font-semibold text-white">{provider.name}</h3>
											{#if provider.logo_url}
												<img src={provider.logo_url} alt="{provider.name} logo" class="w-6 h-6 rounded" />
											{/if}
										</div>
										<p class="text-white/60 mt-1">
											{provider.offerings_count} offerings · {provider.domain}
										</p>
										{#if provider.website_url}
											<a href={provider.website_url} target="_blank" rel="noopener noreferrer" class="text-blue-400 hover:text-blue-300 text-sm mt-1 inline-block">
												{provider.website_url} ↗
											</a>
										{/if}
									</div>
									<div class="flex items-center gap-3">
										{#if !alreadyReseller}
											<div class="flex items-center gap-2">
												<input
													type="number"
													min="0"
													max="50"
													bind:value={commissionInputs[provider.pubkey]}
													placeholder="10"
													class="w-20 px-3 py-2 bg-white/10 border border-white/20 rounded-lg text-white focus:outline-none focus:border-blue-400"
												/>
												<span class="text-white/70 text-sm">%</span>
											</div>
											<button
												onclick={() => handleBecomeReseller(provider)}
												disabled={creatingRelationship[provider.pubkey]}
												class="px-6 py-2 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg text-white font-semibold hover:brightness-110 transition-all disabled:opacity-50"
											>
												{creatingRelationship[provider.pubkey] ? "Creating..." : "Become Reseller"}
											</button>
										{:else}
											<span class="px-4 py-2 bg-emerald-500/20 border border-emerald-500/30 rounded-lg text-emerald-300">
												✓ Active Reseller
											</span>
										{/if}
									</div>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</section>
		{/if}
	{/if}
</div>

<!-- Fulfill Order Modal -->
{#if fulfillModalOpen && selectedOrder}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50" onclick={closeFulfillModal}>
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="bg-gradient-to-br from-slate-900 to-slate-800 border border-white/20 rounded-2xl p-8 max-w-2xl w-full mx-4 shadow-2xl" onclick={(e) => e.stopPropagation()}>
			<div class="flex items-center justify-between mb-6">
				<h3 class="text-2xl font-bold text-white">
					{selectedOrder.status === 'pending' ? 'Fulfill Order' : 'Order Details'}
				</h3>
				<button onclick={closeFulfillModal} aria-label="Close modal" class="text-white/60 hover:text-white transition-colors">
					<svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
					</svg>
				</button>
			</div>

			<div class="space-y-6">
				<!-- Order Summary -->
				<div class="bg-white/5 border border-white/10 rounded-lg p-4">
					<p class="text-white/60 text-sm mb-1">Order ID</p>
					<p class="text-white font-mono">{selectedOrder.contract_id}</p>
				</div>

				{#if selectedOrder.status === 'pending'}
					<!-- External Order ID Input -->
					<div>
						<label for="external-order-id" class="block text-white/80 text-sm font-semibold mb-2">
							External Order ID <span class="text-red-400">*</span>
						</label>
						<input
							id="external-order-id"
							type="text"
							bind:value={externalOrderId}
							placeholder="e.g., HZN-12345678"
							class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/40 focus:outline-none focus:border-blue-400 transition-colors"
						/>
					</div>

					<!-- Instance Details Textarea -->
					<div>
						<label for="instance-details" class="block text-white/80 text-sm font-semibold mb-2">
							Instance Details (JSON)
						</label>
						<textarea
							id="instance-details"
							bind:value={externalOrderDetails}
							placeholder={'{\n  "ip": "1.2.3.4",\n  "username": "root",\n  "password": "..."\n}'}
							rows="8"
							class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/40 focus:outline-none focus:border-blue-400 transition-colors font-mono text-sm"
						></textarea>
						<p class="text-white/50 text-xs mt-1">Optional: Add instance details like IP, credentials, etc. in JSON format</p>
					</div>

					<!-- Action Buttons -->
					<div class="flex items-center justify-end gap-3 pt-4">
						<button
							onclick={closeFulfillModal}
							disabled={fulfillingOrder}
							class="px-6 py-3 bg-white/10 border border-white/20 rounded-lg text-white/70 hover:bg-white/20 transition-colors disabled:opacity-50"
						>
							Cancel
						</button>
						<button
							onclick={handleFulfillOrder}
							disabled={fulfillingOrder || !externalOrderId.trim()}
							class="px-6 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg text-white font-semibold hover:brightness-110 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
						>
							{fulfillingOrder ? "Fulfilling..." : "Mark as Fulfilled"}
						</button>
					</div>
				{:else}
					<!-- View-only mode for fulfilled orders -->
					{#if selectedOrder.external_order_id}
						<div>
							<p class="block text-white/80 text-sm font-semibold mb-2">External Order ID</p>
							<p class="px-4 py-3 bg-white/5 border border-white/10 rounded-lg text-white">{selectedOrder.external_order_id}</p>
						</div>
					{/if}

					{#if externalOrderDetails}
						<div>
							<p class="block text-white/80 text-sm font-semibold mb-2">Instance Details</p>
							<pre class="px-4 py-3 bg-white/5 border border-white/10 rounded-lg text-white font-mono text-sm overflow-auto max-h-64">{externalOrderDetails}</pre>
						</div>
					{/if}

					<div class="flex justify-end pt-4">
						<button
							onclick={closeFulfillModal}
							class="px-6 py-3 bg-white/10 border border-white/20 rounded-lg text-white/70 hover:bg-white/20 transition-colors"
						>
							Close
						</button>
					</div>
				{/if}
			</div>
		</div>
	</div>
{/if}
