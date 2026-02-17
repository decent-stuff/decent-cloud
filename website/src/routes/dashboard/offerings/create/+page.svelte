<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import {
		createProviderOffering,
		listCloudAccounts,
		getCloudAccountCatalog,
		getProductTypes,
		type CloudAccount,
		type BackendCatalog,
		type ServerType,
		type Location,
		type Image,
		type CreateOfferingParams,
		type ProductType
	} from '$lib/services/api';
	import { hexEncode } from '$lib/services/api';
	import { authStore } from '$lib/stores/auth';
	import { signRequest } from '$lib/services/auth-api';
	import Icon from '$lib/components/Icons.svelte';
	import type { IdentityInfo } from '$lib/stores/auth';

	// Auth
	let currentIdentity = $state<IdentityInfo | null>(null);

	// Cloud accounts & catalog
	let cloudAccounts = $state<CloudAccount[]>([]);
	let selectedAccountId = $state('');
	let catalog = $state<BackendCatalog | null>(null);
	let catalogLoading = $state(false);
	let catalogError = $state<string | null>(null);

	// Catalog selections
	let selectedServerType = $state<ServerType | null>(null);
	let selectedLocation = $state<Location | null>(null);
	let selectedImage = $state<Image | null>(null);

	// Offering details
	let offerName = $state('');
	let offeringId = $state('');
	let offeringIdManuallyEdited = $state(false);
	let description = $state('');
	let productType = $state('compute');
	let visibility = $state('private');
	let monthlyPrice = $state<number | null>(null);
	let currency = $state('USD');
	let setupFee = $state(0);

	// Recipe
	let postProvisionScript = $state('');

	// UI state
	let loading = $state(true);
	let submitting = $state(false);
	let error = $state<string | null>(null);
	let accountsLoaded = $state(false);
	let productTypes = $state<ProductType[]>([]);

	function slugify(text: string): string {
		return text
			.toLowerCase()
			.trim()
			.replace(/[^a-z0-9]+/g, '-')
			.replace(/^-+|-+$/g, '');
	}

	function handleNameBlur() {
		if (offerName && !offeringIdManuallyEdited) {
			offeringId = slugify(offerName);
		}
	}

	function handleOfferingIdInput() {
		offeringIdManuallyEdited = true;
	}

	async function loadCloudAccounts() {
		if (!currentIdentity?.identity || !currentIdentity?.publicKeyBytes) return;

		const path = '/api/v1/cloud-accounts';
		const signed = await signRequest(currentIdentity.identity, 'GET', path);
		const accounts = await listCloudAccounts(signed.headers);
		cloudAccounts = accounts.filter((a) => a.backendType === 'hetzner');
		accountsLoaded = true;
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

		if (!currentIdentity?.identity || !currentIdentity?.publicKeyBytes) {
			error = 'Please authenticate first';
			return;
		}
		if (!offerName.trim()) {
			error = 'Offer name is required';
			return;
		}
		if (!offeringId.trim()) {
			error = 'Offering ID is required';
			return;
		}
		if (monthlyPrice === null || monthlyPrice <= 0) {
			error = 'Monthly price must be greater than 0';
			return;
		}
		if (!selectedServerType || !selectedLocation || !selectedImage) {
			error = 'Please select server type, location, and image';
			return;
		}

		submitting = true;

		try {
			const pubkeyHex = hexEncode(currentIdentity.publicKeyBytes);
			const path = `/api/v1/providers/${pubkeyHex}/offerings`;

			const offering: CreateOfferingParams = {
				offering_id: offeringId,
				offer_name: offerName.trim(),
				description: description.trim() || null,
				product_page_url: null,
				currency,
				monthly_price: monthlyPrice,
				setup_fee: setupFee,
				visibility,
				product_type: productType,
				virtualization_type: 'kvm',
				billing_interval: 'monthly',
				billing_unit: 'month',
				pricing_model: null,
				price_per_unit: undefined,
				included_units: undefined,
				overage_price_per_unit: undefined,
				stripe_metered_price_id: null,
				is_subscription: true,
				subscription_interval_days: 30,
				stock_status: 'in_stock',
				// Hardware specs from selected server type
				processor_brand: null,
				processor_amount: undefined,
				processor_cores: selectedServerType.cores,
				processor_speed: null,
				processor_name: null,
				memory_error_correction: null,
				memory_type: null,
				memory_amount: `${selectedServerType.memoryGb} GB`,
				hdd_amount: undefined,
				total_hdd_capacity: null,
				ssd_amount: undefined,
				total_ssd_capacity: `${selectedServerType.diskGb} GB`,
				unmetered_bandwidth: false,
				uplink_speed: null,
				traffic: undefined,
				// Location from selected location
				datacenter_country: selectedLocation.country,
				datacenter_city: selectedLocation.city,
				datacenter_latitude: null,
				datacenter_longitude: null,
				control_panel: null,
				gpu_name: null,
				gpu_count: undefined,
				gpu_memory_gb: undefined,
				min_contract_hours: undefined,
				max_contract_hours: undefined,
				payment_methods: null,
				features: null,
				operating_systems: selectedImage.name,
				trust_score: undefined,
				has_critical_flags: undefined,
				is_example: false,
				offering_source: undefined,
				external_checkout_url: null,
				reseller_name: undefined,
				reseller_commission_percent: undefined,
				owner_username: undefined,
				// Provisioner config
				provisioner_type: 'hetzner',
				provisioner_config: JSON.stringify({
					server_type: selectedServerType.name,
					location: selectedLocation.name,
					image: selectedImage.name
				}),
				template_name: null,
				agent_pool_id: null,
				post_provision_script: postProvisionScript.trim() || null,
				provider_online: undefined
			};

			const signed = await signRequest(currentIdentity.identity, 'POST', path, offering);
			await createProviderOffering(pubkeyHex, signed.body, signed.headers);
			goto('/dashboard/offerings');
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to create offering';
		} finally {
			submitting = false;
		}
	}

	onMount(() => {
		const unsubscribe = authStore.currentIdentity.subscribe(async (identity) => {
			currentIdentity = identity;
			if (identity) {
				try {
					await loadCloudAccounts();
					productTypes = await getProductTypes();
				} catch (e) {
					error = e instanceof Error ? e.message : 'Failed to load data';
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
			<h1 class="text-2xl font-bold text-white tracking-tight">Create Offering</h1>
			<p class="text-neutral-500">Configure infrastructure, pricing, and optional recipe script</p>
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
			<p class="text-neutral-400">Please authenticate to create offerings.</p>
		</div>
	{:else}
		<!-- Section 1: Infrastructure -->
		<div class="card p-6 border border-neutral-800 space-y-5">
			<h2 class="text-lg font-semibold text-white flex items-center gap-2">
				<Icon name="cpu" size={20} class="text-primary-400" />
				Infrastructure
			</h2>

			{#if accountsLoaded && cloudAccounts.length === 0}
				<div
					class="bg-amber-500/20 border border-amber-500/30 p-4 flex items-start gap-3"
				>
					<Icon name="alert" size={20} class="text-amber-400 shrink-0 mt-0.5" />
					<div>
						<p class="text-amber-400 font-medium">No Hetzner cloud accounts found</p>
						<p class="text-amber-400/80 text-sm mt-1">
							<a
								href="/dashboard/cloud/accounts"
								class="underline hover:text-amber-300"
							>
								Connect a Hetzner account
							</a> to create auto-provisioned offerings.
						</p>
					</div>
				</div>
			{:else if cloudAccounts.length > 0}
				<!-- Cloud Account -->
				<div>
					<label for="cloud-account" class="block text-sm font-medium text-neutral-400 mb-1.5"
						>Cloud Account</label
					>
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
						<div
							class="animate-spin rounded-full h-4 w-4 border-t-2 border-b-2 border-primary-400"
						></div>
						Loading server catalog...
					</div>
				{/if}

				{#if catalogError}
					<div class="text-red-400 text-sm">{catalogError}</div>
				{/if}

				{#if catalog}
					<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
						<!-- Server Type -->
						<div>
							<label for="server-type" class="block text-sm font-medium text-neutral-400 mb-1.5"
								>Server Type</label
							>
							<select
								id="server-type"
								onchange={handleServerTypeChange}
								class="w-full bg-surface-elevated border border-neutral-700 text-white px-3 py-2 focus:border-primary-500 focus:outline-none"
							>
								<option value="">Select...</option>
								{#each catalog.serverTypes as st}
									<option value={st.name}>
										{st.name} — {st.cores}v, {st.memoryGb}GB, {st.diskGb}GB
										{#if st.priceMonthly}(${st.priceMonthly}/mo){/if}
									</option>
								{/each}
							</select>
						</div>

						<!-- Location -->
						<div>
							<label for="location" class="block text-sm font-medium text-neutral-400 mb-1.5"
								>Location</label
							>
							<select
								id="location"
								onchange={handleLocationChange}
								class="w-full bg-surface-elevated border border-neutral-700 text-white px-3 py-2 focus:border-primary-500 focus:outline-none"
							>
								<option value="">Select...</option>
								{#each catalog.locations as loc}
									<option value={loc.name}>{loc.city}, {loc.country}</option>
								{/each}
							</select>
						</div>

						<!-- Image -->
						<div>
							<label for="image" class="block text-sm font-medium text-neutral-400 mb-1.5"
								>Image</label
							>
							<select
								id="image"
								onchange={handleImageChange}
								class="w-full bg-surface-elevated border border-neutral-700 text-white px-3 py-2 focus:border-primary-500 focus:outline-none"
							>
								<option value="">Select...</option>
								{#each catalog.images as img}
									<option value={img.name}>
										{img.name}
										{#if img.osVersion}({img.osVersion}){/if}
									</option>
								{/each}
							</select>
						</div>
					</div>

					<!-- Spec summary card -->
					{#if selectedServerType}
						<div
							class="bg-primary-500/10 border border-primary-500/20 p-4 text-sm space-y-1"
						>
							<p class="text-primary-400 font-medium mb-2">Selected Configuration</p>
							<div class="grid grid-cols-2 md:grid-cols-4 gap-3 text-neutral-300">
								<div>
									<span class="text-neutral-500 text-xs block">vCPUs</span>
									{selectedServerType.cores}
								</div>
								<div>
									<span class="text-neutral-500 text-xs block">Memory</span>
									{selectedServerType.memoryGb} GB
								</div>
								<div>
									<span class="text-neutral-500 text-xs block">SSD</span>
									{selectedServerType.diskGb} GB
								</div>
								{#if selectedServerType.priceMonthly}
									<div>
										<span class="text-neutral-500 text-xs block">Hetzner Cost</span>
										${selectedServerType.priceMonthly}/mo
									</div>
								{/if}
							</div>
						</div>
					{/if}
				{/if}
			{/if}
		</div>

		<!-- Section 2: Offering Details -->
		<div class="card p-6 border border-neutral-800 space-y-5">
			<h2 class="text-lg font-semibold text-white flex items-center gap-2">
				<Icon name="package" size={20} class="text-primary-400" />
				Offering Details
			</h2>

			<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
				<!-- Offer Name -->
				<div>
					<label for="offer-name" class="block text-sm font-medium text-neutral-400 mb-1.5"
						>Offer Name <span class="text-red-400">*</span></label
					>
					<input
						id="offer-name"
						type="text"
						bind:value={offerName}
						onblur={handleNameBlur}
						placeholder="e.g. WordPress on Ubuntu"
						class="w-full bg-surface-elevated border border-neutral-700 text-white px-3 py-2 focus:border-primary-500 focus:outline-none"
					/>
				</div>

				<!-- Offering ID -->
				<div>
					<label for="offering-id" class="block text-sm font-medium text-neutral-400 mb-1.5"
						>Offering ID <span class="text-red-400">*</span></label
					>
					<input
						id="offering-id"
						type="text"
						bind:value={offeringId}
						oninput={handleOfferingIdInput}
						placeholder="auto-generated from name"
						class="w-full bg-surface-elevated border border-neutral-700 text-white px-3 py-2 focus:border-primary-500 focus:outline-none font-mono text-sm"
					/>
				</div>
			</div>

			<!-- Description -->
			<div>
				<label for="description" class="block text-sm font-medium text-neutral-400 mb-1.5"
					>Description</label
				>
				<textarea
					id="description"
					bind:value={description}
					rows={3}
					placeholder="Describe what this offering provides..."
					class="w-full bg-surface-elevated border border-neutral-700 text-white px-3 py-2 focus:border-primary-500 focus:outline-none resize-y"
				></textarea>
			</div>

			<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
				<!-- Product Type -->
				<div>
					<label for="product-type" class="block text-sm font-medium text-neutral-400 mb-1.5"
						>Product Type</label
					>
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

				<!-- Visibility -->
				<div>
					<label for="visibility" class="block text-sm font-medium text-neutral-400 mb-1.5"
						>Visibility</label
					>
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

				<!-- Currency -->
				<div>
					<label for="currency" class="block text-sm font-medium text-neutral-400 mb-1.5"
						>Currency</label
					>
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

			<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
				<!-- Monthly Price -->
				<div>
					<label for="monthly-price" class="block text-sm font-medium text-neutral-400 mb-1.5"
						>Monthly Price <span class="text-red-400">*</span></label
					>
					<input
						id="monthly-price"
						type="number"
						bind:value={monthlyPrice}
						min="0.01"
						step="0.01"
						placeholder="0.00"
						class="w-full bg-surface-elevated border border-neutral-700 text-white px-3 py-2 focus:border-primary-500 focus:outline-none"
					/>
					{#if selectedServerType?.priceMonthly}
						<p class="text-neutral-500 text-xs mt-1">
							Hetzner cost: ${selectedServerType.priceMonthly}/mo
						</p>
					{/if}
				</div>

				<!-- Setup Fee -->
				<div>
					<label for="setup-fee" class="block text-sm font-medium text-neutral-400 mb-1.5"
						>Setup Fee</label
					>
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
				Recipe (Optional)
			</h2>

			<div>
				<label for="post-provision-script" class="block text-sm font-medium text-neutral-400 mb-1.5"
					>Post-Provision Script</label
				>
				<textarea
					id="post-provision-script"
					bind:value={postProvisionScript}
					rows={10}
					placeholder="#!/bin/bash&#10;# Script executed as root via SSH after VM boots&#10;apt-get update && apt-get install -y ..."
					class="w-full bg-surface-elevated border border-neutral-700 text-white px-3 py-2 focus:border-primary-500 focus:outline-none resize-y font-mono text-sm"
				></textarea>
				<p class="text-neutral-500 text-xs mt-1">
					Shell script executed as root via SSH after the VM boots. Include a shebang line (e.g.
					<code class="text-neutral-400">#!/bin/bash</code>).
				</p>
			</div>
		</div>

		<!-- Submit -->
		<div class="flex items-center justify-between">
			<a href="/dashboard/offerings" class="text-neutral-400 hover:text-white transition-colors">
				Cancel
			</a>
			<button
				onclick={handleSubmit}
				disabled={submitting || cloudAccounts.length === 0 || !selectedServerType || !selectedLocation || !selectedImage || !offerName.trim() || !offeringId.trim() || !monthlyPrice || monthlyPrice <= 0}
				class="px-8 py-3 bg-gradient-to-r from-primary-500 to-primary-600 font-semibold hover:brightness-110 hover:scale-105 transition-all disabled:opacity-50 disabled:hover:scale-100 disabled:hover:brightness-100 flex items-center gap-2"
			>
				{#if submitting}
					<div
						class="animate-spin rounded-full h-4 w-4 border-t-2 border-b-2 border-white"
					></div>
					Creating...
				{:else}
					Create Offering
				{/if}
			</button>
		</div>
	{/if}
</div>
