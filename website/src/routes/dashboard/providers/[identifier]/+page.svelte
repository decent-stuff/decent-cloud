<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import {
		getProviderProfile,
		getProviderOfferings,
		getProviderTrustMetrics,
		getProviderSlaSummary,
		getProviderHealthSummary,
		getProviderFeedbackStats,
		getProviderContacts,
		fetchIcpPrice,
		type ProviderProfile,
		type ProviderTrustMetrics,
		type ProviderHealthSummary,
		type ProviderFeedbackStats,
		type ProviderContact,
		type ProviderSlaSummary,
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
	let providerSlaSummary = $state<ProviderSlaSummary | null>(null);
	let feedbackStats = $state<ProviderFeedbackStats | null>(null);
	let contacts = $state<ProviderContact[]>([]);
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

	function parseJsonField<T>(field: string | undefined | null): T[] {
		if (!field) return [];
		try {
			return JSON.parse(field) as T[];
		} catch {
			return [];
		}
	}

	onMount(async () => {
		try {
			const resolved = await resolveIdentifierToPubkey(identifier);
			if (!resolved) {
				error = `Provider not found: ${identifier}`;
				loading = false;
				return;
			}
			pubkey = resolved;

			const [profileData, offeringsData, trustData, healthData, providerSlaData, feedbackData, contactsData, icp] = await Promise.all([
				getProviderProfile(pubkey).catch(() => null),
				getProviderOfferings(pubkey).catch(() => []),
				getProviderTrustMetrics(pubkey).catch(() => null),
				getProviderHealthSummary(pubkey, 30).catch(() => null),
				getProviderSlaSummary(pubkey, 30).catch(() => null),
				getProviderFeedbackStats(pubkey).catch(() => null),
				getProviderContacts(pubkey).catch(() => []),
				fetchIcpPrice()
			]);

			profile = profileData;
			offerings = offeringsData;
			trustMetrics = trustData;
			healthSummary = healthData;
			providerSlaSummary = providerSlaData;
			feedbackStats = feedbackData;
			contacts = contactsData;
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

	const sellingPoints = $derived(parseJsonField<string>(profile?.unique_selling_points));
	const supportChannels = $derived(parseJsonField<string>(profile?.support_channels));
	const paymentMethods = $derived(parseJsonField<string>(profile?.payment_methods));

	function reliabilityTone(score: number | undefined): string {
		if (score === undefined || score === null) return 'text-neutral-300';
		if (score >= 95) return 'text-success';
		if (score >= 85) return 'text-warning';
		return 'text-danger';
	}
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
		<div class="bg-danger/20 border border-danger/30 p-6 text-center">
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
							{#if trustMetrics.provider_tenure}
								<span class="px-2 py-0.5 text-xs border border-neutral-700 text-neutral-400 rounded">
									{trustMetrics.provider_tenure}
								</span>
							{/if}
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
						{#if contacts.length > 0}
							{#each contacts as contact}
								<span class="inline-flex items-center gap-1.5 text-sm text-neutral-400">
									<Icon name="mail" size={16} />
									<span class="capitalize">{contact.contactType}:</span>
									<span>{contact.contactValue}</span>
								</span>
							{/each}
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

		<!-- Trust & Reliability Summary -->
		{#if trustMetrics || healthSummary}
			<div class="grid grid-cols-2 md:grid-cols-5 gap-3">
				{#if trustMetrics}
					<div class="metric-card">
						<div class="metric-label">Trust Score</div>
						<div class="metric-value">{trustMetrics.trust_score}</div>
						<div class="metric-subtext">{trustMetrics.provider_tenure}</div>
					</div>

					{#if trustMetrics.reliability_score != null}
						<div class="metric-card">
							<div class="metric-label">Reliability</div>
							<div class="metric-value">{trustMetrics.reliability_score.toFixed(1)}</div>
							<div class="metric-subtext">uptime + completion</div>
						</div>
					{/if}

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

		{#if providerSlaSummary && (providerSlaSummary.offeringsTracked > 0 || providerSlaSummary.reports30d > 0)}
			<div class="card p-5 border border-neutral-800">
				<div class="flex items-start justify-between gap-4 mb-4">
					<div>
						<h2 class="text-lg font-semibold text-white">Provider-Reported SLA</h2>
						<p class="text-sm text-neutral-400 mt-1">
							Current phase uses provider-submitted SLI reports. Independent platform monitoring comes later.
						</p>
					</div>
					{#if providerSlaSummary.averageSlaTargetPercent !== undefined}
						<div class="text-right shrink-0">
							<div class="text-xs uppercase tracking-wide text-neutral-500">Average SLA target</div>
							<div class="text-2xl font-semibold text-white">{providerSlaSummary.averageSlaTargetPercent.toFixed(2)}%</div>
						</div>
					{/if}
				</div>

				<div class="grid grid-cols-2 md:grid-cols-4 gap-4">
					<div>
						<div class="data-label mb-1">30d Compliance</div>
						<div class="text-2xl font-semibold {reliabilityTone(providerSlaSummary.compliance30dPercent)}">
							{providerSlaSummary.compliance30dPercent?.toFixed(1) ?? '—'}%
						</div>
					</div>
					<div>
						<div class="data-label mb-1">Average Uptime</div>
						<div class="text-2xl font-semibold text-white">
							{providerSlaSummary.averageUptime30d?.toFixed(2) ?? '—'}%
						</div>
					</div>
					<div>
						<div class="data-label mb-1">Breach Days</div>
						<div class="text-2xl font-semibold {providerSlaSummary.breachDays30d > 0 ? 'text-danger' : 'text-success'}">
							{providerSlaSummary.breachDays30d}
						</div>
						<div class="text-xs text-neutral-500 mt-1">across {providerSlaSummary.reports30d} reported days</div>
					</div>
					<div>
						<div class="data-label mb-1">Reliability Penalty</div>
						<div class="text-2xl font-semibold {providerSlaSummary.penaltyPoints > 0 ? 'text-yellow-300' : 'text-neutral-300'}">
							-{providerSlaSummary.penaltyPoints.toFixed(1)}
						</div>
						<div class="text-xs text-neutral-500 mt-1">applied to provider score</div>
					</div>
				</div>
			</div>
		{/if}

		<!-- Feedback Section -->
		{#if feedbackStats && feedbackStats.total_responses > 0}
			<div class="card p-5 border border-neutral-800">
				<h2 class="text-lg font-semibold text-white mb-4">Renter Feedback</h2>
				<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
					<div>
						<div class="data-label mb-1">Total Responses</div>
						<div class="text-2xl font-semibold text-white">{feedbackStats.total_responses}</div>
					</div>
					<div>
						<div class="data-label mb-1">Service Matched Description</div>
						<div class="text-2xl font-semibold {feedbackStats.service_match_rate_pct >= 80 ? 'text-success' : feedbackStats.service_match_rate_pct >= 60 ? 'text-warning' : 'text-danger'}">
							{feedbackStats.service_match_rate_pct.toFixed(0)}%
						</div>
						<div class="text-xs text-neutral-500 mt-1">
							{feedbackStats.service_matched_yes} yes / {feedbackStats.service_matched_no} no
						</div>
						<div class="mt-2 h-2 bg-neutral-800 rounded-full overflow-hidden">
							<div
								class="h-full bg-green-500 rounded-full transition-all"
								style="width: {feedbackStats.service_match_rate_pct}%"
							></div>
						</div>
					</div>
					<div>
						<div class="data-label mb-1">Would Rent Again</div>
						<div class="text-2xl font-semibold {feedbackStats.would_rent_again_rate_pct >= 80 ? 'text-success' : feedbackStats.would_rent_again_rate_pct >= 60 ? 'text-warning' : 'text-danger'}">
							{feedbackStats.would_rent_again_rate_pct.toFixed(0)}%
						</div>
						<div class="text-xs text-neutral-500 mt-1">
							{feedbackStats.would_rent_again_yes} yes / {feedbackStats.would_rent_again_no} no
						</div>
						<div class="mt-2 h-2 bg-neutral-800 rounded-full overflow-hidden">
							<div
								class="h-full bg-green-500 rounded-full transition-all"
								style="width: {feedbackStats.would_rent_again_rate_pct}%"
							></div>
						</div>
					</div>
				</div>
			</div>
		{/if}

		<!-- Why Choose Us / Selling Points -->
		{#if profile?.why_choose_us || sellingPoints.length > 0}
			<div class="card p-5 border border-neutral-800">
				<h2 class="text-lg font-semibold text-white mb-3">Why Choose This Provider</h2>
				{#if profile?.why_choose_us}
					<p class="text-neutral-400 text-sm leading-relaxed">{profile.why_choose_us}</p>
				{/if}
				{#if sellingPoints.length > 0}
					<ul class="mt-3 space-y-1.5">
						{#each sellingPoints as point}
							<li class="flex items-start gap-2 text-sm text-neutral-300">
								<Icon name="check" size={16} class="text-success mt-0.5 shrink-0" />
								{point}
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		{/if}

		<!-- Support & SLA Section -->
		{#if profile?.support_email || profile?.support_hours || profile?.support_channels || profile?.sla_guarantee || profile?.refund_policy || profile?.payment_methods}
			<div class="card p-5 border border-neutral-800">
				<h2 class="text-lg font-semibold text-white mb-4">Support & Policies</h2>
				<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
					{#if profile.support_email || profile.support_hours || profile.support_channels}
						<div>
							<h3 class="data-label mb-2">Support</h3>
							<div class="space-y-1.5">
								{#if profile.support_email}
									<div class="flex items-center gap-2 text-sm">
										<Icon name="mail" size={16} class="text-neutral-500" />
										<span class="text-neutral-300">{profile.support_email}</span>
									</div>
								{/if}
								{#if profile.support_hours}
									<div class="flex items-center gap-2 text-sm">
										<Icon name="clock" size={16} class="text-neutral-500" />
										<span class="text-neutral-300">{profile.support_hours}</span>
									</div>
								{/if}
								{#if profile.support_channels}
									{#if supportChannels.length > 0}
										<div class="flex items-center gap-2 text-sm flex-wrap">
											<Icon name="mail" size={16} class="text-neutral-500" />
											{#each supportChannels as channel}
												<span class="px-2 py-0.5 text-xs bg-neutral-800 border border-neutral-700 text-neutral-300 rounded">{channel}</span>
											{/each}
										</div>
									{/if}
								{/if}
							</div>
						</div>
					{/if}

					{#if profile.sla_guarantee}
						<div>
							<h3 class="data-label mb-2">SLA Guarantee</h3>
							<p class="text-sm text-neutral-300 leading-relaxed">{profile.sla_guarantee}</p>
						</div>
					{/if}

					{#if profile.payment_methods}
						<div>
							<h3 class="data-label mb-2">Payment Methods</h3>
							<div class="flex flex-wrap gap-1.5">
								{#each paymentMethods as method}
									<span class="px-2 py-0.5 text-xs bg-neutral-800 border border-neutral-700 text-neutral-300 rounded">{method}</span>
								{/each}
							</div>
						</div>
					{/if}

					{#if profile.refund_policy}
						<div>
							<h3 class="data-label mb-2">Refund Policy</h3>
							<p class="text-sm text-neutral-300 leading-relaxed">{profile.refund_policy}</p>
						</div>
					{/if}
				</div>
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
										<span class="flex items-center gap-1 px-1.5 py-0.5 text-xs bg-danger/20 text-danger rounded" title="Provider is not actively monitoring — requests are still accepted when agent comes back online">
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
