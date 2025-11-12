<script lang="ts">
	import { onMount } from 'svelte';
	import {
		getProviderOfferings,
		exportProviderOfferingsCSV,
		type Offering,
		type CsvImportResult,
		downloadCSVTemplate,
		fetchCSVTemplate
	} from '$lib/services/api';
	import { authStore } from '$lib/stores/auth';
	import { hexEncode } from '$lib/services/api';
	import { signRequest } from '$lib/services/auth-api';
	import OfferingsEditor from '$lib/components/OfferingsEditor.svelte';
	import QuickEditOfferingDialog from '$lib/components/QuickEditOfferingDialog.svelte';
	import type { Ed25519KeyIdentity } from '@dfinity/identity';

	let offerings = $state<Offering[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let currentIdentity = $state<any>(null);
	let showEditorDialog = $state(false);
	let showEditDialog = $state(false);
	let editingOffering = $state<Offering | null>(null);
	let importSuccess = $state<string | null>(null);
	let editorCsvContent = $state('');

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

	function handleStockToggle(offering: Offering, event: Event) {
		event.stopPropagation();
		editingOffering = offering;
		showEditDialog = true;
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
			} else {
				editorCsvContent = await fetchCSVTemplate();
			}

			showEditorDialog = true;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load CSV';
			console.error('Error loading CSV:', e);
		}
	}

	onMount(() => {
		const unsubscribe = authStore.currentIdentity.subscribe((identity) => {
			currentIdentity = identity;
		});

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

	function getTypeIcon(productType: string) {
		const type = productType.toLowerCase();
		if (type.includes('compute') || type.includes('vm')) return '💻';
		if (type.includes('storage')) return '💾';
		if (type.includes('network') || type.includes('cdn')) return '🌐';
		return '📦';
	}

	function formatPrice(offering: Offering): string {
		if (offering.price_per_hour_e9s) {
			const price = offering.price_per_hour_e9s / 1_000_000_000;
			return `${price.toFixed(4)} DCT/hr`;
		}
		if (offering.monthly_price) {
			return `${offering.monthly_price.toFixed(2)} ${offering.currency}/mo`;
		}
		return 'Price on request';
	}
</script>

<div class="space-y-8">
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-4xl font-bold text-white mb-2">My Offerings</h1>
			<p class="text-white/60">Manage your cloud service offerings</p>
		</div>
		<div class="flex gap-3">
			<button
				onclick={downloadCSVTemplate}
				class="px-6 py-3 bg-white/10 backdrop-blur rounded-lg font-semibold hover:bg-white/20 transition-all flex items-center gap-2"
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
				class="px-6 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold hover:brightness-110 hover:scale-105 transition-all flex items-center gap-2"
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
			class="bg-green-500/20 border border-green-500/30 rounded-lg p-4 flex items-center gap-2 animate-fade-in"
		>
			<span class="text-2xl">✅</span>
			<p class="text-green-400 font-semibold">{importSuccess}</p>
		</div>
	{/if}

	{#if error}
		<div class="bg-red-500/20 border border-red-500/30 rounded-lg p-4 text-red-400">
			<p class="font-semibold">Error loading offerings</p>
			<p class="text-sm mt-1">{error}</p>
		</div>
	{/if}

	{#if loading}
		<div class="flex justify-center items-center p-8">
			<div class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-400"></div>
		</div>
	{:else}
		<!-- Stats Summary -->
		<div class="grid grid-cols-1 md:grid-cols-3 gap-6">
			<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
				<div class="flex items-center justify-between mb-2">
					<h3 class="text-white/70 text-sm font-medium">Active Offerings</h3>
					<span class="text-2xl">📦</span>
				</div>
				<p class="text-3xl font-bold text-white">
					{offerings.filter((o) => o.stock_status === 'in_stock').length}
				</p>
			</div>

			<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
				<div class="flex items-center justify-between mb-2">
					<h3 class="text-white/70 text-sm font-medium">Total Offerings</h3>
					<span class="text-2xl">📊</span>
				</div>
				<p class="text-3xl font-bold text-white">{offerings.length}</p>
			</div>

			<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
				<div class="flex items-center justify-between mb-2">
					<h3 class="text-white/70 text-sm font-medium">Product Types</h3>
					<span class="text-2xl">🏷️</span>
				</div>
				<p class="text-3xl font-bold text-white">
					{new Set(offerings.map((o) => o.product_type)).size}
				</p>
			</div>
		</div>

		<!-- Offerings Grid -->
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
			{#each offerings as offering}
				<div
					class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20 hover:border-white/40 transition-all group"
				>
					<div class="flex items-start justify-between mb-4">
						<span class="text-4xl">{getTypeIcon(offering.product_type)}</span>
						<div class="flex items-center gap-2">
							<!-- Interactive stock status toggle -->
							<button
								onclick={(e) => handleStockToggle(offering, e)}
								class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full text-xs font-medium border transition-all hover:scale-105 cursor-pointer {getStatusColor(
									offering.stock_status
								)}"
								title="Click to edit stock status"
							>
								<span class="w-2 h-2 rounded-full bg-current"></span>
								{offering.stock_status.replace('_', ' ')}
							</button>
							<!-- Edit pencil icon -->
							<button
								onclick={() => handleEditClick(offering)}
								class="p-1.5 bg-white/10 rounded-lg hover:bg-white/20 transition-all hover:scale-110"
								title="Edit offering"
								aria-label="Edit offering"
							>
								<svg class="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										stroke-width="2"
										d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"
									/>
								</svg>
							</button>
						</div>
					</div>

					<h3 class="text-xl font-bold text-white mb-2 group-hover:text-blue-400 transition-colors">
						{offering.offer_name}
					</h3>

					<div class="space-y-2 text-sm">
						<div class="flex items-center justify-between text-white/70">
							<span>Type</span>
							<span class="text-white font-medium">{offering.product_type}</span>
						</div>
						<div class="flex items-center justify-between text-white/70">
							<span>Price</span>
							<span class="text-white font-medium">{formatPrice(offering)}</span>
						</div>
						{#if offering.datacenter_country}
							<div class="flex items-center justify-between text-white/70">
								<span>Location</span>
								<span class="text-white font-medium">{offering.datacenter_city}, {offering.datacenter_country}</span>
							</div>
						{/if}
						{#if offering.description}
							<div class="text-white/60 text-xs mt-2 line-clamp-2">{offering.description}</div>
						{/if}
					</div>
				</div>
			{/each}
		</div>

		<!-- Empty State (if no offerings) -->
		{#if offerings.length === 0}
			<div class="text-center py-16">
				<span class="text-6xl mb-4 block">📦</span>
				<h3 class="text-2xl font-bold text-white mb-2">No Offerings Yet</h3>
				<p class="text-white/60 mb-6">Create your first cloud service offering to get started</p>
				<button
					onclick={openEditor}
					class="px-6 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold hover:brightness-110 hover:scale-105 transition-all"
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
