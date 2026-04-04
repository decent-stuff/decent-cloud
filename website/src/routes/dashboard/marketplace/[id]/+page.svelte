<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import {
		getOffering,
		getOfferingSlaSummary,
		reviewRecipe,
		searchOfferings,
		fetchIcpPrice,
		getProviderTrustMetrics,
		getProviderProfile,
		getSavedOfferingIds,
		saveOffering,
		unsaveOffering,
		contactOffering,
		trackOfferingView,
		hexEncode,
		type Offering,
		type RecipeReview,
		type ProviderTrustMetrics,
		type ProviderProfile,
		type OfferingSlaSummary
	} from '$lib/services/api';
	import { toggleSavedId } from '$lib/services/saved-offerings';
	import RentalRequestDialog from '$lib/components/RentalRequestDialog.svelte';
	import AuthPromptModal from '$lib/components/AuthPromptModal.svelte';
	import TrustBadge from '$lib/components/TrustBadge.svelte';
	import Icon, { type IconName } from '$lib/components/Icons.svelte';
	import { authStore } from '$lib/stores/auth';
	import { signRequest } from '$lib/services/auth-api';
	import { Ed25519KeyIdentity } from '@dfinity/identity';
	import { truncatePubkey } from '$lib/utils/identity';
	import { recordView } from '$lib/utils/recently-viewed';
	import Breadcrumb from '$lib/components/Breadcrumb.svelte';
	import SlaBreachTimeline from '$lib/components/SlaBreachTimeline.svelte';
	import { filterSimilarOfferings } from './similar-offerings';

	const offeringId = parseInt($page.params.id ?? '', 10);

	let offering = $state<Offering | null>(null);
	let trustMetrics = $state<ProviderTrustMetrics | null>(null);
	let providerProfile = $state<ProviderProfile | null>(null);
	let offeringSlaSummary = $state<OfferingSlaSummary | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let selectedOffering = $state<Offering | null>(null);
	let isAuthenticated = $state(false);
	let showAuthModal = $state(false);
	let successMessage = $state<string | null>(null);
	let copyLinkFeedback = $state(false);
	let icpPriceUsd = $state<number | null>(null);
	let savedIds = $state(new Set<number>());
	let trustWarningDismissed = $state(false);
	let similarOfferings = $state<Offering[]>([]);
	let showOptionsMenu = $state(false);
	let optionsMenuEl = $state<HTMLDivElement | null>(null);
	let recipeReview = $state<RecipeReview | null>(null);
	let recipeReviewLoading = $state(false);
	let recipeReviewError = $state<string | null>(null);

	authStore.isAuthenticated.subscribe((value) => {
		isAuthenticated = value;
	});

	onMount(async () => {
		try {
			[offering, icpPriceUsd] = await Promise.all([getOffering(offeringId), fetchIcpPrice()]);
			if (offering) {
				recordView(offeringId);
				// Fire-and-forget: log a view on the backend for analytics (errors are non-fatal)
				trackOfferingView(offeringId).catch((err) => console.warn('Failed to track offering view:', err));
				if (offering.post_provision_script) {
					recipeReviewLoading = true;
					recipeReviewError = null;
					reviewRecipe(offering.post_provision_script)
						.then((result) => {
							recipeReview = result;
						})
						.catch((err) => {
							recipeReviewError = err instanceof Error ? err.message : 'Recipe review unavailable';
						})
						.finally(() => {
							recipeReviewLoading = false;
						});
				}
				[trustMetrics, providerProfile, offeringSlaSummary] = await Promise.all([
					getProviderTrustMetrics(offering.pubkey).catch(() => null),
					getProviderProfile(offering.pubkey).catch(() => null),
					getOfferingSlaSummary(offering.id!, 30).catch(() => null)
				]);
				try {
					const all = await searchOfferings({ limit: 10, in_stock_only: true });
					similarOfferings = filterSimilarOfferings(all, offering!, 4);
				} catch { /* ignore */ }
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load offering';
		} finally {
			loading = false;
		}
		trustWarningDismissed = sessionStorage.getItem(`trust_warning_dismissed_${offeringId}`) === '1';
		if (isAuthenticated) {
			try {
				const info = await authStore.getSigningIdentity();
				if (info && info.identity instanceof Ed25519KeyIdentity) {
					const pubkeyHex = hexEncode(info.publicKeyBytes);
					const { headers } = await signRequest(info.identity, 'GET', `/api/v1/users/${pubkeyHex}/saved-offering-ids`);
					const ids = await getSavedOfferingIds(headers, pubkeyHex);
					savedIds = new Set(ids);
				}
			} catch (err) {
				console.error('Failed to load saved offerings:', err);
			}
		}
		window.addEventListener('keydown', handleOptionsKeydown);
		document.addEventListener('click', handleOptionsClickOutside, true);
	});

	function closeOptionsMenu() {
		showOptionsMenu = false;
	}

	function handleOptionsClickOutside(e: MouseEvent) {
		if (showOptionsMenu && optionsMenuEl && !optionsMenuEl.contains(e.target as Node)) {
			closeOptionsMenu();
		}
	}

	function handleOptionsKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape' && showOptionsMenu) {
			closeOptionsMenu();
		}
	}

	onDestroy(() => {
		if (typeof window !== 'undefined') {
			window.removeEventListener('keydown', handleOptionsKeydown);
			document.removeEventListener('click', handleOptionsClickOutside, true);
		}
	});

	async function toggleBookmark() {
		if (!isAuthenticated) {
			showAuthModal = true;
			return;
		}
		if (!offering?.id) return;
		const info = await authStore.getSigningIdentity();
		if (!info || !(info.identity instanceof Ed25519KeyIdentity)) return;
		const pubkeyHex = hexEncode(info.publicKeyBytes);
		const isSaved = savedIds.has(offering.id);
		savedIds = toggleSavedId(savedIds, offering.id);
		try {
			if (isSaved) {
				const { headers } = await signRequest(info.identity, 'DELETE', `/api/v1/users/${pubkeyHex}/saved-offerings/${offering.id}`);
				await unsaveOffering(headers, pubkeyHex, offering.id);
			} else {
				const { headers } = await signRequest(info.identity, 'POST', `/api/v1/users/${pubkeyHex}/saved-offerings/${offering.id}`);
				await saveOffering(headers, pubkeyHex, offering.id);
			}
		} catch (err) {
			savedIds = toggleSavedId(savedIds, offering.id);
			console.error('Failed to toggle bookmark:', err);
		}
	}

	function handleRentClick() {
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

	function copyOfferingLink() {
		navigator.clipboard.writeText(window.location.href);
		copyLinkFeedback = true;
		setTimeout(() => {
			copyLinkFeedback = false;
		}, 2000);
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

	function formatSimilarPrice(o: Offering): { primary: string; usdEquivalent: string | null } {
		if (!o.monthly_price) {
			return { primary: 'On request', usdEquivalent: null };
		}
		let price = o.monthly_price;
		if (o.reseller_commission_percent) {
			price += price * (o.reseller_commission_percent / 100);
		}
		const currency = o.currency?.toUpperCase();
		if (currency === 'USD') {
			return { primary: `$${price.toFixed(2)}`, usdEquivalent: null };
		}
		if (currency === 'ICP' && icpPriceUsd && icpPriceUsd > 0) {
			const usdAmount = price * icpPriceUsd;
			return { primary: `${price.toFixed(2)} ICP`, usdEquivalent: `≈ $${usdAmount.toFixed(2)}/mo` };
		}
		return { primary: `${price.toFixed(2)} ${o.currency}`, usdEquivalent: null };
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
			const hours = o.min_contract_hours;
			if (hours >= 720) parts.push(`Min ${Math.round(hours / 720)}mo`);
			else if (hours >= 24) parts.push(`Min ${Math.round(hours / 24)}d`);
			else parts.push(`Min ${hours}h`);
		}
		if (o.max_contract_hours) {
			const hours = o.max_contract_hours;
			if (hours >= 720) parts.push(`Max ${Math.round(hours / 720)}mo`);
			else if (hours >= 24) parts.push(`Max ${Math.round(hours / 24)}d`);
			else parts.push(`Max ${hours}h`);
		}
		return parts.length > 0 ? parts.join(' · ') : '—';
	}

	function formatProvisionTime(hours: number | undefined): string {
		if (hours === undefined || hours === null) return '—';
		if (hours < 1 / 60) return '<1 min';
		if (hours < 1) return `~${Math.round(hours * 60)} min`;
		if (hours < 24) return `~${hours.toFixed(1)}h`;
		return `~${Math.round(hours / 24)}d`;
	}

	function reviewTone(value: number, inverse = false): string {
		if (inverse) {
			if (value >= 8) return 'text-red-300 border-red-500/40 bg-red-500/10';
			if (value >= 5) return 'text-yellow-300 border-yellow-500/40 bg-yellow-500/10';
			return 'text-green-300 border-green-500/40 bg-green-500/10';
		}

		if (value >= 8) return 'text-green-300 border-green-500/40 bg-green-500/10';
		if (value >= 5) return 'text-yellow-300 border-yellow-500/40 bg-yellow-500/10';
		return 'text-red-300 border-red-500/40 bg-red-500/10';
	}

	const DURATION_PRESETS = [
		{ label: '1h', hours: 1 },
		{ label: '12h', hours: 12 },
		{ label: '1d', hours: 24 },
		{ label: '7d', hours: 168 },
		{ label: '30d', hours: 720 },
		{ label: '90d', hours: 2160 },
	] as const;

	let selectedDurationHours = $state<number>(720);

	const estimatedCost = $derived(
		offering?.monthly_price != null ? (offering.monthly_price / 720) * selectedDurationHours : null
	);

	// Contact provider dialog
	let showContactDialog = $state(false);
	let contactMessage = $state('');
	let contactSubmitting = $state(false);
	let contactError = $state<string | null>(null);
	let contactSuccess = $state(false);

	function slaTone(value: number | undefined): string {
		if (value === undefined || value === null) return 'text-neutral-300';
		if (value >= 99) return 'text-emerald-400';
		if (value >= 95) return 'text-yellow-300';
		return 'text-red-400';
	}

	async function handleContactSubmit() {
		if (!isAuthenticated) {
			showAuthModal = true;
			return;
		}
		if (!offering?.id || !contactMessage.trim()) return;
		const info = await authStore.getSigningIdentity();
		if (!info || !(info.identity instanceof Ed25519KeyIdentity)) return;
		contactSubmitting = true;
		contactError = null;
		try {
			const { headers } = await signRequest(info.identity, 'POST', `/api/v1/offerings/${offering.id}/contact`, { message: contactMessage.trim() });
			await contactOffering(offering.id, contactMessage.trim(), headers);
			contactSuccess = true;
			contactMessage = '';
			setTimeout(() => {
				showContactDialog = false;
				contactSuccess = false;
			}, 2000);
		} catch (e) {
			contactError = e instanceof Error ? e.message : 'Failed to send inquiry';
		} finally {
			contactSubmitting = false;
		}
	}
</script>

<div class="space-y-6 max-w-5xl">
	<Breadcrumb items={[
		isAuthenticated
			? { label: 'Dashboard', href: '/dashboard/rentals' }
			: { label: 'Home', href: '/' },
		{ label: 'Marketplace', href: '/dashboard/marketplace' },
		{ label: offering?.offer_name ?? '…' },
	]} />

	<!-- Mobile back button -->
	<button
		onclick={() => history.back()}
		class="md:hidden fixed bottom-6 right-6 z-40 flex items-center gap-2 px-4 py-2.5 bg-surface-elevated border border-neutral-700 text-neutral-300 hover:text-white shadow-lg transition-colors"
		aria-label="Go back"
	>
		← Back
	</button>

	{#if successMessage}
		<div class="bg-success/10 border border-success/20 p-3 text-success text-sm">
			{successMessage}
		</div>
	{/if}

	{#if error}
		<div class="bg-red-500/20 border border-red-500/30 p-6 text-center">
			<h2 class="text-2xl font-bold text-red-400 mb-2">Offering Not Found</h2>
			<p class="text-neutral-400 mb-4">{error}</p>
			<a
				href="/dashboard/marketplace"
				class="inline-block px-6 py-3 bg-surface-elevated font-semibold hover:bg-surface-elevated transition-all"
			>
				Back to Marketplace
			</a>
		</div>
	{:else if loading}
		<div class="flex justify-center items-center p-12">
			<div class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-primary-400"></div>
		</div>
	{:else if offering}
	<!-- Trust warning banners -->
	{#if trustMetrics && trustMetrics.total_contracts === 0}
		<div class="bg-blue-500/10 border border-blue-500/30 p-4 flex items-start gap-3">
			<span class="text-blue-400 shrink-0 text-base leading-none mt-0.5">ℹ</span>
			<p class="text-sm text-blue-300">
				This is a new provider with no completed contracts yet.
				Consider starting with a short rental to test reliability.
			</p>
		</div>
	{:else if !trustWarningDismissed && trustMetrics && Number(trustMetrics.trust_score) < 60}
		<div class="bg-amber-500/10 border border-amber-500/30 p-4 flex items-start justify-between gap-4">
			<div class="flex items-start gap-3">
				<span class="text-amber-400 shrink-0 text-base leading-none mt-0.5">⚠</span>
				<p class="text-sm text-amber-300">
					This provider has a low trust score ({Number(trustMetrics.trust_score)}%). New or underperforming providers may be less reliable.
				</p>
			</div>
			<div class="flex items-center gap-3 shrink-0">
				<a
					href="/dashboard/marketplace?types={offering.product_type.toLowerCase().split(' ')[0]}"
					class="text-xs text-amber-400 hover:text-amber-300 underline whitespace-nowrap"
				>See alternatives</a>
				<button
					onclick={() => { trustWarningDismissed = true; sessionStorage.setItem(`trust_warning_dismissed_${offeringId}`, '1'); }}
					class="text-xs text-neutral-400 hover:text-white whitespace-nowrap"
				>I understand, continue</button>
			</div>
		</div>
	{/if}
		<div class="grid grid-cols-1 lg:grid-cols-[1fr_272px] gap-6 items-start">
		<div class="space-y-6">
		<!-- Header -->
		<div class="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-4">
			<div>
				<div class="flex items-center gap-3 flex-wrap">
					<h1 class="text-2xl font-bold text-white tracking-tight">{offering.offer_name}</h1>
					{#if offering.provider_online === false}
						<span class="flex items-center gap-1 px-2 py-0.5 text-xs bg-red-500/20 text-red-400 rounded" title="Provider is not actively monitoring — requests are still accepted when agent comes back online">
							<span class="h-1.5 w-1.5 rounded-full bg-red-400"></span>Offline
						</span>
					{/if}
					{#if offering.trust_score !== undefined}
						<TrustBadge score={offering.trust_score} hasFlags={offering.has_critical_flags ?? false} compact={true} />
					{/if}
					{#if offering.post_provision_script}
						<span class="px-2 py-0.5 text-xs bg-blue-500/20 text-blue-400 rounded">Recipe</span>
					{/if}
					{#if offering.is_subscription}
						<span class="inline-flex items-center gap-1 px-2 py-0.5 text-xs bg-purple-500/20 text-purple-400 rounded">
							<Icon name="repeat" size={14} class="text-purple-400" /> Subscription
						</span>
					{/if}
				</div>
				<div class="flex items-center gap-3 mt-1">
					<span class="inline-flex items-center gap-1.5 text-sm text-neutral-400">
						<Icon name={getTypeIcon(offering.product_type)} size={16} />
						{offering.product_type}
					</span>
					<a
						href="/dashboard/providers/{offering.owner_username || offering.pubkey}"
						class="text-sm text-neutral-500 hover:text-primary-400 {offering.owner_username ? '' : 'font-mono'}"
					>
						{offering.owner_username ? `@${offering.owner_username}` : truncatePubkey(offering.pubkey)}
					</a>
					<a href="/dashboard/marketplace?provider={offering.pubkey}" class="text-xs text-neutral-500 hover:text-primary-400 transition-colors">
						View all offerings →
					</a>
				</div>
			</div>

			<div class="flex items-center gap-3">
				{#if offering.offering_source === 'seeded' && offering.external_checkout_url}
					<a
						href={offering.external_checkout_url}
						target="_blank"
						rel="noopener noreferrer"
						class="inline-flex items-center gap-1 px-5 py-2.5 bg-primary-600 hover:bg-primary-500 font-semibold transition-colors"
					>
						Visit Provider <Icon name="external" size={16} class="text-white" />
					</a>
				{:else}
					<div class="relative" bind:this={optionsMenuEl}>
						<button
							onclick={() => showOptionsMenu = !showOptionsMenu}
							class="p-2.5 text-neutral-400 hover:text-white hover:bg-surface-elevated transition-colors"
							aria-label="More options"
							aria-expanded={showOptionsMenu}
						>
							<Icon name="more-vertical" size={18} />
						</button>
						{#if showOptionsMenu}
							<div class="absolute right-0 top-full mt-1 w-48 bg-surface-elevated border border-neutral-700 shadow-xl z-50">
								<button
									onclick={() => { copyOfferingLink(); closeOptionsMenu(); }}
									class="w-full px-4 py-2.5 text-left text-sm text-neutral-300 hover:bg-surface-hover hover:text-white flex items-center gap-2.5 transition-colors"
								>
									<Icon name="link" size={14} />
									{#if copyLinkFeedback}
										<span class="text-green-400">Copied!</span>
									{:else}
										Copy link
									{/if}
								</button>
								{#if offering.id !== undefined}
									<button
										onclick={() => { toggleBookmark(); closeOptionsMenu(); }}
										class="w-full px-4 py-2.5 text-left text-sm text-neutral-300 hover:bg-surface-hover hover:text-white flex items-center gap-2.5 transition-colors {savedIds.has(offering.id) ? 'text-primary-400' : ''}"
									>
										<Icon name="bookmark" size={14} />
										{savedIds.has(offering.id) ? 'Saved' : 'Save'}
									</button>
								{/if}
								<button
									onclick={() => { closeOptionsMenu(); if (!isAuthenticated) { showAuthModal = true; } else { showContactDialog = true; } }}
									class="w-full px-4 py-2.5 text-left text-sm text-neutral-300 hover:bg-surface-hover hover:text-white flex items-center gap-2.5 transition-colors"
								>
									<Icon name="mail" size={14} />
									Ask Provider
								</button>
							</div>
						{/if}
					</div>
					<button
						onclick={handleRentClick}
						disabled={offering.is_example || offering.provider_online === false}
						class="px-5 py-2.5 bg-gradient-to-r from-primary-500 to-primary-600 font-semibold hover:brightness-110 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
						title={offering.provider_online === false ? 'This provider is currently offline. Your request will be queued until they return.' : ''}
					>
						{#if offering.provider_online === false}
							Provider Offline
						{:else}
							Rent this offering
						{/if}
					</button>
				{/if}
			</div>
		</div>

		{#if offeringSlaSummary && (offeringSlaSummary.slaTargetPercent !== undefined || offeringSlaSummary.reports30d > 0)}
			<div class="card p-5 border border-neutral-800 space-y-4">
				<div class="flex items-start justify-between gap-4">
					<div>
						<h2 class="text-lg font-semibold text-white">SLA & Reported Reliability</h2>
						<p class="text-sm text-neutral-400 mt-1">
							Provider-submitted SLI data for this offering. Breach markers show days that missed the stated SLA.
						</p>
					</div>
					<div class="text-right shrink-0">
						<div class="text-xs uppercase tracking-wide text-neutral-500">Promised SLA</div>
						<div class="text-3xl font-semibold text-white">{offeringSlaSummary.slaTargetPercent?.toFixed(2) ?? '—'}%</div>
					</div>
				</div>

				<div class="grid grid-cols-2 md:grid-cols-4 gap-4">
					<div>
						<div class="data-label mb-1">30d Compliance</div>
						<div class="text-2xl font-semibold {slaTone(offeringSlaSummary.compliance30dPercent)}">
							{offeringSlaSummary.compliance30dPercent?.toFixed(1) ?? '—'}%
						</div>
					</div>
					<div>
						<div class="data-label mb-1">Average Uptime</div>
						<div class="text-2xl font-semibold text-white">{offeringSlaSummary.averageUptime30d?.toFixed(2) ?? '—'}%</div>
					</div>
					<div>
						<div class="data-label mb-1">Breach Days</div>
						<div class="text-2xl font-semibold {offeringSlaSummary.breachDays30d > 0 ? 'text-red-400' : 'text-emerald-400'}">{offeringSlaSummary.breachDays30d}</div>
					</div>
					<div>
						<div class="data-label mb-1">Latest Report</div>
						<div class="text-sm font-medium text-white">{offeringSlaSummary.latestReportDate ?? 'No report yet'}</div>
						{#if offeringSlaSummary.latestUptimePercent !== undefined}
							<div class="text-xs text-neutral-500 mt-1">{offeringSlaSummary.latestUptimePercent.toFixed(2)}% uptime</div>
						{/if}
					</div>
				</div>

				<SlaBreachTimeline timeline={offeringSlaSummary.timeline} days={30} />
			</div>
		{/if}

		<!-- Price card -->
		<div class="card p-6 border border-neutral-800">
			<div class="flex items-baseline justify-between">
				<div>
					<span class="text-3xl font-bold text-white">{formatPrice(offering)}</span>
					<span class="text-neutral-500 text-sm ml-2">/ month</span>
					{#if formatUsdEquivalent(offering)}
						<span class="text-neutral-500 text-sm ml-2">{formatUsdEquivalent(offering)}</span>
					{/if}
				</div>
				{#if offering.setup_fee > 0}
					<div class="text-neutral-400 text-sm">
						+ {offering.setup_fee.toFixed(2)} {offering.currency} setup
					</div>
				{/if}
			</div>
			<div class="flex items-center gap-4 mt-2 text-sm text-neutral-500">
				<span>Billing: {formatBilling(offering)}</span>
				{#if offering.min_contract_hours || offering.max_contract_hours}
					<span>Contract: {formatContractTerms(offering)}</span>
				{/if}
			</div>
			{#if estimatedCost != null}
				<div class="mt-4 pt-4 border-t border-neutral-800 space-y-3">
					<h3 class="text-xs font-medium text-neutral-500 uppercase tracking-wide">Estimate cost</h3>
					<div class="flex flex-wrap gap-2">
						{#each DURATION_PRESETS as preset}
							<button
								onclick={() => selectedDurationHours = preset.hours}
								class="px-3 py-1.5 text-xs font-medium border transition-colors {selectedDurationHours === preset.hours
									? 'bg-primary-600 border-primary-500 text-white'
									: 'bg-surface-elevated border-neutral-700 text-neutral-400 hover:text-white hover:border-neutral-600'}"
							>
								{preset.label}
							</button>
						{/each}
					</div>
					<div class="flex items-baseline gap-2">
						<span class="text-2xl font-bold text-white">{estimatedCost.toFixed(4)}</span>
						<span class="text-neutral-400 text-sm">{offering.currency}</span>
						{#if icpPriceUsd && offering.currency?.toUpperCase() === 'ICP'}
							<span class="text-neutral-500 text-xs">≈ ${(estimatedCost * icpPriceUsd).toFixed(2)} USD</span>
						{/if}
					</div>
					<p class="text-neutral-600 text-xs">Based on {offering.monthly_price!.toFixed(4)} {offering.currency}/mo rate</p>
				</div>
			{/if}
		</div>

		<!-- Description -->
		{#if offering.description}
			<div class="card p-6 border border-neutral-800">
				<h2 class="text-sm font-medium text-neutral-500 uppercase tracking-wide mb-3">Description</h2>
				<p class="text-neutral-300 whitespace-pre-wrap">{offering.description}</p>
			</div>
		{/if}

		<!-- Specs -->
		<div class="card p-6 border border-neutral-800">
			<h2 class="text-sm font-medium text-neutral-500 uppercase tracking-wide mb-4">Specifications</h2>
			<div class="grid grid-cols-2 md:grid-cols-3 gap-4 text-sm">
				{#if offering.processor_cores}
					<div>
						<span class="text-neutral-500 text-xs block">vCPUs</span>
						<span class="text-white">{offering.processor_cores}
							{#if offering.processor_name || offering.processor_brand}
								<span class="text-neutral-500 text-xs">({offering.processor_name || offering.processor_brand})</span>
							{/if}
						</span>
					</div>
				{/if}
				{#if offering.memory_amount}
					<div>
						<span class="text-neutral-500 text-xs block">Memory</span>
						<span class="text-white">{offering.memory_amount}
							{#if offering.memory_type}
								<span class="text-neutral-500 text-xs">({offering.memory_type})</span>
							{/if}
						</span>
					</div>
				{/if}
				{#if offering.total_ssd_capacity || offering.total_hdd_capacity}
					<div>
						<span class="text-neutral-500 text-xs block">Storage</span>
						<span class="text-white">
							{[
								offering.total_ssd_capacity ? `${offering.total_ssd_capacity} SSD` : null,
								offering.total_hdd_capacity ? `${offering.total_hdd_capacity} HDD` : null
							].filter(Boolean).join(' + ')}
						</span>
					</div>
				{/if}
				{#if offering.uplink_speed || offering.unmetered_bandwidth}
					<div>
						<span class="text-neutral-500 text-xs block">Network</span>
						<span class="text-white">
							{offering.uplink_speed || ''}
							{offering.unmetered_bandwidth ? ' (Unmetered)' : offering.traffic ? ` (${offering.traffic} TB)` : ''}
						</span>
					</div>
				{/if}
				{#if offering.gpu_name}
					<div>
						<span class="text-neutral-500 text-xs block">GPU</span>
						<span class="text-white">
							{offering.gpu_count ? `${offering.gpu_count}x ` : ''}{offering.gpu_name}
							{offering.gpu_memory_gb ? ` ${offering.gpu_memory_gb}GB` : ''}
						</span>
					</div>
				{/if}
				{#if offering.virtualization_type}
					<div>
						<span class="text-neutral-500 text-xs block">Platform</span>
						<span class="text-white">{offering.virtualization_type}</span>
					</div>
				{/if}
				{#if offering.datacenter_city || offering.datacenter_country}
					<div>
						<span class="text-neutral-500 text-xs block">Location</span>
						<span class="text-white">
							{[offering.datacenter_city, offering.datacenter_country].filter(Boolean).join(', ')}
						</span>
					</div>
				{/if}
				{#if offering.operating_systems}
					<div>
						<span class="text-neutral-500 text-xs block">OS</span>
						<span class="text-white">{offering.operating_systems}</span>
					</div>
				{/if}
				{#if offering.features}
					<div>
						<span class="text-neutral-500 text-xs block">Features</span>
						<span class="text-white">{offering.features}</span>
					</div>
				{/if}
				{#if offering.control_panel}
					<div>
						<span class="text-neutral-500 text-xs block">Control Panel</span>
						<span class="text-white">{offering.control_panel}</span>
					</div>
				{/if}
			</div>
		</div>

		<!-- Recipe script -->
		{#if offering.post_provision_script}
			<div class="card p-6 border border-blue-500/30">
				<h2 class="text-sm font-medium text-blue-400 uppercase tracking-wide mb-3 flex items-center gap-2">
					<Icon name="code" size={16} class="text-blue-400" />
					Recipe Script
				</h2>
				<p class="text-neutral-500 text-xs mb-3">
					This script runs as root via SSH after the VM boots. Review it before renting.
				</p>
				<div class="mb-4 rounded-lg border border-neutral-800 bg-base/40 p-4">
					<div class="flex items-center justify-between gap-3 mb-3">
						<h3 class="text-xs font-medium text-neutral-400 uppercase tracking-wide">AI Recipe Review</h3>
						{#if recipeReviewLoading}
							<span class="text-xs text-neutral-500">Analyzing...</span>
						{/if}
					</div>
					{#if recipeReview}
						<div class="grid grid-cols-1 sm:grid-cols-3 gap-2 mb-3 text-sm">
							<div class={`rounded border px-3 py-2 ${reviewTone(recipeReview.security_risk, true)}`}>
								<div class="text-xs uppercase tracking-wide opacity-80 mb-1">Security Risk</div>
								<div class="font-semibold">{recipeReview.security_risk}/10</div>
							</div>
							<div class={`rounded border px-3 py-2 ${reviewTone(recipeReview.completeness)}`}>
								<div class="text-xs uppercase tracking-wide opacity-80 mb-1">Completeness</div>
								<div class="font-semibold">{recipeReview.completeness}/10</div>
							</div>
							<div class={`rounded border px-3 py-2 ${reviewTone(recipeReview.user_value)}`}>
								<div class="text-xs uppercase tracking-wide opacity-80 mb-1">User Value</div>
								<div class="font-semibold">{recipeReview.user_value}/10</div>
							</div>
						</div>
						<p class="text-sm text-neutral-300 mb-3">{recipeReview.summary}</p>
						{#if recipeReview.concerns.length > 0}
							<ul class="space-y-1 text-xs text-neutral-400">
								{#each recipeReview.concerns as concern}
									<li>{concern}</li>
								{/each}
							</ul>
						{/if}
					{:else if recipeReviewError}
						<p class="text-xs text-neutral-500">AI review unavailable: {recipeReviewError}</p>
					{:else}
						<p class="text-xs text-neutral-500">AI review will appear here when available.</p>
					{/if}
				</div>
				<pre class="p-4 bg-base/50 border border-neutral-800 text-sm text-neutral-300 font-mono overflow-x-auto max-h-96 overflow-y-auto whitespace-pre-wrap">{offering.post_provision_script}</pre>
			</div>
		{/if}

		<!-- Bottom CTA -->
		<div class="flex items-center justify-between">
			<a href="/dashboard/marketplace" class="text-neutral-400 hover:text-white transition-colors">
				Back to Marketplace
			</a>
			{#if offering.offering_source === 'seeded' && offering.external_checkout_url}
				<a
					href={offering.external_checkout_url}
					target="_blank"
					rel="noopener noreferrer"
					class="inline-flex items-center gap-1 px-6 py-3 bg-primary-600 hover:bg-primary-500 font-semibold transition-colors"
				>
					Visit Provider <Icon name="external" size={16} class="text-white" />
				</a>
			{:else}
				<button
					onclick={handleRentClick}
					disabled={offering.is_example || offering.provider_online === false}
					class="px-8 py-3 bg-gradient-to-r from-primary-500 to-primary-600 font-semibold hover:brightness-110 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
					title={offering.provider_online === false ? 'This provider is currently offline. Your request will be queued until they return.' : ''}
				>
					{#if offering.provider_online === false}
						Provider Offline
					{:else}
						Rent this offering
					{/if}
				</button>
			{/if}
		</div>
		</div>

		<!-- Provider Sidebar -->
		<div class="space-y-4 lg:sticky lg:top-6">
			<div class="card p-5 border border-neutral-800">
				<h2 class="text-xs font-medium text-neutral-500 uppercase tracking-wide mb-4">Provider</h2>

				<a
					href="/dashboard/providers/{offering.owner_username || offering.pubkey}"
					class="flex items-center gap-2 group mb-4"
				>
					<div class="h-8 w-8 rounded-full bg-primary-500/20 flex items-center justify-center shrink-0">
						<Icon name="user" size={16} class="text-primary-400" />
					</div>
					<span class="text-sm font-medium text-white group-hover:text-primary-400 transition-colors {offering.owner_username ? '' : 'font-mono truncate'}">
						{offering.owner_username ? `@${offering.owner_username}` : truncatePubkey(offering.pubkey)}
					</span>
				</a>

				{#if offering.trust_score !== undefined}
					<div class="mb-4">
						<span class="text-xs text-neutral-500 block mb-1.5">Trust Score</span>
						<TrustBadge
							score={offering.trust_score}
							hasFlags={offering.has_critical_flags ?? false}
							compact={false}
						/>
					</div>
				{/if}

				{#if offering.reliability_score !== undefined}
					<div class="mb-4">
						<span class="text-xs text-neutral-500 block mb-1">Reliability</span>
						<div class="flex items-center gap-2">
							<div class="flex-1 h-1.5 bg-neutral-800 rounded-full overflow-hidden">
								<div
									class="h-full rounded-full {offering.reliability_score >= 80 ? 'bg-green-500' : offering.reliability_score >= 60 ? 'bg-yellow-500' : 'bg-red-500'}"
									style="width: {offering.reliability_score}%"
								></div>
							</div>
							<span class="text-xs text-neutral-400 shrink-0">{offering.reliability_score}%</span>
						</div>
					</div>
				{/if}

				{#if trustMetrics}
					<div class="space-y-2 mb-4 text-sm">
						<div class="flex justify-between">
							<span class="text-neutral-500">Rentals</span>
							<span class="text-white">{trustMetrics.total_contracts}</span>
						</div>
						{#if trustMetrics.completion_rate_pct > 0}
							<div class="flex justify-between">
								<span class="text-neutral-500">Completion</span>
								<span class="text-white">{trustMetrics.completion_rate_pct.toFixed(0)}%</span>
							</div>
						{/if}
						<div class="flex justify-between">
							<span class="text-neutral-500">Tenure</span>
							<span class="text-white capitalize">{trustMetrics.provider_tenure}</span>
						</div>
						{#if trustMetrics.total_contracts > 0 && trustMetrics.time_to_delivery_hours !== undefined}
							<div class="flex justify-between">
								<span class="text-neutral-500">Setup Time</span>
								<span class="text-white">{formatProvisionTime(trustMetrics.time_to_delivery_hours)}</span>
							</div>
						{/if}
					</div>
				{/if}

				<a
					href="/dashboard/providers/{offering.owner_username || offering.pubkey}"
					class="flex items-center justify-center gap-1.5 w-full px-3 py-2 text-sm border border-neutral-700 text-neutral-300 hover:border-primary-500 hover:text-primary-400 transition-colors"
				>
					View Provider Profile
					<Icon name="external" size={14} />
				</a>

				{#if providerProfile?.website_url || providerProfile?.support_email}
					<div class="pt-3 mt-3 border-t border-neutral-800 space-y-2">
						{#if providerProfile.website_url}
							<a
								href={providerProfile.website_url}
								target="_blank"
								rel="noopener noreferrer"
								class="flex items-center gap-2 text-xs text-neutral-400 hover:text-primary-400 transition-colors"
							>
								<Icon name="globe" size={14} />
								Website
							</a>
						{/if}
						{#if providerProfile.support_email}
							<a
								href="mailto:{providerProfile.support_email}"
								class="flex items-center gap-2 text-xs text-neutral-400 hover:text-primary-400 transition-colors"
							>
								<Icon name="mail" size={14} />
								{providerProfile.support_email}
							</a>
						{/if}
					</div>
				{/if}
			</div>
		</div>
		</div>

	{#if similarOfferings.length > 0}
		<div class="mt-8">
			<h3 class="text-sm font-semibold text-neutral-400 uppercase tracking-wide mb-3">Similar Offerings</h3>
			<div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
				{#each similarOfferings as o}
					{@const price = formatSimilarPrice(o)}
					<a href="/dashboard/marketplace/{o.id}" class="card p-4 border border-neutral-800 hover:border-neutral-600 transition-colors block">
						<div class="flex items-center justify-between gap-2">
							<span class="text-white font-medium text-sm truncate">{o.offer_name}</span>
							<span class="text-primary-400 text-sm font-semibold whitespace-nowrap">{price.primary}/mo</span>
						</div>
						<div class="flex items-center justify-between mt-1">
							<span class="text-neutral-500 text-xs">{o.product_type}{o.datacenter_country ? ` · ${o.datacenter_country}` : ''}</span>
							{#if price.usdEquivalent}
								<span class="text-neutral-500 text-xs">{price.usdEquivalent}</span>
							{/if}
						</div>
					</a>
				{/each}
			</div>
		</div>
	{/if}
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

{#if showContactDialog}
	<div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70" role="dialog" aria-modal="true">
		<div class="bg-surface-elevated border border-neutral-700 w-full max-w-md space-y-5 p-6">
			<div class="flex items-center justify-between">
				<h2 class="text-lg font-semibold text-white">Ask the Provider</h2>
				<button onclick={() => { showContactDialog = false; contactError = null; contactMessage = ''; contactSuccess = false; }} class="text-neutral-400 hover:text-white transition-colors">
					<Icon name="x" size={20} />
				</button>
			</div>

			{#if contactSuccess}
				<div class="bg-success/10 border border-success/20 p-4 text-success text-sm text-center">
					Message sent! The provider will see it in their notifications.
				</div>
			{:else}
				<p class="text-sm text-neutral-400">
					Send a question to <span class="text-white">{offering?.owner_username ? `@${offering.owner_username}` : 'this provider'}</span> about their offering. They'll receive it as a notification.
				</p>

				{#if contactError}
					<div class="bg-red-500/20 border border-red-500/30 p-3 text-red-400 text-sm">{contactError}</div>
				{/if}

				<div>
					<label for="contact-message" class="block text-sm font-medium text-neutral-400 mb-1.5">Your Message</label>
					<textarea
						id="contact-message"
						bind:value={contactMessage}
						rows={5}
						placeholder="Hi, I have a question about this offering..."
						maxlength={2000}
						class="w-full bg-base border border-neutral-700 text-white px-3 py-2 focus:border-primary-500 focus:outline-none resize-y text-sm"
					></textarea>
					<p class="text-xs text-neutral-600 mt-1">{contactMessage.length}/2000</p>
				</div>

				<div class="flex items-center justify-end gap-3">
					<button
						onclick={() => { showContactDialog = false; contactError = null; contactMessage = ''; }}
						class="text-neutral-400 hover:text-white transition-colors text-sm"
					>
						Cancel
					</button>
					<button
						onclick={handleContactSubmit}
						disabled={contactSubmitting || !contactMessage.trim()}
						class="px-5 py-2 bg-gradient-to-r from-primary-500 to-primary-600 font-semibold hover:brightness-110 transition-all disabled:opacity-50 disabled:cursor-not-allowed text-sm flex items-center gap-2"
					>
						{#if contactSubmitting}
							<div class="animate-spin rounded-full h-3.5 w-3.5 border-t-2 border-b-2 border-white"></div>
							Sending...
						{:else}
							Send Message
						{/if}
					</button>
				</div>
			{/if}
		</div>
	</div>
{/if}
