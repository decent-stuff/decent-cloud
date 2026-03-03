<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import {
		getProviderProfile,
		getProviderOfferings,
		getProviderTrustMetrics,
		getProviderHealthSummary,
		fetchIcpPrice,
		type ProviderProfile,
		type ProviderTrustMetrics,
		type ProviderHealthSummary,
		type Offering
	} from '$lib/services/api';
	import RentalRequestDialog from '$lib/components/RentalRequestDialog.svelte';
	import AuthPromptModal from '$lib/components/AuthPromptModal.svelte';
	import TrustBadge from '$lib/components/TrustBadge.svelte';
	import Icon, { type IconName } from '$lib/components/Icons.svelte';
	import { authStore } from '$lib/stores/auth';
	import { truncatePubkey } from '$lib/utils/identity';
	import { resolveIdentifierToPubkey } from '$lib/utils/identity';

	const identifier = $page.params.identifier ?? '';

	let pubkey = $state<string>('');
	let profile = $state<ProviderProfile | null>(null);
	let offerings = $state<Offering[]>([]);
	let trustMetrics = $state<ProviderTrustMetrics | null>(null);
	let healthSummary = $state<ProviderHealthSummary | null>(null);
	let icpPriceUsd = $state<number | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let selectedOffering = $state<Offering | null>(null);
	let showAuthModal = $state(false);
	let isAuthenticated = $state(false);
	let successMessage = $state<string | null>(null);

	authStore.isAuthenticated.subscribe((value) => {
		isAuthenticated = value;
	});

	onMount(async () => {
		try {
			const resolved = await resolveIdentifierToPubkey(identifier);
			if (!resolved) {
				error = `Provider not found: ${identifier}`;
				loading = false;
				return;
			}
			pubkey = resolved;

			const [profileData, offeringsData, trustData, healthData, icp] = await Promise.all([
				getProviderProfile(pubkey).catch(() => null),
				getProviderOfferings(pubkey).catch(() => []),
				getProviderTrustMetrics(pubkey).catch(() => null),
				getProviderHealthSummary(pubkey, 30).catch(() => null),
				fetchIcpPrice()
			]);

			profile = profileData;
			offerings = offeringsData;
			trustMetrics = trustData;
			healthSummary = healthData;
			icpPriceUsd = icp;

			if (!profile && offerings.length === 0 && !trustMetrics) {
				error = `No provider data found for: ${identifier}`;
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load provider profile';
		} finally {
			loading = false;
		}
	});

	function handleRentClick(e: Event, offering: Offering) {
		e.stopPropagation();
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

	function getTypeIcon(productType: string): IconName {
		const type = productType.toLowerCase();
		if (type.includes('gpu')) return 'gpu';
		if (type.includes('compute') || type.includes('vm')) return 'cpu';
		if (type.includes('storage')) return 'hard-drive';
		if (type.includes('network') || type.includes('cdn')) return 'globe';
		return 'package';
	}

	function formatPrice(o: Offering): string {
		if (o.reseller_commission_percent && o.monthly_price) {
			const commission = o.monthly_price * (o.reseller_commission_percent / 100);
			return `${(o.monthly_price + commission).toFixed(2)} ${o.currency}`;
		}
		if (o.monthly_price) return `${o.monthly_price.toFixed(2)} ${o.currency}`;
		return 'On request';
	}

	function formatUsdEquivalent(o: Offering): string | null {
		if (!icpPriceUsd || !o.monthly_price) return null;
		if (o.currency?.toUpperCase() !== 'ICP') return null;
		let price = o.monthly_price;
		if (o.reseller_commission_percent) price += price * (o.reseller_commission_percent / 100);
		return `≈ $${(price * icpPriceUsd).toFixed(2)}/mo`;
	}

	function formatSpecs(o: Offering): string {
		const type = o.product_type.toLowerCase();
		if (type.includes('gpu')) {
			const parts = [
				o.gpu_name,
				o.gpu_count ? `${o.gpu_count}x` : null,
				o.gpu_memory_gb ? `${o.gpu_memory_gb}GB` : null
			].filter(Boolean);
			return parts.join(' ') || '—';
		}
		const parts = [
			o.processor_cores ? `${o.processor_cores} vCPU` : null,
			o.memory_amount,
			o.total_ssd_capacity ? `${o.total_ssd_capacity} SSD` : o.total_hdd_capacity ? `${o.total_hdd_capacity} HDD` : null
		].filter(Boolean);
		return parts.join(' · ') || '—';
	}

	function formatLocation(o: Offering): string {
		if (o.datacenter_city && o.datacenter_country) {
			return `${o.datacenter_city}, ${o.datacenter_country}`;
		}
		return o.datacenter_country || '—';
	}

	const displayName = $derived(
		profile?.name || (offerings[0]?.owner_username ? `@${offerings[0].owner_username}` : truncatePubkey(pubkey))
	);
</script>

<div class="space-y-6 max-w-5xl">
	<!-- Breadcrumb -->
	<nav class="text-sm text-neutral-500">
		<a href="/dashboard/marketplace" class="hover:text-white transition-colors">Marketplace</a>
		<span class="mx-2">/</span>
		<span class="text-white">{loading ? '...' : displayName}</span>
	</nav>

	{#if successMessage}
		<div class="bg-success/10 border border-success/20 p-3 text-success text-sm">
			{successMessage}
		</div>
	{/if}

	{#if loading}
		<div class="flex justify-center items-center p-12">
			<div class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-primary-400"></div>
		</div>
	{:else if error && !profile && offerings.length === 0}
		<div class="bg-red-500/20 border border-red-500/30 p-6 text-center">
			<div class="icon-box mx-auto mb-4">
				<Icon name="search" size={20} />
			</div>
			<h2 class="text-lg font-semibold text-white mb-2">Provider Not Found</h2>
			<p class="text-neutral-400 text-sm mb-4">{error}</p>
			<a
				href="/dashboard/marketplace"
				class="inline-block px-6 py-3 bg-surface-elevated font-semibold hover:bg-surface-elevated transition-all"
			>
				Back to Marketplace
			</a>
		</div>
	{:else}
		<!-- Provider Header -->
		<div class="card p-6 border border-neutral-800">
			<div class="flex items-start justify-between gap-4">
				<div class="flex-1">
					<div class="flex items-center gap-3 flex-wrap">
						{#if profile?.logo_url}
							<img src={profile.logo_url} alt="{displayName} logo" class="h-10 w-10 object-contain" />
						{/if}
						<h1 class="text-2xl font-bold text-white tracking-tight">{displayName}</h1>
						{#if trustMetrics}
							<TrustBadge
								score={Number(trustMetrics.trust_score)}
								hasFlags={trustMetrics.has_critical_flags}
								compact={false}
							/>
						{/if}
					</div>

					{#if profile?.description}
						<p class="text-neutral-400 mt-2 text-sm leading-relaxed">{profile.description}</p>
					{/if}

					<div class="flex flex-wrap items-center gap-4 mt-3">
						{#if profile?.website_url}
							<a
								href={profile.website_url}
								target="_blank"
								rel="noopener noreferrer"
								class="inline-flex items-center gap-1.5 text-sm text-primary-400 hover:text-primary-300 transition-colors"
							>
								<Icon name="globe" size={16} />
								{profile.website_url.replace(/^https?:\/\//, '')}
								<Icon name="external" size={14} />
							</a>
						{/if}
						{#if profile?.regions}
							<span class="inline-flex items-center gap-1.5 text-sm text-neutral-400">
								<Icon name="globe" size={16} />
								{profile.regions}
							</span>
						{/if}
					</div>
				</div>

				<div class="shrink-0">
					<p class="data-label mb-1">Public Key</p>
					<p class="font-mono text-xs text-neutral-500 break-all max-w-48">{pubkey}</p>
					<a
						href="/dashboard/reputation/{identifier}"
						class="inline-flex items-center gap-1 mt-2 text-xs text-neutral-500 hover:text-primary-400 transition-colors"
					>
						<Icon name="shield" size={14} />
						Full reputation report
					</a>
				</div>
			</div>
		</div>

		<!-- Trust Summary -->
		{#if trustMetrics || healthSummary}
			<div class="grid grid-cols-2 md:grid-cols-4 gap-3">
				{#if trustMetrics}
					<div class="metric-card">
						<div class="metric-label">Trust Score</div>
						<div class="metric-value">{trustMetrics.trust_score}</div>
						<div class="metric-subtext">{trustMetrics.provider_tenure}</div>
					</div>

					<div class="metric-card">
						<div class="metric-label">Contracts</div>
						<div class="metric-value">{trustMetrics.total_contracts}</div>
						<div class="metric-subtext">{trustMetrics.completion_rate_pct.toFixed(0)}% completed</div>
					</div>

					<div class="metric-card">
						<div class="metric-label">Repeat Customers</div>
						<div class="metric-value">{trustMetrics.repeat_customer_count}</div>
					</div>
				{/if}

				{#if healthSummary}
					<div class="metric-card">
						<div class="metric-label">Uptime (30d)</div>
						<div class="metric-value">{healthSummary.uptimePercent.toFixed(1)}%</div>
						<div class="metric-subtext">{healthSummary.totalChecks} checks</div>
					</div>
				{:else if trustMetrics}
					<div class="metric-card">
						<div class="metric-label">Active Since</div>
						<div class="metric-value text-base">{trustMetrics.days_since_last_checkin}d ago</div>
					</div>
				{/if}
			</div>
		{/if}

		<!-- Offerings Grid -->
		<div>
			<h2 class="text-lg font-semibold text-white mb-3">
				Offerings
				{#if offerings.length > 0}
					<span class="text-neutral-500 font-normal text-sm ml-1">({offerings.length})</span>
				{/if}
			</h2>

			{#if offerings.length === 0}
				<div class="card p-8 text-center border border-neutral-800">
					<Icon name="package" size={32} class="text-neutral-600 mx-auto mb-3" />
					<p class="text-neutral-500">No public offerings available from this provider.</p>
				</div>
			{:else}
				<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
					{#each offerings as offering (offering.id)}
						<div class="card p-4 border border-neutral-800 flex flex-col gap-3">
							<div class="flex items-start justify-between gap-2">
								<div class="flex-1 min-w-0">
									<a
										href="/dashboard/marketplace/{offering.id}"
										class="font-medium text-white hover:text-primary-400 transition-colors block truncate"
									>
										{offering.offer_name}
									</a>
									<span class="inline-flex items-center gap-1 text-xs text-neutral-500 mt-0.5">
										<Icon name={getTypeIcon(offering.product_type)} size={12} />
										{offering.product_type}
									</span>
								</div>
								<div class="flex flex-col items-end gap-1 shrink-0">
									{#if offering.provider_online === false}
										<span class="flex items-center gap-1 px-1.5 py-0.5 text-xs bg-red-500/20 text-red-400 rounded" title="Provider is not actively monitoring — requests are still accepted when agent comes back online">
											<span class="h-1.5 w-1.5 rounded-full bg-red-400"></span>Offline
										</span>
									{/if}
									{#if offering.trust_score !== undefined}
										<TrustBadge
											score={offering.trust_score}
											hasFlags={offering.has_critical_flags ?? false}
											compact={true}
										/>
									{/if}
								</div>
							</div>

							<div class="text-xs text-neutral-400">{formatSpecs(offering)}</div>

							{#if formatLocation(offering) !== '—'}
								<div class="text-xs text-neutral-500">
									<Icon name="globe" size={12} class="inline mr-1" />{formatLocation(offering)}
								</div>
							{/if}

							<div class="flex items-center justify-between mt-auto pt-2 border-t border-neutral-800/60">
								<div>
									<div class="font-semibold text-white text-sm">{formatPrice(offering)}</div>
									{#if formatUsdEquivalent(offering)}
										<div class="text-xs text-neutral-500">{formatUsdEquivalent(offering)}</div>
									{/if}
								</div>

								{#if offering.offering_source === 'seeded' && offering.external_checkout_url}
									<a
										href={offering.external_checkout_url}
										target="_blank"
										rel="noopener noreferrer"
										class="inline-flex items-center gap-1 px-3 py-1.5 bg-primary-600 hover:bg-primary-500 text-xs font-medium transition-colors"
									>
										Visit <Icon name="external" size={12} class="text-white" />
									</a>
								{:else if offering.is_example}
									<span class="px-3 py-1.5 bg-neutral-700 text-neutral-500 text-xs font-medium cursor-not-allowed">Demo</span>
								{:else}
									<button
										onclick={(e) => handleRentClick(e, offering)}
										class="px-3 py-1.5 bg-primary-600 hover:bg-primary-500 text-xs font-medium transition-colors"
									>
										Rent
									</button>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</div>

		<!-- Bottom nav -->
		<div class="flex items-center justify-between pt-2">
			<a href="/dashboard/marketplace" class="text-neutral-400 hover:text-white transition-colors text-sm">
				<Icon name="arrow-left" size={16} class="inline mr-1" />Back to Marketplace
			</a>
			<a
				href="/dashboard/reputation/{identifier}"
				class="text-sm text-neutral-400 hover:text-primary-400 transition-colors"
			>
				<Icon name="shield" size={16} class="inline mr-1" />Full trust report
			</a>
		</div>
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
