<script lang="ts">
	import { onMount } from 'svelte';
	import {
		getProviderOfferings,
		exportProviderOfferingsCSV,
		updateProviderOffering,
		type Offering,
		type CsvImportResult,
		getExampleOfferingsCSV,
		getProductTypes,
		type ProductType
	} from '$lib/services/api';
	import { authStore } from '$lib/stores/auth';
	import { hexEncode } from '$lib/services/api';
	import { signRequest } from '$lib/services/auth-api';
	import OfferingsEditor from '$lib/components/OfferingsEditor.svelte';
	import QuickEditOfferingDialog from '$lib/components/QuickEditOfferingDialog.svelte';
	import Icon, { type IconName } from '$lib/components/Icons.svelte';
	import type { Ed25519KeyIdentity } from '@dfinity/identity';

	let offerings = $state<Offering[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let currentIdentity = $state<any>(null);
	let showEditorDialog = $state(false);
	let showEditDialog = $state(false);
	let showTemplateDialog = $state(false);
	let editingOffering = $state<Offering | null>(null);
	let importSuccess = $state<string | null>(null);
	let editorCsvContent = $state('');
	let productTypes = $state<ProductType[]>([]);

	async function loadOfferings() {
		try {
			loading = true;
			error = null;

			if (!currentIdentity || !currentIdentity.publicKeyBytes) {
				error = 'Please authenticate to view your offerings';
				return;
			}

			const pubkeyHex = hexEncode(currentIdentity.publicKeyBytes);
			offerings = await getProviderOfferings(pubkeyHex);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load offerings';
			console.error('Error loading offerings:', e);
		} finally {
			loading = false;
		}
	}

	function handleImportSuccess(event: CustomEvent<CsvImportResult>) {
		const result = event.detail;
		importSuccess = `Successfully imported ${result.success_count} offering${result.success_count !== 1 ? 's' : ''}`;

		setTimeout(() => {
			importSuccess = null;
		}, 5000);

		loadOfferings();
	}

	function handleEditClick(offering: Offering) {
		editingOffering = offering;
		showEditDialog = true;
	}

	function handleEditSuccess() {
		importSuccess = 'Offering updated successfully';
		setTimeout(() => {
			importSuccess = null;
		}, 5000);
		loadOfferings();
	}

	async function updateOfferingField(offering: Offering, updates: Partial<Offering>) {
		if (!currentIdentity?.identity || !currentIdentity?.publicKeyBytes || offering.id === undefined) {
			throw new Error('Authentication or offering data missing');
		}

		const pubkeyHex = hexEncode(currentIdentity.publicKeyBytes);
		const path = `/api/v1/providers/${pubkeyHex}/offerings/${offering.id}`;
		const params = { ...offering, ...updates };
		const signed = await signRequest(currentIdentity.identity, 'PUT', path, params);

		if (!signed.body) throw new Error('Failed to sign request');

		await updateProviderOffering(currentIdentity.publicKeyBytes, offering.id, signed.body, signed.headers);
		await loadOfferings();
	}

	async function handleStockToggle(offering: Offering, event: Event) {
		event.stopPropagation();
		const statuses = ['in_stock', 'out_of_stock', 'discontinued'];
		const currentIndex = statuses.indexOf(offering.stock_status);
		const newStatus = statuses[(currentIndex + 1) % statuses.length];

		try {
			await updateOfferingField(offering, { stock_status: newStatus });
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to update stock status';
			console.error('Error updating stock status:', e);
		}
	}

	async function handleVisibilityToggle(offering: Offering, event: Event) {
		event.stopPropagation();
		const newVisibility = offering.visibility === 'public' ? 'private' : 'public';

		try {
			await updateOfferingField(offering, { visibility: newVisibility });
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to update visibility';
			console.error('Error updating visibility:', e);
		}
	}

	async function openEditor() {
		try {
			if (!currentIdentity?.identity || !currentIdentity?.publicKeyBytes) {
				error = 'Please authenticate to edit offerings';
				return;
			}

			if (offerings.length > 0) {
				const pubkeyHex = hexEncode(currentIdentity.publicKeyBytes);
				const path = `/api/v1/providers/${pubkeyHex}/offerings/export`;
				const signed = await signRequest(currentIdentity.identity, 'GET', path);
				editorCsvContent = await exportProviderOfferingsCSV(
					currentIdentity.publicKeyBytes,
					signed.headers
				);
				showEditorDialog = true;
			} else {
				showTemplateDialog = true;
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load CSV';
			console.error('Error loading CSV:', e);
		}
	}

	async function downloadTemplate(productType: string) {
		try {
			const csv = await getExampleOfferingsCSV(productType);
			const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
			const link = document.createElement('a');
			const url = URL.createObjectURL(blob);

			link.setAttribute('href', url);
			link.setAttribute('download', `offerings-template-${productType}.csv`);
			link.style.visibility = 'hidden';
			document.body.appendChild(link);
			link.click();
			document.body.removeChild(link);
			URL.revokeObjectURL(url);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to download template';
			console.error('Error downloading template:', e);
		}
	}

	async function openEditorWithTemplate(productType: string) {
		try {
			editorCsvContent = await getExampleOfferingsCSV(productType);
			showTemplateDialog = false;
			showEditorDialog = true;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load template';
			console.error('Error loading template:', e);
		}
	}

	async function loadProductTypes() {
		try {
			productTypes = await getProductTypes();
		} catch (e) {
			console.error('Error loading product types:', e);
		}
	}

	onMount(() => {
		const unsubscribe = authStore.currentIdentity.subscribe((identity) => {
			currentIdentity = identity;
		});

		loadProductTypes();
		loadOfferings();
		return unsubscribe;
	});

	function getStatusColor(stockStatus: string) {
		switch (stockStatus) {
			case 'in_stock':
				return 'bg-green-500/20 text-green-400 border-green-500/30';
			case 'out_of_stock':
				return 'bg-red-500/20 text-red-400 border-red-500/30';
			case 'discontinued':
				return 'bg-gray-500/20 text-gray-400 border-gray-500/30';
			default:
				return 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30';
		}
	}

	function getTypeIcon(productType: string): IconName {
		const type = productType.toLowerCase();
		if (type.includes('gpu')) return 'gpu';
		if (type.includes('compute') || type.includes('vm')) return 'cpu';
		if (type.includes('storage')) return 'hard-drive';
		if (type.includes('network') || type.includes('cdn')) return 'globe';
		return 'package';
	}

	function formatPrice(offering: Offering): string {
		if (offering.monthly_price) {
			return `${offering.monthly_price.toFixed(2)} ${offering.currency}/mo`;
		}
		return 'Price on request';
	}
</script>

<div class="space-y-8">
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-2xl font-bold text-white tracking-tight">My Offerings</h1>
			<p class="text-neutral-500">Manage your cloud service offerings</p>
		</div>
		<div class="flex gap-3">
			<button
				onclick={() => (showTemplateDialog = true)}
				class="px-6 py-3 bg-surface-elevated backdrop-blur  font-semibold hover:bg-surface-elevated transition-all flex items-center gap-2"
				title="Download CSV template with example offerings"
			>
				<svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
					/>
				</svg>
				Download Template
			</button>
			<button
				onclick={openEditor}
				class="px-6 py-3 bg-gradient-to-r from-primary-500 to-primary-600  font-semibold hover:brightness-110 hover:scale-105 transition-all flex items-center gap-2"
			>
				<svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
					/>
				</svg>
				Edit Offerings
			</button>
		</div>
	</div>

	{#if importSuccess}
		<div
			class="bg-green-500/20 border border-green-500/30  p-4 flex items-center gap-2 animate-fade-in"
		>
			<Icon name="check" size={24} class="text-green-400" />
			<p class="text-green-400 font-semibold">{importSuccess}</p>
		</div>
	{/if}

	{#if error}
		<div class="bg-red-500/20 border border-red-500/30  p-4 text-red-400">
			<p class="font-semibold">Error loading offerings</p>
			<p class="text-sm mt-1">{error}</p>
		</div>
	{/if}

	{#if offerings.filter(o => !o.resolved_pool_id).length > 0}
		{@const offeringsWithoutPool = offerings.filter(o => !o.resolved_pool_id)}
		<div class="bg-amber-500/20 border border-amber-500/30  p-4 flex items-start gap-3">
			<Icon name="alert" size={24} class="text-amber-400 shrink-0" />
			<div>
				<p class="text-amber-400 font-semibold">
					{offeringsWithoutPool.length} offering{offeringsWithoutPool.length !== 1 ? 's' : ''} without matching pool
				</p>
				<p class="text-amber-400/80 text-sm mt-1">
					These offerings are hidden from the public marketplace. Create agent pools or assign pools via CSV to enable provisioning.
				</p>
			</div>
		</div>
	{/if}

	{#if loading}
		<div class="flex justify-center items-center p-8">
			<div class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-primary-400"></div>
		</div>
	{:else}
		<!-- Stats Summary -->
		<div class="grid grid-cols-1 md:grid-cols-3 gap-6">
			<div class="card p-6 border border-neutral-800">
				<div class="flex items-center justify-between mb-2">
					<h3 class="text-neutral-400 text-sm font-medium">Active Offerings</h3>
					<Icon name="package" size={24} class="text-primary-400" />
				</div>
				<p class="text-3xl font-bold text-white">
					{offerings.filter((o) => o.stock_status === 'in_stock').length}
				</p>
			</div>

			<div class="card p-6 border border-neutral-800">
				<div class="flex items-center justify-between mb-2">
					<h3 class="text-neutral-400 text-sm font-medium">Total Offerings</h3>
					<Icon name="chart" size={24} class="text-primary-400" />
				</div>
				<p class="text-3xl font-bold text-white">{offerings.length}</p>
			</div>

			<div class="card p-6 border border-neutral-800">
				<div class="flex items-center justify-between mb-2">
					<h3 class="text-neutral-400 text-sm font-medium">Product Types</h3>
					<Icon name="grid" size={24} class="text-primary-400" />
				</div>
				<p class="text-3xl font-bold text-white">
					{new Set(offerings.map((o) => o.product_type)).size}
				</p>
			</div>
		</div>

		<!-- Offerings Grid -->
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
			{#each offerings as offering}
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="card p-6 border border-neutral-800 hover:border-white/40 transition-all group cursor-pointer"
					onclick={() => handleEditClick(offering)}
				>
					<!-- Header: Icon and Badges -->
					<div class="flex items-start justify-between mb-4">
						<Icon name={getTypeIcon(offering.product_type)} size={36} class="text-primary-400" />
						<div class="flex items-center gap-2 flex-wrap justify-end">
							<!-- Offline indicator -->
							{#if !offering.provider_online}
								<span
									class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium bg-red-500/20 text-red-400 border border-red-500/30"
									title="Provider agent is offline - provisioning will not work until agent comes online"
								>
									<span class="w-2 h-2 rounded-full bg-red-400 animate-pulse"></span>
									Offline
								</span>
							{/if}
							<!-- Visibility toggle -->
							<button
								onclick={(e) => handleVisibilityToggle(offering, e)}
								class="inline-flex items-center px-2 py-1 rounded-md text-xs font-medium border transition-all hover:scale-105 cursor-pointer {offering.visibility === 'public'
									? 'bg-green-500/20 text-green-400 border-green-500/30 hover:bg-green-500/30'
									: 'bg-red-500/20 text-red-400 border-red-500/30 hover:bg-red-500/30'}"
								title="Click to toggle visibility"
							>
								{offering.visibility === 'public' ? 'Public' : 'Private'}
							</button>
							<!-- Stock status toggle -->
							<button
								onclick={(e) => handleStockToggle(offering, e)}
								class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium border transition-all hover:scale-105 cursor-pointer {getStatusColor(
									offering.stock_status
								)}"
								title="Click to cycle stock status: {offering.stock_status.replace('_', ' ')}"
							>
								<span class="w-2 h-2 rounded-full bg-current"></span>
								{offering.stock_status.replace('_', ' ')}
							</button>
						</div>
					</div>

					<!-- Title -->
					<h3 class="text-xl font-bold text-white mb-3 group-hover:text-primary-400 transition-colors">
						{offering.offer_name}
					</h3>

					<!-- Details -->
					<div class="space-y-2 text-sm">
						<div class="flex items-center justify-between text-neutral-400">
							<span>Type</span>
							<span class="text-white font-medium">{offering.product_type}</span>
						</div>
						<div class="flex items-center justify-between text-neutral-400">
							<span>Price</span>
							<span class="text-white font-medium">{formatPrice(offering)}</span>
						</div>
						{#if offering.datacenter_country}
							<div class="flex items-center justify-between text-neutral-400">
								<span>Location</span>
								<span class="text-white font-medium">{offering.datacenter_city}, {offering.datacenter_country}</span>
							</div>
						{/if}
						<div class="flex items-center justify-between text-neutral-400">
							<span>Pool</span>
							{#if offering.resolved_pool_name}
								<span class="text-primary-400 font-medium">→ {offering.resolved_pool_name}</span>
							{:else}
								<span class="inline-flex items-center gap-1 text-amber-400 font-medium"><Icon name="alert" size={12} class="text-amber-400" /> No pool</span>
							{/if}
						</div>
						{#if offering.description}
							<div class="text-neutral-500 text-xs mt-3 line-clamp-2">{offering.description}</div>
						{/if}
					</div>
				</div>
			{/each}
		</div>

		<!-- Empty State (if no offerings) -->
		{#if offerings.length === 0}
			<div class="text-center py-16">
				<div class="flex justify-center mb-4">
					<Icon name="package" size={56} class="text-neutral-600" />
				</div>
				<h3 class="text-2xl font-bold text-white mb-2">No Offerings Yet</h3>
				<p class="text-neutral-500 mb-6">Create your first cloud service offering to get started</p>
				<button
					onclick={openEditor}
					class="px-6 py-3 bg-gradient-to-r from-primary-500 to-primary-600  font-semibold hover:brightness-110 hover:scale-105 transition-all"
				>
					Create Your First Offering
				</button>
			</div>
		{/if}
	{/if}
</div>

<OfferingsEditor
	bind:open={showEditorDialog}
	identity={currentIdentity?.identity as Ed25519KeyIdentity}
	pubkeyBytes={currentIdentity?.publicKeyBytes}
	csvContent={editorCsvContent}
	on:success={handleImportSuccess}
/>

<QuickEditOfferingDialog
	bind:open={showEditDialog}
	offering={editingOffering}
	identity={currentIdentity?.identity as Ed25519KeyIdentity}
	pubkeyBytes={currentIdentity?.publicKeyBytes}
	on:success={handleEditSuccess}
/>

<!-- Product Type Selection Dialog -->
{#if showTemplateDialog}
	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
	<div
		class="fixed inset-0 bg-base/70 backdrop-blur-sm flex items-center justify-center z-50"
		onclick={() => (showTemplateDialog = false)}
	>
		<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
		<div
			class="bg-gradient-to-br from-slate-900 to-slate-800  p-8 max-w-2xl w-full mx-4 border border-neutral-800 shadow-2xl max-h-[90vh] overflow-y-auto"
			onclick={(e) => e.stopPropagation()}
		>
			<h2 class="text-2xl font-bold text-white mb-4">Select Product Type</h2>
			<p class="text-neutral-500 mb-6">Choose a product type to download an example template</p>
			<div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
				{#each productTypes as productType}
					<button
						onclick={() => downloadTemplate(productType.key)}
						class="p-4 bg-surface-elevated backdrop-blur  border border-neutral-800 hover:border-white/40 hover:bg-surface-elevated transition-all text-left group"
					>
						<div class="text-2xl mb-2">{productType.label.split(' ')[0]}</div>
						<div class="text-white font-medium group-hover:text-primary-400 transition-colors">
							{productType.label.substring(productType.label.indexOf(' ') + 1)}
						</div>
						<div class="text-neutral-500 text-sm mt-1">Download template</div>
					</button>
				{/each}
			</div>
			{#if offerings.length === 0}
				<div class="border-t border-neutral-800 pt-6">
					<p class="text-neutral-500 mb-4 text-sm">Or start editing with a template:</p>
					<div class="grid grid-cols-1 md:grid-cols-2 gap-3">
						{#each productTypes as productType}
							<button
								onclick={() => openEditorWithTemplate(productType.key)}
								class="p-3 bg-primary-500/20 backdrop-blur  border border-primary-500/30 hover:border-primary-500/50 hover:bg-primary-500/30 transition-all text-left"
							>
								<div class="text-sm text-primary-400 font-medium">
									Edit {productType.label.substring(productType.label.indexOf(' ') + 1)}
								</div>
							</button>
						{/each}
					</div>
				</div>
			{/if}
			<div class="flex justify-end mt-6">
				<button
					onclick={() => (showTemplateDialog = false)}
					class="px-6 py-2 bg-surface-elevated  hover:bg-surface-elevated transition-colors"
				>
					Cancel
				</button>
			</div>
		</div>
	</div>
{/if}
