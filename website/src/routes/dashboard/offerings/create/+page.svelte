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
	import OfferingPreviewCard from '$lib/components/OfferingPreviewCard.svelte';
	import type { IdentityInfo } from '$lib/stores/auth';
	import { validateStep1, validateStep2, validateStep3 } from '$lib/utils/offering-wizard';
	import { RECIPE_TEMPLATES } from '$lib/data/recipe-templates';
	import { OFFERING_TEMPLATES } from '$lib/data/offering-templates';

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
	let isDraft = $state(false);
	let monthlyPrice = $state<number | null>(null);
	let currency = $state('USD');
	let setupFee = $state(0);

	// Recipe
	let postProvisionScript = $state('');
	let selectedTemplate = $state('');
	let selectedOfferingTemplate = $state('');

	// UI state
	let loading = $state(true);
	let submitting = $state(false);
	let error = $state<string | null>(null);
	let accountsLoaded = $state(false);
	let productTypes = $state<ProductType[]>([]);

	// Wizard state
	let currentStep = $state(1);
	const STEPS = [
		{ n: 1, label: 'Basics' },
		{ n: 2, label: 'Infrastructure' },
		{ n: 3, label: 'Pricing & Recipe' }
	] as const;

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

	function applyOfferingTemplate(key: string) {
		if (selectedOfferingTemplate === key) {
			selectedOfferingTemplate = '';
			return;
		}
		const tpl = OFFERING_TEMPLATES.find((t) => t.key === key);
		if (!tpl) return;
		selectedOfferingTemplate = key;
		offerName = tpl.offerName;
		description = tpl.offeringDescription;
		productType = tpl.productType;
		monthlyPrice = tpl.monthlyPrice;
		visibility = tpl.visibility;
		if (!offeringIdManuallyEdited) {
			offeringId = slugify(tpl.offerName);
		}
	}

	function applyTemplate() {
		const tpl = RECIPE_TEMPLATES.find((t) => t.key === selectedTemplate);
		if (tpl) {
			postProvisionScript = tpl.script;
			selectedTemplate = '';
		}
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

	function goNext() {
		error = null;
		if (currentStep === 1) {
			const err = validateStep1(offerName, offeringId);
			if (err) { error = err; return; }
		} else if (currentStep === 2) {
			const err = validateStep2({ selectedAccountId, selectedServerType, selectedLocation, selectedImage });
			if (err) { error = err; return; }
		}
		currentStep = Math.min(currentStep + 1, 3) as 1 | 2 | 3;
	}

	function goBack() {
		error = null;
		currentStep = Math.max(currentStep - 1, 1) as 1 | 2 | 3;
	}

	async function handleSubmit() {
		error = null;

		if (!currentIdentity?.identity || !currentIdentity?.publicKeyBytes) {
			error = 'Please authenticate first';
			return;
		}

		const step3Err = validateStep3(monthlyPrice);
		if (step3Err) { error = step3Err; return; }

		submitting = true;

		try {
			const pubkeyHex = hexEncode(currentIdentity.publicKeyBytes);
			const path = `/api/v1/providers/${pubkeyHex}/offerings`;

			const offering: CreateOfferingParams = {
				offering_id: offeringId,
				offer_name: offerName.trim(),
				description: description.trim() || undefined,
				product_page_url: undefined,
				currency,
				monthly_price: monthlyPrice!,
				setup_fee: setupFee,
				visibility,
				product_type: productType,
				virtualization_type: 'kvm',
				billing_interval: 'monthly',
				billing_unit: 'month',
				pricing_model: undefined,
				price_per_unit: undefined,
				included_units: undefined,
				overage_price_per_unit: undefined,
				stripe_metered_price_id: undefined,
				is_subscription: true,
				subscription_interval_days: 30,
				stock_status: 'in_stock',
				processor_brand: undefined,
				processor_amount: undefined,
				processor_cores: selectedServerType?.cores,
				processor_speed: undefined,
				processor_name: undefined,
				memory_error_correction: undefined,
				memory_type: undefined,
				memory_amount: selectedServerType ? `${selectedServerType.memoryGb} GB` : undefined,
				hdd_amount: undefined,
				total_hdd_capacity: undefined,
				ssd_amount: undefined,
				total_ssd_capacity: selectedServerType ? `${selectedServerType.diskGb} GB` : undefined,
				unmetered_bandwidth: false,
				uplink_speed: undefined,
				traffic: undefined,
				datacenter_country: selectedLocation?.country ?? '',
				datacenter_city: selectedLocation?.city ?? '',
				datacenter_latitude: undefined,
				datacenter_longitude: undefined,
				control_panel: undefined,
				gpu_name: undefined,
				gpu_count: undefined,
				gpu_memory_gb: undefined,
				min_contract_hours: undefined,
				max_contract_hours: undefined,
				payment_methods: undefined,
				features: undefined,
				operating_systems: selectedImage?.name,
				trust_score: undefined,
				has_critical_flags: undefined,
				reliability_score: undefined,
				is_example: false,
				is_draft: isDraft,
				offering_source: undefined,
				external_checkout_url: undefined,
				reseller_name: undefined,
				reseller_commission_percent: undefined,
				owner_username: undefined,
				provisioner_type: selectedServerType ? 'hetzner' : undefined,
				provisioner_config:
					selectedServerType && selectedLocation && selectedImage
						? JSON.stringify({
								server_type: selectedServerType.name,
								location: selectedLocation.name,
								image: selectedImage.name
							})
						: undefined,
				template_name: undefined,
				agent_pool_id: undefined,
				post_provision_script: postProvisionScript.trim() || undefined,
				provider_online: undefined,
				created_at_ns: undefined
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

	// Derived preview object for OfferingPreviewCard
	const previewOffering = $derived({
		offer_name: offerName,
		offering_id: offeringId,
		description,
		product_type: productType,
		monthly_price: monthlyPrice ?? undefined,
		currency,
		setup_fee: setupFee,
		datacenter_city: selectedLocation?.city,
		datacenter_country: selectedLocation?.country,
		processor_cores: selectedServerType?.cores,
		memory_amount: selectedServerType ? `${selectedServerType.memoryGb} GB` : undefined,
		total_ssd_capacity: selectedServerType ? `${selectedServerType.diskGb} GB` : undefined,
		post_provision_script: postProvisionScript || undefined,
		is_draft: isDraft
	});

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

<div class="space-y-6 max-w-4xl">
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

	{#if loading}
		<div class="flex justify-center items-center p-8">
			<div class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-primary-400"></div>
		</div>
	{:else if !currentIdentity}
		<div class="card p-8 border border-neutral-800 text-center">
			<p class="text-neutral-400">Please authenticate to create offerings.</p>
		</div>
	{:else}
		<!-- Step progress indicator -->
		<div class="flex items-center gap-0">
			{#each STEPS as step, i}
				<div class="flex items-center {i < STEPS.length - 1 ? 'flex-1' : ''}">
					<div class="flex items-center gap-2 shrink-0">
						<div
							class="w-7 h-7 rounded-full flex items-center justify-center text-xs font-bold transition-colors
							{currentStep === step.n
								? 'bg-primary-500 text-white'
								: currentStep > step.n
									? 'bg-primary-500/30 text-primary-400'
									: 'bg-neutral-800 text-neutral-500'}"
						>
							{#if currentStep > step.n}
								<Icon name="check" size={14} />
							{:else}
								{step.n}
							{/if}
						</div>
						<span
							class="text-sm font-medium transition-colors
							{currentStep === step.n ? 'text-white' : currentStep > step.n ? 'text-primary-400' : 'text-neutral-600'}"
						>
							{step.label}
						</span>
					</div>
					{#if i < STEPS.length - 1}
						<div class="flex-1 h-px mx-3 {currentStep > step.n ? 'bg-primary-500/40' : 'bg-neutral-800'}"></div>
					{/if}
				</div>
			{/each}
		</div>

		{#if error}
			<div class="bg-red-500/20 border border-red-500/30 p-4 text-red-400">
				<p class="font-semibold">Error</p>
				<p class="text-sm mt-1">{error}</p>
			</div>
		{/if}

		<!-- Step 1: Basics -->
		{#if currentStep === 1}
			<div class="card p-6 border border-neutral-800 space-y-5">
				<h2 class="text-lg font-semibold text-white flex items-center gap-2">
					<Icon name="package" size={20} class="text-primary-400" />
					Basics
				</h2>

				<!-- Offering Templates -->
				<div class="space-y-3">
					<p class="text-sm font-medium text-neutral-300">
						Start from a template <span class="text-neutral-600 font-normal">(optional)</span>
					</p>
					<div class="flex flex-wrap gap-2">
						{#each OFFERING_TEMPLATES as tpl}
							<button
								type="button"
								onclick={() => applyOfferingTemplate(tpl.key)}
								class="flex items-center gap-2 px-3 py-2 border transition-colors text-sm {selectedOfferingTemplate === tpl.key ? 'bg-primary-500/20 border-primary-500/50 text-primary-300' : 'bg-surface-elevated border-neutral-700 text-neutral-400 hover:text-white hover:border-neutral-500'}"
							>
								<span>{tpl.icon}</span>
								<div class="text-left">
									<div class="font-medium">{tpl.label}</div>
									<div class="text-xs text-neutral-500">{tpl.description}</div>
								</div>
							</button>
						{/each}
					</div>
				</div>

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

				<!-- Draft mode toggle -->
				<div class="flex items-center gap-3 p-3 bg-surface-elevated border border-neutral-700">
					<input type="checkbox" id="isDraft" bind:checked={isDraft} class="w-4 h-4 accent-primary-400" />
					<label for="isDraft" class="text-neutral-300 text-sm cursor-pointer">
						Save as draft <span class="text-neutral-500">(hidden from marketplace until published)</span>
					</label>
				</div>
			</div>

			<div class="flex items-center justify-between">
				<a href="/dashboard/offerings" class="text-neutral-400 hover:text-white transition-colors">
					Cancel
				</a>
				<button
					onclick={goNext}
					class="px-6 py-2.5 bg-gradient-to-r from-primary-500 to-primary-600 font-semibold hover:brightness-110 transition-all flex items-center gap-2"
				>
					Next: Infrastructure
					<Icon name="arrow-right" size={16} />
				</button>
			</div>

		<!-- Step 2: Infrastructure -->
		{:else if currentStep === 2}
			<div class="card p-6 border border-neutral-800 space-y-5">
				<h2 class="text-lg font-semibold text-white flex items-center gap-2">
					<Icon name="cpu" size={20} class="text-primary-400" />
					Infrastructure
				</h2>

				{#if accountsLoaded && cloudAccounts.length === 0}
					<div class="bg-amber-500/20 border border-amber-500/30 p-4 flex items-start gap-3">
						<Icon name="alert" size={20} class="text-amber-400 shrink-0 mt-0.5" />
						<div>
							<p class="text-amber-400 font-medium">No Hetzner cloud accounts found</p>
							<p class="text-amber-400/80 text-sm mt-1">
								<a href="/dashboard/cloud/accounts" class="underline hover:text-amber-300">
									Connect a Hetzner account
								</a> to create auto-provisioned offerings. You can still proceed without one.
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
							<div class="animate-spin rounded-full h-4 w-4 border-t-2 border-b-2 border-primary-400"></div>
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
							<div class="bg-primary-500/10 border border-primary-500/20 p-4 text-sm space-y-1">
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

			<div class="flex items-center justify-between">
				<button onclick={goBack} class="flex items-center gap-2 text-neutral-400 hover:text-white transition-colors">
					<Icon name="arrow-left" size={16} />
					Back
				</button>
				<button
					onclick={goNext}
					class="px-6 py-2.5 bg-gradient-to-r from-primary-500 to-primary-600 font-semibold hover:brightness-110 transition-all flex items-center gap-2"
				>
					Next: Pricing & Recipe
					<Icon name="arrow-right" size={16} />
				</button>
			</div>

		<!-- Step 3: Pricing & Recipe -->
		{:else}
			<div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
				<!-- Left: form -->
				<div class="space-y-5">
					<!-- Pricing card -->
					<div class="card p-6 border border-neutral-800 space-y-5">
						<h2 class="text-lg font-semibold text-white flex items-center gap-2">
							<Icon name="wallet" size={20} class="text-primary-400" />
							Pricing
						</h2>

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

						<!-- Draft toggle (echo from step 1 for convenience) -->
						<div class="flex items-center gap-3 p-3 bg-surface-elevated border border-neutral-700">
							<input type="checkbox" id="isDraft2" bind:checked={isDraft} class="w-4 h-4 accent-primary-400" />
							<label for="isDraft2" class="text-neutral-300 text-sm cursor-pointer">
								Save as draft <span class="text-neutral-500">(hidden from marketplace until published)</span>
							</label>
						</div>
					</div>

					<!-- Recipe card -->
					<div class="card p-6 border border-neutral-800 space-y-5">
						<h2 class="text-lg font-semibold text-white flex items-center gap-2">
							<Icon name="code" size={20} class="text-primary-400" />
							Recipe <span class="text-neutral-600 text-base font-normal">(Optional)</span>
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
				</div>

				<!-- Right: live marketplace preview -->
				<div class="lg:sticky lg:top-6 space-y-4 self-start">
					<OfferingPreviewCard offering={previewOffering} />
				</div>
			</div>

			<div class="flex items-center justify-between">
				<button onclick={goBack} class="flex items-center gap-2 text-neutral-400 hover:text-white transition-colors">
					<Icon name="arrow-left" size={16} />
					Back
				</button>
				<button
					onclick={handleSubmit}
					disabled={submitting}
					class="px-8 py-3 bg-gradient-to-r from-primary-500 to-primary-600 font-semibold hover:brightness-110 hover:scale-105 transition-all disabled:opacity-50 disabled:hover:scale-100 disabled:hover:brightness-100 flex items-center gap-2"
				>
					{#if submitting}
						<div class="animate-spin rounded-full h-4 w-4 border-t-2 border-b-2 border-white"></div>
						Creating...
					{:else if isDraft}
						Save as Draft
					{:else}
						Create Offering
					{/if}
				</button>
			</div>
		{/if}
	{/if}
</div>
