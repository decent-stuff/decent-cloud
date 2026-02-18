<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { getOffering, type Offering } from '$lib/services/api';
	import RentalRequestDialog from '$lib/components/RentalRequestDialog.svelte';
	import AuthPromptModal from '$lib/components/AuthPromptModal.svelte';
	import TrustBadge from '$lib/components/TrustBadge.svelte';
	import Icon, { type IconName } from '$lib/components/Icons.svelte';
	import { authStore } from '$lib/stores/auth';
	import { truncatePubkey } from '$lib/utils/identity';

	const offeringId = parseInt($page.params.id ?? '', 10);

	let offering = $state<Offering | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let selectedOffering = $state<Offering | null>(null);
	let isAuthenticated = $state(false);
	let showAuthModal = $state(false);
	let successMessage = $state<string | null>(null);
	let copyLinkFeedback = $state(false);

	authStore.isAuthenticated.subscribe((value) => {
		isAuthenticated = value;
	});

	onMount(async () => {
		try {
			offering = await getOffering(offeringId);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load offering';
		} finally {
			loading = false;
		}
	});

	function handleRentClick() {
		if (!isAuthenticated) {
			showAuthModal = true;
			return;
		}
		selectedOffering = offering;
	}

	function handleRentalSuccess(contractId: string) {
		selectedOffering = null;
		successMessage = `Rental request created! Contract ID: ${contractId}`;
		setTimeout(() => (successMessage = null), 5000);
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
</script>

<div class="space-y-6 max-w-4xl">
	<!-- Breadcrumb -->
	<nav class="text-sm text-neutral-500">
		<a href="/dashboard/marketplace" class="hover:text-white transition-colors">Marketplace</a>
		<span class="mx-2">/</span>
		<span class="text-white">{offering?.offer_name ?? '...'}</span>
	</nav>

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
		<!-- Header -->
		<div class="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-4">
			<div>
				<div class="flex items-center gap-3 flex-wrap">
					<h1 class="text-2xl font-bold text-white tracking-tight">{offering.offer_name}</h1>
					{#if !offering.provider_online}
						<span class="flex items-center gap-1 px-2 py-0.5 text-xs bg-red-500/20 text-red-400 rounded">
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
						href="/dashboard/reputation/{offering.owner_username || offering.pubkey}"
						class="text-sm text-neutral-500 hover:text-primary-400 {offering.owner_username ? '' : 'font-mono'}"
					>
						{offering.owner_username ? `@${offering.owner_username}` : truncatePubkey(offering.pubkey)}
					</a>
				</div>
			</div>

			<div class="flex items-center gap-3">
				<button
					onclick={copyOfferingLink}
					class="px-3 py-1.5 text-sm bg-surface-elevated text-neutral-400 border border-neutral-800 hover:text-white transition-colors"
				>
					{#if copyLinkFeedback}
						<span class="text-green-400">Copied!</span>
					{:else}
						<Icon name="link" size={14} class="inline mr-1" />Copy link
					{/if}
				</button>
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
					<button
						onclick={handleRentClick}
						disabled={offering.is_example}
						class="px-5 py-2.5 bg-gradient-to-r from-primary-500 to-primary-600 font-semibold hover:brightness-110 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
					>
						Rent this offering
					</button>
				{/if}
			</div>
		</div>

		<!-- Price card -->
		<div class="card p-6 border border-neutral-800">
			<div class="flex items-baseline justify-between">
				<div>
					<span class="text-3xl font-bold text-white">{formatPrice(offering)}</span>
					<span class="text-neutral-500 text-sm ml-2">/ month</span>
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
					disabled={offering.is_example}
					class="px-8 py-3 bg-gradient-to-r from-primary-500 to-primary-600 font-semibold hover:brightness-110 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
				>
					Rent this offering
				</button>
			{/if}
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
