<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import {
		getOffering,
		updateProviderOffering,
		listCloudAccounts,
		getCloudAccountCatalog,
		getProductTypes,
		type CloudAccount,
		type BackendCatalog,
		type ServerType,
		type Location,
		type Image,
		type ProductType
	} from '$lib/services/api';
	import { hexEncode } from '$lib/services/api';
	import { authStore } from '$lib/stores/auth';
	import { signRequest } from '$lib/services/auth-api';
	import Icon from '$lib/components/Icons.svelte';
	import OfferingPreviewCard from '$lib/components/OfferingPreviewCard.svelte';
	import type { IdentityInfo } from '$lib/stores/auth';
	import type { Offering } from '$lib/services/api';
	import {
		buildOfferingDraftDiff,
		type OfferingDraftSnapshot
	} from '$lib/utils/offering-draft-diff';

	const offeringDbId = parseInt($page.params.id ?? '', 10);

	// Auth
	let currentIdentity = $state<IdentityInfo | null>(null);

	// Existing offering
	let existing = $state<Offering | null>(null);

	// Cloud accounts & catalog (for infrastructure display)
	let cloudAccounts = $state<CloudAccount[]>([]);
	let selectedAccountId = $state('');
	let catalog = $state<BackendCatalog | null>(null);
	let catalogLoading = $state(false);
	let catalogError = $state<string | null>(null);

	// Catalog selections
	let selectedServerType = $state<ServerType | null>(null);
	let selectedLocation = $state<Location | null>(null);
	let selectedImage = $state<Image | null>(null);

	// Editable fields
	let offerName = $state('');
	let description = $state('');
	let productType = $state('compute');
	let visibility = $state('private');
	let isDraft = $state(false);
	let publishAt = $state("");
	let monthlyPrice = $state<number | null>(null);
	let currency = $state('USD');
	let setupFee = $state(0);
	let postProvisionScript = $state('');

	// UI state
	let loading = $state(true);
	let submitting = $state(false);
	let error = $state<string | null>(null);
	let productTypes = $state<ProductType[]>([]);

	// Recipe template state
	let selectedTemplate = $state('');
	import { RECIPE_TEMPLATES } from '$lib/data/recipe-templates';

	interface ProvisionerConfigSnapshot {
		server_type?: string;
		location?: string;
		image?: string;
	}

	// Derived preview object for OfferingPreviewCard
	const previewOffering = $derived({
		offer_name: offerName,
		offering_id: existing?.offering_id,
		description,
		product_type: productType,
		monthly_price: monthlyPrice ?? undefined,
		currency,
		setup_fee: setupFee,
		datacenter_city: selectedLocation?.city ?? existing?.datacenter_city,
		datacenter_country: selectedLocation?.country ?? existing?.datacenter_country,
		processor_cores: selectedServerType?.cores ?? existing?.processor_cores,
		memory_amount: selectedServerType ? `${selectedServerType.memoryGb} GB` : (existing?.memory_amount ?? undefined),
		total_ssd_capacity: selectedServerType ? `${selectedServerType.diskGb} GB` : (existing?.total_ssd_capacity ?? undefined),
		post_provision_script: postProvisionScript || undefined,
		is_draft: isDraft
	});

	const existingProvisionerConfig = $derived.by(() =>
		parseProvisionerConfig(existing?.provisioner_config ?? null)
	);

	const baselineDraftSnapshot = $derived.by<OfferingDraftSnapshot | null>(() => {
		if (!existing) {
			return null;
		}

		return {
			offerName: existing.offer_name,
			description: existing.description ?? '',
			productType: existing.product_type,
			visibility: existing.visibility,
			isDraft: existing.is_draft ?? false,
			publishAt: existing.publish_at ? existing.publish_at.slice(0, 16) : '',
			monthlyPrice: existing.monthly_price,
			currency: existing.currency,
			setupFee: existing.setup_fee,
			postProvisionScript: existing.post_provision_script ?? '',
			serverType: existingProvisionerConfig.server_type ?? '',
			location: existingProvisionerConfig.location ?? '',
			image: existingProvisionerConfig.image ?? ''
		};
	});

	const currentDraftSnapshot = $derived.by<OfferingDraftSnapshot | null>(() => {
		if (!existing) {
			return null;
		}

		const effectiveProvisionerConfig =
			selectedServerType && selectedLocation && selectedImage
				? {
						server_type: selectedServerType.name,
						location: selectedLocation.name,
						image: selectedImage.name
					}
				: existingProvisionerConfig;

		return {
			offerName,
			description,
			productType,
			visibility,
			isDraft,
			publishAt,
			monthlyPrice,
			currency,
			setupFee,
			postProvisionScript,
			serverType: effectiveProvisionerConfig.server_type ?? '',
			location: effectiveProvisionerConfig.location ?? '',
			image: effectiveProvisionerConfig.image ?? ''
		};
	});

	const draftDiffRows = $derived.by(() => {
		if (!baselineDraftSnapshot || !currentDraftSnapshot) {
			return [];
		}

		return buildOfferingDraftDiff(baselineDraftSnapshot, currentDraftSnapshot);
	});

	function applyTemplate() {
		const tpl = RECIPE_TEMPLATES.find((t) => t.key === selectedTemplate);
		if (tpl) {
			postProvisionScript = tpl.script;
			selectedTemplate = '';
		}
	}

	function parseProvisionerConfig(rawConfig: string | null): ProvisionerConfigSnapshot {
		if (!rawConfig) {
			return {};
		}

		try {
			const parsed = JSON.parse(rawConfig) as ProvisionerConfigSnapshot;
			return {
				server_type: parsed.server_type,
				location: parsed.location,
				image: parsed.image
			};
		} catch {
			return {};
		}
	}

	async function loadCloudAccounts() {
		if (!currentIdentity?.identity || !currentIdentity?.publicKeyBytes) return;
		const path = '/api/v1/cloud-accounts';
		const signed = await signRequest(currentIdentity.identity, 'GET', path);
		cloudAccounts = (await listCloudAccounts(signed.headers)).filter(
			(a) => a.backendType === 'hetzner'
		);
	}

	async function loadCatalog() {
		if (!selectedAccountId || !currentIdentity?.identity) return;

		catalogLoading = true;
		catalogError = null;
		catalog = null;
		selectedServerType = null;
		selectedLocation = null;
		selectedImage = null;

		try {
			const path = `/api/v1/cloud-accounts/${selectedAccountId}/catalog`;
			const signed = await signRequest(currentIdentity.identity, 'GET', path);
			catalog = await getCloudAccountCatalog(selectedAccountId, signed.headers);

			// Pre-select from existing provisioner_config
			if (existing?.provisioner_config) {
				try {
					const config = JSON.parse(existing.provisioner_config);
					selectedServerType =
						catalog?.serverTypes.find((s) => s.name === config.server_type) ?? null;
					selectedLocation =
						catalog?.locations.find((l) => l.name === config.location) ?? null;
					selectedImage = catalog?.images.find((i) => i.name === config.image) ?? null;
				} catch {
					// provisioner_config not parseable - ignore
				}
			}
		} catch (e) {
			catalogError = e instanceof Error ? e.message : 'Failed to load catalog';
		} finally {
			catalogLoading = false;
		}
	}

	function handleServerTypeChange(e: Event) {
		const name = (e.target as HTMLSelectElement).value;
		selectedServerType = catalog?.serverTypes.find((s) => s.name === name) ?? null;
	}

	function handleLocationChange(e: Event) {
		const name = (e.target as HTMLSelectElement).value;
		selectedLocation = catalog?.locations.find((l) => l.name === name) ?? null;
	}

	function handleImageChange(e: Event) {
		const name = (e.target as HTMLSelectElement).value;
		selectedImage = catalog?.images.find((i) => i.name === name) ?? null;
	}

	async function handleSubmit() {
		error = null;
		if (!currentIdentity?.identity || !currentIdentity?.publicKeyBytes || !existing) {
			error = 'Please authenticate first';
			return;
		}
		if (!offerName.trim()) {
			error = 'Offer name is required';
			return;
		}
		if (monthlyPrice === null || monthlyPrice <= 0) {
			error = 'Monthly price must be greater than 0';
			return;
		}

		submitting = true;
		try {
			const pubkeyHex = hexEncode(currentIdentity.publicKeyBytes);
			const path = `/api/v1/providers/${pubkeyHex}/offerings/${offeringDbId}`;

			// Build provisioner config from catalog selections if changed, else keep existing
			let provisionerConfig = existing.provisioner_config ?? null;
			if (selectedServerType && selectedLocation && selectedImage) {
				provisionerConfig = JSON.stringify({
					server_type: selectedServerType.name,
					location: selectedLocation.name,
					image: selectedImage.name
				});
			}

			const offering = {
				...existing,
				is_draft: isDraft,
				publish_at: isDraft && publishAt ? new Date(publishAt).toISOString() : null,
				offer_name: offerName.trim(),
				description: description.trim() || null,
				currency,
				monthly_price: monthlyPrice,
				setup_fee: setupFee,
				visibility,
				product_type: productType,
				post_provision_script: postProvisionScript.trim() || null,
				provisioner_config: provisionerConfig,
				// Update hardware specs if server type changed
				...(selectedServerType
					? {
							processor_cores: selectedServerType.cores,
							memory_amount: `${selectedServerType.memoryGb} GB`,
							total_ssd_capacity: `${selectedServerType.diskGb} GB`
						}
					: {}),
				...(selectedLocation
					? {
							datacenter_country: selectedLocation.country,
							datacenter_city: selectedLocation.city
						}
					: {}),
				...(selectedImage
					? {
							operating_systems: selectedImage.name
						}
					: {})
			};

			const signed = await signRequest(currentIdentity.identity, 'PUT', path, offering);
			await updateProviderOffering(pubkeyHex, offeringDbId, signed.body, signed.headers);
			goto('/dashboard/offerings');
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to update offering';
		} finally {
			submitting = false;
		}
	}

	onMount(() => {
		const unsubscribe = authStore.currentIdentity.subscribe(async (identity) => {
			currentIdentity = identity;
			if (identity) {
				try {
					// Load offering, cloud accounts, and product types in parallel
					const [offering, , pts] = await Promise.all([
						getOffering(offeringDbId),
						loadCloudAccounts(),
						getProductTypes()
					]);
					existing = offering;
					productTypes = pts;

					// Populate form fields from existing offering
					offerName = offering.offer_name;
					description = offering.description ?? '';
					productType = offering.product_type;
					visibility = offering.visibility;
					monthlyPrice = offering.monthly_price;
					currency = offering.currency;
					setupFee = offering.setup_fee;
					postProvisionScript = offering.post_provision_script ?? '';
					isDraft = offering.is_draft ?? false;
					// Convert ISO timestamp to datetime-local format (strip seconds and timezone)
					publishAt = offering.publish_at ? offering.publish_at.slice(0, 16) : '';
				} catch (e) {
					error = e instanceof Error ? e.message : 'Failed to load offering';
				}
			}
			loading = false;
		});
		return unsubscribe;
	});
</script>

<div class="space-y-8 max-w-3xl">
	<!-- Header -->
	<div class="flex items-center gap-4">
		<a
			href="/dashboard/offerings"
			class="p-2 hover:bg-surface-elevated transition-colors rounded"
			title="Back to offerings"
		>
			<Icon name="arrow-left" size={20} class="text-neutral-400" />
		</a>
		<div>
			<h1 class="text-2xl font-bold text-white tracking-tight">Edit Offering</h1>
			{#if existing}
				<p class="text-neutral-500">
					<span class="font-mono text-sm">{existing.offering_id}</span>
				</p>
			{/if}
		</div>
	</div>

	{#if error}
		<div class="bg-red-500/20 border border-red-500/30 p-4 text-red-400">
			<p class="font-semibold">Error</p>
			<p class="text-sm mt-1">{error}</p>
		</div>
	{/if}

	{#if loading}
		<div class="flex justify-center items-center p-8">
			<div
				class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-primary-400"
			></div>
		</div>
	{:else if !currentIdentity}
		<div class="card p-8 border border-neutral-800 text-center">
			<p class="text-neutral-400">Please authenticate to edit offerings.</p>
		</div>
	{:else if existing}
		<!-- Section 1: Infrastructure -->
		<div class="card p-6 border border-neutral-800 space-y-5">
			<h2 class="text-lg font-semibold text-white flex items-center gap-2">
				<Icon name="cpu" size={20} class="text-primary-400" />
				Infrastructure
			</h2>

			{#if existing.provisioner_type === 'hetzner'}
				<!-- Current config summary -->
				<div class="bg-primary-500/10 border border-primary-500/20 p-4 text-sm space-y-1">
					<p class="text-primary-400 font-medium mb-2">Current Configuration</p>
					<div class="grid grid-cols-2 md:grid-cols-4 gap-3 text-neutral-300">
						<div>
							<span class="text-neutral-500 text-xs block">Server Type</span>
							{existingProvisionerConfig.server_type ?? '—'}
						</div>
						<div>
							<span class="text-neutral-500 text-xs block">Location</span>
							{existingProvisionerConfig.location ?? '—'}
						</div>
						<div>
							<span class="text-neutral-500 text-xs block">Image</span>
							{existingProvisionerConfig.image ?? '—'}
						</div>
						<div>
							<span class="text-neutral-500 text-xs block">Provisioner</span>
							Hetzner
						</div>
					</div>
				</div>

				<!-- Optional: Change infrastructure -->
				{#if cloudAccounts.length > 0}
					<details class="text-sm">
						<summary class="text-neutral-400 cursor-pointer hover:text-white">
							Change infrastructure...
						</summary>
						<div class="mt-4 space-y-4">
							<div>
								<label for="cloud-account" class="block text-sm font-medium text-neutral-400 mb-1.5">
									Cloud Account
								</label>
								<select
									id="cloud-account"
									bind:value={selectedAccountId}
									onchange={loadCatalog}
									class="w-full bg-surface-elevated border border-neutral-700 text-white px-3 py-2 focus:border-primary-500 focus:outline-none"
								>
									<option value="">Select a Hetzner account...</option>
									{#each cloudAccounts as account}
										<option value={account.id}>
											{account.name} ({account.accountId})
											{#if !account.isValid}— Invalid{/if}
										</option>
									{/each}
								</select>
							</div>

							{#if catalogLoading}
								<div class="flex items-center gap-2 text-neutral-400 text-sm py-2">
									<div class="animate-spin rounded-full h-4 w-4 border-t-2 border-b-2 border-primary-400"></div>
									Loading server catalog...
								</div>
							{/if}

							{#if catalogError}
								<div class="text-red-400 text-sm">{catalogError}</div>
							{/if}

							{#if catalog}
								<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
									<div>
										<label for="server-type" class="block text-sm font-medium text-neutral-400 mb-1.5">Server Type</label>
										<select id="server-type" onchange={handleServerTypeChange} class="w-full bg-surface-elevated border border-neutral-700 text-white px-3 py-2 focus:border-primary-500 focus:outline-none">
											<option value="">Select...</option>
											{#each catalog.serverTypes as st}
												<option value={st.name} selected={st.name === selectedServerType?.name}>
													{st.name} — {st.cores}v, {st.memoryGb}GB, {st.diskGb}GB
													{#if st.priceMonthly}(${st.priceMonthly}/mo){/if}
												</option>
											{/each}
										</select>
									</div>
									<div>
										<label for="location" class="block text-sm font-medium text-neutral-400 mb-1.5">Location</label>
										<select id="location" onchange={handleLocationChange} class="w-full bg-surface-elevated border border-neutral-700 text-white px-3 py-2 focus:border-primary-500 focus:outline-none">
											<option value="">Select...</option>
											{#each catalog.locations as loc}
												<option value={loc.name} selected={loc.name === selectedLocation?.name}>
													{loc.city}, {loc.country}
												</option>
											{/each}
										</select>
									</div>
									<div>
										<label for="image" class="block text-sm font-medium text-neutral-400 mb-1.5">Image</label>
										<select id="image" onchange={handleImageChange} class="w-full bg-surface-elevated border border-neutral-700 text-white px-3 py-2 focus:border-primary-500 focus:outline-none">
											<option value="">Select...</option>
											{#each catalog.images as img}
												<option value={img.name} selected={img.name === selectedImage?.name}>
													{img.name}
													{#if img.osVersion}({img.osVersion}){/if}
												</option>
											{/each}
										</select>
									</div>
								</div>
							{/if}
						</div>
					</details>
				{/if}
			{:else}
				<div class="text-neutral-400 text-sm">
					Provisioner: <span class="text-white">{existing.provisioner_type ?? 'None'}</span>
				</div>
			{/if}
		</div>

		<!-- Section 2: Offering Details -->
		<div class="card p-6 border border-neutral-800 space-y-5">
			<h2 class="text-lg font-semibold text-white flex items-center gap-2">
				<Icon name="package" size={20} class="text-primary-400" />
				Offering Details
			</h2>

			<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
				<div>
					<label for="offer-name" class="block text-sm font-medium text-neutral-400 mb-1.5">
						Offer Name <span class="text-red-400">*</span>
					</label>
					<input
						id="offer-name"
						type="text"
						bind:value={offerName}
						placeholder="e.g. WordPress on Ubuntu"
						class="w-full bg-surface-elevated border border-neutral-700 text-white px-3 py-2 focus:border-primary-500 focus:outline-none"
					/>
				</div>

				<div>
					<label for="offering-id" class="block text-sm font-medium text-neutral-400 mb-1.5">
						Offering ID
					</label>
					<input
						id="offering-id"
						type="text"
						value={existing.offering_id}
						disabled
						class="w-full bg-surface-elevated border border-neutral-700 text-neutral-500 px-3 py-2 font-mono text-sm cursor-not-allowed"
					/>
				</div>
			</div>

			<div>
				<label for="description" class="block text-sm font-medium text-neutral-400 mb-1.5">
					Description
				</label>
				<textarea
					id="description"
					bind:value={description}
					rows={3}
					placeholder="Describe what this offering provides..."
					class="w-full bg-surface-elevated border border-neutral-700 text-white px-3 py-2 focus:border-primary-500 focus:outline-none resize-y"
				></textarea>
			</div>

			<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
				<div>
					<label for="product-type" class="block text-sm font-medium text-neutral-400 mb-1.5">
						Product Type
					</label>
					<select
						id="product-type"
						bind:value={productType}
						class="w-full bg-surface-elevated border border-neutral-700 text-white px-3 py-2 focus:border-primary-500 focus:outline-none"
					>
						{#if productTypes.length > 0}
							{#each productTypes as pt}
								<option value={pt.key}>{pt.label}</option>
							{/each}
						{:else}
							<option value="compute">Compute</option>
							<option value="gpu">GPU</option>
							<option value="storage">Storage</option>
							<option value="network">Network</option>
							<option value="dedicated">Dedicated</option>
						{/if}
					</select>
				</div>

				<div>
					<label for="visibility" class="block text-sm font-medium text-neutral-400 mb-1.5">
						Visibility
					</label>
					<select
						id="visibility"
						bind:value={visibility}
						class="w-full bg-surface-elevated border border-neutral-700 text-white px-3 py-2 focus:border-primary-500 focus:outline-none"
					>
						<option value="private">Private (testing)</option>
						<option value="public">Public</option>
						<option value="shared">Shared</option>
					</select>
				</div>

				<div>
					<label for="currency" class="block text-sm font-medium text-neutral-400 mb-1.5">
						Currency
					</label>
					<select
						id="currency"
						bind:value={currency}
						class="w-full bg-surface-elevated border border-neutral-700 text-white px-3 py-2 focus:border-primary-500 focus:outline-none"
					>
						<option value="USD">USD</option>
						<option value="EUR">EUR</option>
					</select>
				</div>
			</div>

			<!-- Draft mode toggle -->
			<div class="space-y-3">
				<div class="flex items-center gap-3 p-3 bg-surface-elevated border border-neutral-700">
					<input type="checkbox" id="isDraft" bind:checked={isDraft} class="w-4 h-4 accent-primary-400" />
					<label for="isDraft" class="text-neutral-300 text-sm cursor-pointer">
						Save as draft <span class="text-neutral-500">(hidden from marketplace until published)</span>
					</label>
				</div>
				{#if isDraft}
					<div>
						<label for="publishAt" class="block text-sm font-medium text-neutral-400 mb-1.5">
							Schedule publish
						</label>
						<input
							id="publishAt"
							type="datetime-local"
							bind:value={publishAt}
							class="w-full bg-surface-elevated border border-neutral-700 text-white px-3 py-2 focus:border-primary-500 focus:outline-none"
						/>
						<p class="text-xs text-neutral-500 mt-1">If set, this draft will automatically publish at the specified date and time.</p>
					</div>
				{/if}
			</div>

			<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
				<div>
					<label for="monthly-price" class="block text-sm font-medium text-neutral-400 mb-1.5">
						Monthly Price <span class="text-red-400">*</span>
					</label>
					<input
						id="monthly-price"
						type="number"
						bind:value={monthlyPrice}
						min="0.01"
						step="0.01"
						placeholder="0.00"
						class="w-full bg-surface-elevated border border-neutral-700 text-white px-3 py-2 focus:border-primary-500 focus:outline-none"
					/>
				</div>

				<div>
					<label for="setup-fee" class="block text-sm font-medium text-neutral-400 mb-1.5">
						Setup Fee
					</label>
					<input
						id="setup-fee"
						type="number"
						bind:value={setupFee}
						min="0"
						step="0.01"
						placeholder="0.00"
						class="w-full bg-surface-elevated border border-neutral-700 text-white px-3 py-2 focus:border-primary-500 focus:outline-none"
					/>
				</div>
			</div>
		</div>

		<!-- Section 3: Recipe -->
		<div class="card p-6 border border-neutral-800 space-y-5">
			<h2 class="text-lg font-semibold text-white flex items-center gap-2">
				<Icon name="code" size={20} class="text-primary-400" />
				Recipe
			</h2>

			<!-- Template selector -->
			<div class="flex items-end gap-3">
				<div class="flex-1">
					<label for="recipe-template" class="block text-sm font-medium text-neutral-400 mb-1.5">
						Start from template
					</label>
					<select
						id="recipe-template"
						bind:value={selectedTemplate}
						class="w-full bg-surface-elevated border border-neutral-700 text-white px-3 py-2 focus:border-primary-500 focus:outline-none"
					>
						<option value="">Select a starter template...</option>
						{#each RECIPE_TEMPLATES as tpl}
							<option value={tpl.key}>{tpl.label}</option>
						{/each}
					</select>
				</div>
				<button
					onclick={applyTemplate}
					disabled={!selectedTemplate}
					class="px-4 py-2 bg-surface-elevated border border-neutral-700 text-neutral-300 hover:text-white hover:border-primary-500 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
				>
					Apply
				</button>
			</div>

			<div>
				<label for="post-provision-script" class="block text-sm font-medium text-neutral-400 mb-1.5">
					Post-Provision Script
				</label>
				<textarea
					id="post-provision-script"
					bind:value={postProvisionScript}
					rows={14}
					placeholder="#!/bin/bash&#10;# Script executed as root via SSH after VM boots&#10;apt-get update && apt-get install -y ..."
					class="w-full bg-surface-elevated border border-neutral-700 text-white px-3 py-2 focus:border-primary-500 focus:outline-none resize-y font-mono text-sm"
				></textarea>
				<p class="text-neutral-500 text-xs mt-1">
					Shell script executed as root via SSH after the VM boots. Include a shebang line (e.g.
					<code class="text-neutral-400">#!/bin/bash</code>).
				</p>
			</div>
		</div>

		<!-- Marketplace preview -->
	<OfferingPreviewCard offering={previewOffering} />

		<!-- Draft diff summary -->
		<div class="card p-6 border border-neutral-800 space-y-4">
			<h2 class="text-lg font-semibold text-white flex items-center gap-2">
				<Icon name="list" size={20} class="text-primary-400" />
				Changes Since Last Save
			</h2>
			{#if draftDiffRows.length === 0}
				<p class="text-sm text-neutral-400">No unsaved changes yet.</p>
			{:else}
				<p class="text-sm text-neutral-400">{draftDiffRows.length} field{draftDiffRows.length === 1 ? '' : 's'} changed.</p>
				<div class="space-y-3">
					{#each draftDiffRows as row (row.key)}
						<div class="border border-neutral-700 bg-surface-elevated p-4 space-y-2">
							<p class="text-sm font-semibold text-white">{row.label}</p>
							<div class="grid grid-cols-1 md:grid-cols-2 gap-3 text-sm">
								<div class="rounded border border-neutral-700 bg-base p-3 space-y-1">
									<p class="text-xs uppercase tracking-wide text-neutral-500">Before</p>
									<p class="text-neutral-200 whitespace-pre-wrap break-words">{row.before}</p>
								</div>
								<div class="rounded border border-primary-500/30 bg-primary-500/10 p-3 space-y-1">
									<p class="text-xs uppercase tracking-wide text-primary-300">After</p>
									<p class="text-neutral-100 whitespace-pre-wrap break-words">{row.after}</p>
								</div>
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</div>

	<!-- Submit -->
		<div class="flex items-center justify-between">
			<a href="/dashboard/offerings" class="text-neutral-400 hover:text-white transition-colors">
				Cancel
			</a>
			<button
				onclick={handleSubmit}
				disabled={submitting || !offerName.trim() || !monthlyPrice || monthlyPrice <= 0}
				class="px-8 py-3 bg-gradient-to-r from-primary-500 to-primary-600 font-semibold hover:brightness-110 hover:scale-105 transition-all disabled:opacity-50 disabled:hover:scale-100 disabled:hover:brightness-100 flex items-center gap-2"
			>
				{#if submitting}
					<div
						class="animate-spin rounded-full h-4 w-4 border-t-2 border-b-2 border-white"
					></div>
					Saving...
				{:else}
					Save Changes
				{/if}
			</button>
		</div>
	{/if}
</div>
