<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import {
		getSavedOfferings,
		unsaveOffering,
		hexEncode,
		type Offering
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

	onMount(async () => {
		const isAuth = await new Promise<boolean>((resolve) => {
			const unsub = authStore.isAuthenticated.subscribe((v) => {
				unsub();
				resolve(v);
			});
		});

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
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load saved offerings';
		} finally {
			loading = false;
		}
	});

	async function handleUnsave(offeringId: number) {
		const info = await authStore.getSigningIdentity();
		if (!info || !(info.identity instanceof Ed25519KeyIdentity)) return;
		const pubkeyHex = hexEncode(info.publicKeyBytes);
		// Optimistic update
		savedIds = toggleSavedId(savedIds, offeringId);
		offerings = offerings.filter((o) => o.id !== offeringId);
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
	<div>
		<h1 class="text-2xl font-bold text-white tracking-tight flex items-center gap-2">
			<Icon name="bookmark" size={24} class="text-primary-400" />
			Saved Offerings
		</h1>
		<p class="text-neutral-500 text-sm mt-1">Offerings you've saved for later</p>
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
			{#each offerings as offering (offering.id)}
				<div class="card p-4 border border-neutral-800">
					<div class="flex items-start justify-between gap-4">
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
								{#if !offering.provider_online}
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
						<div class="flex items-center gap-3 shrink-0">
							<div class="text-right">
								<div class="font-medium text-white">{formatPrice(offering)}</div>
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
