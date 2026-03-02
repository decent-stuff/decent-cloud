<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { getOffering, fetchIcpPrice, type Offering } from '$lib/services/api';
	import { removeFromComparison } from '$lib/utils/compare';
	import {
		buildComparePath,
		copyCompareShareUrl,
		normalizeCompareIds,
	} from '$lib/utils/compare-share';
	import { truncatePubkey } from '$lib/utils/identity';
	import RentalRequestDialog from '$lib/components/RentalRequestDialog.svelte';
	import AuthPromptModal from '$lib/components/AuthPromptModal.svelte';
	import TrustBadge from '$lib/components/TrustBadge.svelte';
	import Icon from '$lib/components/Icons.svelte';
	import Breadcrumb from '$lib/components/Breadcrumb.svelte';
	import { authStore } from '$lib/stores/auth';

	// IDs come from ?ids=1,2,3 — validated and capped in compare-share util
	const rawIds = $page.url.searchParams.get('ids') ?? '';
	const offeringIds: number[] = normalizeCompareIds(rawIds);

	let offerings = $state<Offering[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let icpPriceUsd = $state<number | null>(null);
	let selectedOffering = $state<Offering | null>(null);
	let showAuthModal = $state(false);
	let successMessage = $state<string | null>(null);
	let shareErrorMessage = $state<string | null>(null);
	let isAuthenticated = $state(false);

	authStore.isAuthenticated.subscribe((v) => { isAuthenticated = v; });

	onMount(async () => {
		if (offeringIds.length < 2) {
			goto('/dashboard/marketplace');
			return;
		}

		const canonicalPath = buildComparePath(offeringIds);
		if (`${$page.url.pathname}${$page.url.search}` !== canonicalPath) {
			goto(canonicalPath, { replaceState: true, noScroll: true, keepFocus: true });
		}
		try {
			[offerings, icpPriceUsd] = await Promise.all([
				Promise.all(offeringIds.map((id) => getOffering(id))),
				fetchIcpPrice()
			]);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load offerings';
		} finally {
			loading = false;
		}
	});

	function removeOffering(id: number) {
		const remaining = removeFromComparison(new Set(offeringIds), id);
		if (remaining.size < 2) {
			goto('/dashboard/marketplace');
			return;
		}
		goto(buildComparePath(remaining), { replaceState: true });
	}

	async function shareComparison() {
		successMessage = null;
		shareErrorMessage = null;

		try {
			await copyCompareShareUrl({
				ids: offeringIds,
				origin: window.location.origin,
				clipboard: navigator.clipboard,
			});
			successMessage = 'Comparison link copied to clipboard';
		} catch (e) {
			shareErrorMessage =
				e instanceof Error ? `Failed to copy comparison link: ${e.message}` : 'Failed to copy comparison link';
		}
	}

	function handleRentClick(offering: Offering) {
		if (!isAuthenticated) {
			showAuthModal = true;
			return;
		}
		selectedOffering = offering;
	}

	function handleRentalSuccess(contractId: string) {
		selectedOffering = null;
		// Navigate to contract detail page with welcome state
		goto(`/dashboard/rentals/${contractId}?welcome=true`);
	}

	// Formatting helpers (same logic as marketplace page)
	function formatPrice(o: Offering): string {
		if (o.reseller_commission_percent && o.monthly_price) {
			const commission = o.monthly_price * (o.reseller_commission_percent / 100);
			return `${(o.monthly_price + commission).toFixed(2)} ${o.currency}`;
		}
		if (o.monthly_price) return `${o.monthly_price.toFixed(2)} ${o.currency}`;
		return 'On request';
	}

	function formatUsdEquivalent(o: Offering): string | null {
		if (!icpPriceUsd || !o.monthly_price || o.currency?.toUpperCase() !== 'ICP') return null;
		let price = o.monthly_price;
		if (o.reseller_commission_percent) price += price * (o.reseller_commission_percent / 100);
		return `≈ $${(price * icpPriceUsd).toFixed(2)}/mo`;
	}

	function formatBilling(o: Offering): string {
		const interval = o.billing_interval?.toLowerCase() || '';
		if (interval.includes('hour')) return 'Hourly';
		if (interval.includes('day')) return 'Daily';
		if (interval.includes('month')) return 'Monthly';
		if (interval.includes('year')) return 'Yearly';
		return o.billing_interval || '—';
	}

	function formatContractTerms(o: Offering): string {
		const parts: string[] = [];
		if (o.min_contract_hours) {
			const h = o.min_contract_hours;
			if (h >= 720) parts.push(`Min ${Math.round(h / 720)}mo`);
			else if (h >= 24) parts.push(`Min ${Math.round(h / 24)}d`);
			else parts.push(`Min ${h}h`);
		}
		if (o.max_contract_hours) {
			const h = o.max_contract_hours;
			if (h >= 720) parts.push(`Max ${Math.round(h / 720)}mo`);
			else if (h >= 24) parts.push(`Max ${Math.round(h / 24)}d`);
			else parts.push(`Max ${h}h`);
		}
		return parts.length > 0 ? parts.join(' · ') : '—';
	}

	function formatLocation(o: Offering): string {
		if (o.datacenter_city && o.datacenter_country) return `${o.datacenter_city}, ${o.datacenter_country}`;
		return o.datacenter_country || '—';
	}

	function parseRamGb(memory_amount: string | undefined): number | null {
		if (!memory_amount) return null;
		const m = memory_amount.match(/^([\d.]+)\s*(GB|GiB|MB|MiB|TB|TiB)/i);
		if (!m) return null;
		const val = parseFloat(m[1]);
		const unit = m[2].toUpperCase();
		if (unit === 'MB' || unit === 'MIB') return val / 1024;
		if (unit === 'TB' || unit === 'TIB') return val * 1024;
		return val;
	}

	function findWinner<T>(
		items: { id: number | undefined; val: T | null | undefined }[],
		best: (a: T, b: T) => boolean
	): number | null {
		const valid = items.filter((x) => x.id !== undefined && x.val !== null && x.val !== undefined) as { id: number; val: T }[];
		if (valid.length < 2) return null;
		let winner = valid[0];
		for (const item of valid.slice(1)) {
			if (best(item.val, winner.val)) winner = item;
		}
		const topVal = winner.val;
		const topCount = valid.filter((x) => !best(topVal, x.val) && !best(x.val, topVal)).length;
		return topCount === 1 ? winner.id : null;
	}

	const winners = $derived({
		price: findWinner(
			offerings.map((o) => ({ id: o.id, val: o.monthly_price ?? null })),
			(a: number, b: number) => a < b
		),
		cores: findWinner(
			offerings.map((o) => ({ id: o.id, val: o.processor_cores ?? null })),
			(a: number, b: number) => a > b
		),
		ram: findWinner(
			offerings.map((o) => ({ id: o.id, val: parseRamGb(o.memory_amount) })),
			(a: number, b: number) => a > b
		),
		trust: findWinner(
			offerings.map((o) => ({ id: o.id, val: o.trust_score ?? null })),
			(a: number, b: number) => a > b
		),
		reliability: findWinner(
			offerings.map((o) => ({ id: o.id, val: o.reliability_score ?? null })),
			(a: number, b: number) => a > b
		),
	});
</script>

<div class="space-y-6">
	<Breadcrumb items={[
		isAuthenticated
			? { label: 'Dashboard', href: '/dashboard' }
			: { label: 'Home', href: '/' },
		{ label: 'Marketplace', href: '/dashboard/marketplace' },
		{ label: 'Compare' },
	]} />

	<div>
		<h1 class="text-2xl font-bold text-white tracking-tight">Compare Offerings</h1>
		<div class="mt-1 flex flex-wrap items-center gap-3 text-sm">
			<p class="text-neutral-500">Side-by-side comparison · <a href="/dashboard/marketplace" class="text-primary-400 hover:text-primary-300 transition-colors">Back to marketplace</a></p>
			<button
				onclick={shareComparison}
				class="inline-flex items-center gap-1.5 text-neutral-400 hover:text-primary-400 transition-colors"
				title="Copy shareable comparison URL"
			>
				<Icon name="link" size={14} /> Share comparison
			</button>
		</div>
	</div>

	{#if successMessage}
		<div class="bg-success/10 border border-success/20 p-3 text-success text-sm">
			{successMessage}
		</div>
	{/if}

	{#if shareErrorMessage}
		<div class="bg-red-500/10 border border-red-500/30 p-3 text-red-300 text-sm">
			{shareErrorMessage}
		</div>
	{/if}

	{#if error}
		<div class="bg-red-500/20 border border-red-500/30 p-6 text-center">
			<h2 class="text-xl font-bold text-red-400 mb-2">Failed to load offerings</h2>
			<p class="text-neutral-400 mb-4">{error}</p>
			<a href="/dashboard/marketplace" class="inline-block px-6 py-3 bg-surface-elevated border border-neutral-700 font-semibold hover:border-neutral-500 transition-all">
				Back to Marketplace
			</a>
		</div>
	{:else if loading}
		<div class="flex justify-center items-center p-12">
			<div class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-primary-400"></div>
		</div>
	{:else if offerings.length >= 2}
		<!-- Comparison table — horizontally scrollable on mobile -->
		<div class="overflow-x-auto">
			<table class="w-full text-sm border-collapse min-w-[600px]">
				<!-- Header: one column per offering -->
				<thead>
					<tr class="border-b border-neutral-800">
						<th class="px-4 py-3 text-left text-neutral-500 font-medium w-36 shrink-0">Spec</th>
						{#each offerings as offering}
							<th class="px-4 py-3 text-left align-top min-w-[180px]">
								<div class="flex items-start justify-between gap-2">
									<div class="min-w-0">
										<a
											href="/dashboard/marketplace/{offering.id}"
											class="font-semibold text-white hover:text-primary-400 transition-colors block leading-snug"
										>{offering.offer_name}</a>
										<a
											href="/dashboard/providers/{offering.owner_username || offering.pubkey}"
											class="text-xs text-neutral-500 hover:text-primary-400 transition-colors mt-0.5 block {offering.owner_username ? '' : 'font-mono truncate'}"
										>
											{offering.owner_username ? `@${offering.owner_username}` : truncatePubkey(offering.pubkey)}
										</a>
									</div>
									<button
										onclick={() => offering.id !== undefined && removeOffering(offering.id)}
										title="Remove from comparison"
										class="shrink-0 text-neutral-600 hover:text-white transition-colors mt-0.5"
										aria-label="Remove {offering.offer_name} from comparison"
									>
										<Icon name="x" size={14} />
									</button>
								</div>
							</th>
						{/each}
					</tr>
				</thead>
				<tbody>
					<!-- PRICING -->
					<tr class="bg-neutral-800/20">
						<td colspan={offerings.length + 1} class="px-4 py-2 text-xs font-semibold text-neutral-400 uppercase tracking-wider">Pricing</td>
					</tr>
					<tr class="border-b border-neutral-800/50 hover:bg-surface-elevated transition-colors">
						<td class="px-4 py-3 text-neutral-500">Monthly price</td>
						{#each offerings as offering}
							<td class="px-4 py-3">
								<span class="font-medium {winners.price === offering.id ? 'text-emerald-400' : 'text-white'}">
									{formatPrice(offering)}
								</span>
								{#if formatUsdEquivalent(offering)}
									<span class="text-xs text-neutral-500 ml-1">{formatUsdEquivalent(offering)}</span>
								{/if}
								{#if winners.price === offering.id}
									<span class="ml-1 text-emerald-400 text-xs font-bold" title="Best price">✓</span>
								{/if}
							</td>
						{/each}
					</tr>
					<tr class="border-b border-neutral-800/50 hover:bg-surface-elevated transition-colors">
						<td class="px-4 py-3 text-neutral-500">Billing</td>
						{#each offerings as offering}
							<td class="px-4 py-3 text-neutral-300">{formatBilling(offering)}</td>
						{/each}
					</tr>
					<tr class="border-b border-neutral-800/50 hover:bg-surface-elevated transition-colors">
						<td class="px-4 py-3 text-neutral-500">Setup fee</td>
						{#each offerings as offering}
							<td class="px-4 py-3 text-neutral-300">
								{offering.setup_fee > 0 ? `${offering.setup_fee.toFixed(2)} ${offering.currency}` : '—'}
							</td>
						{/each}
					</tr>
					<tr class="border-b border-neutral-800/50 hover:bg-surface-elevated transition-colors">
						<td class="px-4 py-3 text-neutral-500">Contract terms</td>
						{#each offerings as offering}
							<td class="px-4 py-3 text-neutral-300">{formatContractTerms(offering)}</td>
						{/each}
					</tr>

					<!-- COMPUTE -->
					<tr class="bg-neutral-800/20">
						<td colspan={offerings.length + 1} class="px-4 py-2 text-xs font-semibold text-neutral-400 uppercase tracking-wider">Compute</td>
					</tr>
					<tr class="border-b border-neutral-800/50 hover:bg-surface-elevated transition-colors">
						<td class="px-4 py-3 text-neutral-500">CPU cores</td>
						{#each offerings as offering}
							<td class="px-4 py-3">
								<span class="{winners.cores === offering.id ? 'text-emerald-400 font-medium' : 'text-neutral-300'}">
									{offering.processor_cores ?? '—'}
								</span>
								{#if offering.processor_name || offering.processor_brand}
									<span class="text-neutral-600 text-xs ml-1">({offering.processor_name || offering.processor_brand})</span>
								{/if}
								{#if winners.cores === offering.id}
									<span class="ml-1 text-emerald-400 text-xs font-bold" title="Most cores">✓</span>
								{/if}
							</td>
						{/each}
					</tr>
					<tr class="border-b border-neutral-800/50 hover:bg-surface-elevated transition-colors">
						<td class="px-4 py-3 text-neutral-500">CPU speed</td>
						{#each offerings as offering}
							<td class="px-4 py-3 text-neutral-300">{offering.processor_speed ?? '—'}</td>
						{/each}
					</tr>
					<tr class="border-b border-neutral-800/50 hover:bg-surface-elevated transition-colors">
						<td class="px-4 py-3 text-neutral-500">RAM</td>
						{#each offerings as offering}
							<td class="px-4 py-3">
								<span class="{winners.ram === offering.id ? 'text-emerald-400 font-medium' : 'text-neutral-300'}">
									{offering.memory_amount ?? '—'}
									{#if offering.memory_type}
										<span class="text-neutral-600 text-xs">({offering.memory_type})</span>
									{/if}
								</span>
								{#if winners.ram === offering.id}
									<span class="ml-1 text-emerald-400 text-xs font-bold" title="Most RAM">✓</span>
								{/if}
							</td>
						{/each}
					</tr>
					{#if offerings.some((o) => o.gpu_name)}
						<tr class="border-b border-neutral-800/50 hover:bg-surface-elevated transition-colors">
							<td class="px-4 py-3 text-neutral-500">GPU</td>
							{#each offerings as offering}
								<td class="px-4 py-3 text-neutral-300">
									{#if offering.gpu_name}
										{offering.gpu_count ? `${offering.gpu_count}x ` : ''}{offering.gpu_name}{offering.gpu_memory_gb ? ` ${offering.gpu_memory_gb}GB` : ''}
									{:else}
										<span class="text-neutral-600">—</span>
									{/if}
								</td>
							{/each}
						</tr>
					{/if}
					<tr class="border-b border-neutral-800/50 hover:bg-surface-elevated transition-colors">
						<td class="px-4 py-3 text-neutral-500">Platform</td>
						{#each offerings as offering}
							<td class="px-4 py-3 text-neutral-300">{offering.virtualization_type ?? '—'}</td>
						{/each}
					</tr>

					<!-- STORAGE -->
					<tr class="bg-neutral-800/20">
						<td colspan={offerings.length + 1} class="px-4 py-2 text-xs font-semibold text-neutral-400 uppercase tracking-wider">Storage</td>
					</tr>
					<tr class="border-b border-neutral-800/50 hover:bg-surface-elevated transition-colors">
						<td class="px-4 py-3 text-neutral-500">SSD</td>
						{#each offerings as offering}
							<td class="px-4 py-3 text-neutral-300">{offering.total_ssd_capacity ?? '—'}</td>
						{/each}
					</tr>
					<tr class="border-b border-neutral-800/50 hover:bg-surface-elevated transition-colors">
						<td class="px-4 py-3 text-neutral-500">HDD</td>
						{#each offerings as offering}
							<td class="px-4 py-3 text-neutral-300">{offering.total_hdd_capacity ?? '—'}</td>
						{/each}
					</tr>

					<!-- NETWORK -->
					<tr class="bg-neutral-800/20">
						<td colspan={offerings.length + 1} class="px-4 py-2 text-xs font-semibold text-neutral-400 uppercase tracking-wider">Network</td>
					</tr>
					<tr class="border-b border-neutral-800/50 hover:bg-surface-elevated transition-colors">
						<td class="px-4 py-3 text-neutral-500">Uplink speed</td>
						{#each offerings as offering}
							<td class="px-4 py-3 text-neutral-300">{offering.uplink_speed ?? '—'}</td>
						{/each}
					</tr>
					<tr class="border-b border-neutral-800/50 hover:bg-surface-elevated transition-colors">
						<td class="px-4 py-3 text-neutral-500">Bandwidth</td>
						{#each offerings as offering}
							<td class="px-4 py-3 text-neutral-300">
								{offering.unmetered_bandwidth ? 'Unmetered' : (offering.traffic ? `${offering.traffic} TB` : '—')}
							</td>
						{/each}
					</tr>

					<!-- LOCATION -->
					<tr class="bg-neutral-800/20">
						<td colspan={offerings.length + 1} class="px-4 py-2 text-xs font-semibold text-neutral-400 uppercase tracking-wider">Location</td>
					</tr>
					<tr class="border-b border-neutral-800/50 hover:bg-surface-elevated transition-colors">
						<td class="px-4 py-3 text-neutral-500">Datacenter</td>
						{#each offerings as offering}
							<td class="px-4 py-3 text-neutral-300">{formatLocation(offering)}</td>
						{/each}
					</tr>

					<!-- PROVIDER -->
					<tr class="bg-neutral-800/20">
						<td colspan={offerings.length + 1} class="px-4 py-2 text-xs font-semibold text-neutral-400 uppercase tracking-wider">Provider</td>
					</tr>
					<tr class="border-b border-neutral-800/50 hover:bg-surface-elevated transition-colors">
						<td class="px-4 py-3 text-neutral-500">Provider</td>
						{#each offerings as offering}
							<td class="px-4 py-3">
								<a
									href="/dashboard/providers/{offering.owner_username || offering.pubkey}"
									class="text-primary-400 hover:text-primary-300 transition-colors text-xs {offering.owner_username ? '' : 'font-mono'}"
								>
									{offering.owner_username ? `@${offering.owner_username}` : truncatePubkey(offering.pubkey)}
								</a>
							</td>
						{/each}
					</tr>
					<tr class="border-b border-neutral-800/50 hover:bg-surface-elevated transition-colors">
						<td class="px-4 py-3 text-neutral-500">Trust score</td>
						{#each offerings as offering}
							<td class="px-4 py-3">
								{#if offering.trust_score !== undefined}
									<TrustBadge score={offering.trust_score} hasFlags={offering.has_critical_flags ?? false} compact={true} />
									{#if winners.trust === offering.id}
										<span class="ml-1 text-emerald-400 text-xs font-bold" title="Highest trust">✓</span>
									{/if}
								{:else}
									<span class="text-neutral-600">—</span>
								{/if}
							</td>
						{/each}
					</tr>
					<tr class="border-b border-neutral-800/50 hover:bg-surface-elevated transition-colors">
						<td class="px-4 py-3 text-neutral-500">Reliability</td>
						{#each offerings as offering}
							<td class="px-4 py-3">
								{#if offering.reliability_score !== undefined && offering.reliability_score !== null}
									<span class="font-medium {offering.reliability_score >= 90 ? 'text-emerald-400' : offering.reliability_score >= 70 ? 'text-yellow-400' : 'text-red-400'}">
										{offering.reliability_score.toFixed(1)}%
									</span>
									{#if winners.reliability === offering.id}
										<span class="ml-1 text-emerald-400 text-xs font-bold" title="Highest reliability">✓</span>
									{/if}
								{:else}
									<span class="text-neutral-600">—</span>
								{/if}
							</td>
						{/each}
					</tr>

					<!-- Action row -->
					<tr class="border-t border-neutral-700 bg-surface-elevated">
						<td class="px-4 py-4 text-neutral-500 font-medium text-xs uppercase tracking-wider">Rent</td>
						{#each offerings as offering}
							<td class="px-4 py-4">
								{#if offering.offering_source === 'seeded' && offering.external_checkout_url}
									<a
										href={offering.external_checkout_url}
										target="_blank"
										rel="noopener noreferrer"
										class="inline-flex items-center gap-1 px-4 py-2 bg-primary-600 hover:bg-primary-500 text-white text-xs font-medium transition-colors"
									>
										Visit Provider <Icon name="external" size={14} class="text-white" />
									</a>
								{:else if offering.is_example}
									<span class="inline-flex px-4 py-2 bg-neutral-700 text-neutral-500 text-xs font-medium cursor-not-allowed">Demo only</span>
								{:else}
									<button
										onclick={() => handleRentClick(offering)}
										class="px-4 py-2 bg-gradient-to-r from-primary-500 to-primary-600 text-white text-xs font-semibold hover:brightness-110 transition-all"
									>
										Rent {offering.offer_name}
									</button>
								{/if}
							</td>
						{/each}
					</tr>
				</tbody>
			</table>
		</div>

		<!-- Legend -->
		<p class="text-xs text-neutral-600 flex items-center gap-1.5">
			<span class="text-emerald-400 font-bold">✓</span> marks the best value in each category when there is a clear winner
		</p>
	{/if}
</div>

<RentalRequestDialog
	offering={selectedOffering}
	onClose={() => (selectedOffering = null)}
	onSuccess={handleRentalSuccess}
/>

<AuthPromptModal
	isOpen={showAuthModal}
	onClose={() => (showAuthModal = false)}
	message="Create an account or login to rent cloud resources"
/>
