<script lang="ts">
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';
	import { goto } from '$app/navigation';
	import {
		getSavedOfferings,
		getUserNotifications,
		markNotificationsRead,
		unsaveOffering,
		hexEncode,
		type Offering,
		type UserNotification
	} from '$lib/services/api';
	import { toggleSavedId } from '$lib/services/saved-offerings';
	import Icon from '$lib/components/Icons.svelte';
	import TrustBadge from '$lib/components/TrustBadge.svelte';
	import { authStore } from '$lib/stores/auth';
	import { signRequest } from '$lib/services/auth-api';
	import { Ed25519KeyIdentity } from '@dfinity/identity';
	import { truncatePubkey } from '$lib/utils/identity';

	let offerings = $state<Offering[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let savedIds = $state(new Set<number>());
	let selectedIds = $state(new Set<number>());
	let removing = $state(false);
	let priceChangeMap = $state(new Map<number, { direction: 'up' | 'down'; notificationId: number }>());

	let allSelected = $derived(offerings.length > 0 && offerings.every(o => o.id !== undefined && selectedIds.has(o.id)));
	let someSelected = $derived(selectedIds.size > 0);

	onMount(async () => {
		const isAuth = get(authStore.isAuthenticated);

		if (!isAuth) {
			goto('/dashboard/login?redirect=/dashboard/saved');
			return;
		}

		try {
			const info = await authStore.getSigningIdentity();
			if (!info || !(info.identity instanceof Ed25519KeyIdentity)) {
				goto('/dashboard/login?redirect=/dashboard/saved');
				return;
			}
			const pubkeyHex = hexEncode(info.publicKeyBytes);
			const { headers } = await signRequest(info.identity, 'GET', `/api/v1/users/${pubkeyHex}/saved-offerings`);
			offerings = await getSavedOfferings(headers, pubkeyHex);
			savedIds = new Set(offerings.map((o) => o.id).filter((id): id is number => id !== undefined));

			const notifHeaders = (await signRequest(info.identity, 'GET', `/api/v1/users/${pubkeyHex}/notifications`)).headers;
			const notifications = await getUserNotifications(notifHeaders, pubkeyHex);
			buildPriceChangeMap(notifications);

			const unreadPriceChangeIds = notifications
				.filter(n => n.notificationType === 'saved_offering_price_change' && n.readAt === undefined && n.offeringId !== undefined)
				.map(n => n.id);
			if (unreadPriceChangeIds.length > 0) {
				const markHeaders = (await signRequest(info.identity, 'POST', `/api/v1/users/${pubkeyHex}/notifications/mark-read`)).headers;
				await markNotificationsRead(markHeaders, pubkeyHex, unreadPriceChangeIds);
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load saved offerings';
		} finally {
			loading = false;
		}
	});

	function buildPriceChangeMap(notifications: UserNotification[]) {
		const map = new Map<number, { direction: 'up' | 'down'; notificationId: number }>();
		for (const n of notifications) {
			if (n.notificationType !== 'saved_offering_price_change' || n.offeringId === undefined || n.readAt !== undefined) continue;
			if (map.has(n.offeringId)) continue;
			const direction = n.title.includes('dropped') ? 'down' : 'up';
			map.set(n.offeringId, { direction, notificationId: n.id });
		}
		priceChangeMap = map;
	}

	async function handleUnsave(offeringId: number) {
		const info = await authStore.getSigningIdentity();
		if (!info || !(info.identity instanceof Ed25519KeyIdentity)) return;
		const pubkeyHex = hexEncode(info.publicKeyBytes);
		// Optimistic update
		savedIds = toggleSavedId(savedIds, offeringId);
		offerings = offerings.filter((o) => o.id !== offeringId);
		selectedIds = new Set([...selectedIds].filter(id => id !== offeringId));
		try {
			const { headers } = await signRequest(info.identity, 'DELETE', `/api/v1/users/${pubkeyHex}/saved-offerings/${offeringId}`);
			await unsaveOffering(headers, pubkeyHex, offeringId);
		} catch (err) {
			// Revert
			console.error('Failed to unsave offering:', err);
			// Reload to restore accurate state
			const { headers } = await signRequest(info.identity, 'GET', `/api/v1/users/${pubkeyHex}/saved-offerings`);
			offerings = await getSavedOfferings(headers, pubkeyHex);
			savedIds = new Set(offerings.map((o) => o.id).filter((id): id is number => id !== undefined));
		}
	}

	function toggleSelect(offeringId: number) {
		const newSet = new Set(selectedIds);
		if (newSet.has(offeringId)) {
			newSet.delete(offeringId);
		} else {
			newSet.add(offeringId);
		}
		selectedIds = newSet;
	}

	function toggleSelectAll() {
		if (allSelected) {
			selectedIds = new Set();
		} else {
			selectedIds = new Set(offerings.map(o => o.id).filter((id): id is number => id !== undefined));
		}
	}

	async function handleBulkRemove() {
		if (selectedIds.size === 0) return;
		const info = await authStore.getSigningIdentity();
		if (!info || !(info.identity instanceof Ed25519KeyIdentity)) return;
		const pubkeyHex = hexEncode(info.publicKeyBytes);

		removing = true;
		const idsToRemove = [...selectedIds];
		const failedIds: number[] = [];

		for (const offeringId of idsToRemove) {
			try {
				const { headers } = await signRequest(info.identity, 'DELETE', `/api/v1/users/${pubkeyHex}/saved-offerings/${offeringId}`);
				await unsaveOffering(headers, pubkeyHex, offeringId);
			} catch (err) {
				console.error('Failed to unsave offering:', offeringId, err);
				failedIds.push(offeringId);
			}
		}

		offerings = offerings.filter(o => o.id === undefined || !idsToRemove.includes(o.id) || failedIds.includes(o.id));
		savedIds = new Set(offerings.map(o => o.id).filter((id): id is number => id !== undefined));
		selectedIds = new Set(failedIds);
		removing = false;
	}

	function formatPrice(o: Offering): string {
		if (o.reseller_commission_percent && o.monthly_price) {
			const commission = o.monthly_price * (o.reseller_commission_percent / 100);
			return `${(o.monthly_price + commission).toFixed(2)} ${o.currency}`;
		}
		if (o.monthly_price) return `${o.monthly_price.toFixed(2)} ${o.currency}`;
		return 'On request';
	}

	function formatLocation(o: Offering): string {
		if (o.datacenter_city && o.datacenter_country) return `${o.datacenter_city}, ${o.datacenter_country}`;
		return o.datacenter_country || '—';
	}
</script>

<div class="space-y-6">
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-2xl font-bold text-white tracking-tight flex items-center gap-2">
				<Icon name="bookmark" size={24} class="text-primary-400" />
				Saved Offerings
			</h1>
			<p class="text-neutral-500 text-sm mt-1">Offerings you've saved for later</p>
		</div>
		<div class="flex items-center gap-3">
			{#if someSelected}
				<button
					onclick={handleBulkRemove}
					disabled={removing}
					class="inline-flex items-center gap-2 px-4 py-2 bg-red-600 hover:bg-red-500 disabled:bg-red-600/50 disabled:cursor-not-allowed text-white text-sm font-medium transition-colors"
				>
					{#if removing}
						<div class="animate-spin h-4 w-4 border-2 border-white/30 border-t-white rounded-full"></div>
					{:else}
						<Icon name="trash" size={16} />
					{/if}
					Remove {selectedIds.size} selected
				</button>
			{/if}
			{#if offerings.length >= 2}
				<a
					href="/dashboard/marketplace/compare?ids={offerings.filter(o => o.id !== undefined).slice(0, 3).map(o => o.id).join(',')}"
					class="inline-flex items-center gap-2 px-4 py-2 bg-primary-600 hover:bg-primary-500 text-white text-sm font-medium transition-colors"
				>
					<Icon name="list" size={16} />
					Compare Saved
				</a>
			{/if}
		</div>
	</div>

	{#if error}
		<div class="bg-danger/10 border border-danger/20 p-3 text-danger text-sm">{error}</div>
	{/if}

	{#if loading}
		<div class="flex justify-center py-12">
			<div class="animate-spin rounded-full h-10 w-10 border-t-2 border-b-2 border-primary-400"></div>
		</div>
	{:else if offerings.length === 0}
		<div class="text-center py-16 card">
			<div class="flex justify-center mb-4">
				<Icon name="bookmark" size={48} class="text-neutral-600" />
			</div>
			<p class="text-neutral-400 mb-2">No saved offerings yet.</p>
			<p class="text-neutral-500 text-sm mb-6">Browse the marketplace to save offerings for later.</p>
			<a
				href="/dashboard/marketplace"
				class="inline-flex items-center gap-2 px-5 py-2.5 bg-primary-600 hover:bg-primary-500 text-white font-semibold transition-colors"
			>
				<Icon name="cart" size={16} />
				Browse Marketplace
			</a>
		</div>
	{:else}
		<div class="space-y-3">
			{#if offerings.length > 0}
				<div class="flex items-center gap-3 px-1 py-2 border-b border-neutral-800">
					<label class="flex items-center gap-2 cursor-pointer">
						<input
							type="checkbox"
							checked={allSelected}
							onchange={toggleSelectAll}
							class="w-4 h-4 rounded border-neutral-600 bg-neutral-800 text-primary-500 focus:ring-primary-500 focus:ring-offset-0 cursor-pointer"
						/>
						<span class="text-sm text-neutral-400">Select all</span>
					</label>
					{#if someSelected}
						<span class="text-xs text-neutral-500">{selectedIds.size} selected</span>
					{/if}
				</div>
			{/if}
			{#each offerings as offering (offering.id)}
				<div class="card p-4 border border-neutral-800">
					<div class="flex items-start justify-between gap-4">
						<div class="flex items-start gap-3 flex-1 min-w-0">
							{#if offering.id !== undefined}
								<input
									type="checkbox"
									checked={selectedIds.has(offering.id)}
									onchange={() => toggleSelect(offering.id!)}
									class="w-4 h-4 mt-1 rounded border-neutral-600 bg-neutral-800 text-primary-500 focus:ring-primary-500 focus:ring-offset-0 cursor-pointer shrink-0"
								/>
							{:else}
								<div class="w-4 h-4 mt-1 shrink-0"></div>
							{/if}
							<div class="flex-1 min-w-0">
								<div class="flex items-center gap-2 flex-wrap mb-1">
									<a
										href="/dashboard/marketplace/{offering.id}"
										class="font-medium text-white hover:text-primary-400 transition-colors"
									>{offering.offer_name}</a>
									{#if offering.trust_score !== undefined}
										<TrustBadge
											score={offering.trust_score}
											hasFlags={offering.has_critical_flags ?? false}
											compact={true}
										/>
									{/if}
									{#if offering.provider_online === false}
										<span class="flex items-center gap-1 px-1.5 py-0.5 text-xs bg-red-500/20 text-red-400 rounded" title="Provider is not actively monitoring — requests are still accepted when agent comes back online">
											<span class="h-1.5 w-1.5 rounded-full bg-red-400"></span>
											Offline
										</span>
									{/if}
								</div>
								<a
									href="/dashboard/providers/{offering.owner_username || offering.pubkey}"
									class="text-xs text-neutral-500 hover:text-primary-400 {offering.owner_username ? '' : 'font-mono'}"
								>{offering.owner_username ? `@${offering.owner_username}` : truncatePubkey(offering.pubkey)}</a>
								<div class="text-sm text-neutral-400 mt-1">
									{offering.product_type} · {formatLocation(offering)}
								</div>
							</div>
						</div>
						<div class="flex items-center gap-3 shrink-0">
							<div class="text-right">
								<div class="font-medium text-white flex items-center justify-end gap-1.5">
									{formatPrice(offering)}
									{#if offering.id !== undefined && priceChangeMap.has(offering.id)}
										{@const change = priceChangeMap.get(offering.id)}
										{#if change?.direction === 'down'}
											<span class="inline-flex items-center px-1.5 py-0.5 text-xs font-semibold bg-emerald-500/20 text-emerald-400 rounded" title="Price dropped">
												↓
											</span>
										{:else}
											<span class="inline-flex items-center px-1.5 py-0.5 text-xs font-semibold bg-amber-500/20 text-amber-400 rounded" title="Price increased">
												↑
											</span>
										{/if}
									{/if}
								</div>
								<div class="text-xs text-neutral-500">/month</div>
							</div>
							<a
								href="/dashboard/marketplace/{offering.id}"
								class="px-3 py-1.5 bg-primary-600 hover:bg-primary-500 text-white text-xs font-medium transition-colors"
							>View</a>
							{#if offering.id !== undefined}
								<button
									onclick={() => handleUnsave(offering.id!)}
									title="Remove from saved"
									class="p-1.5 text-primary-400 hover:text-red-400 transition-colors"
								>
									<Icon name="bookmark" size={16} />
								</button>
							{/if}
						</div>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
