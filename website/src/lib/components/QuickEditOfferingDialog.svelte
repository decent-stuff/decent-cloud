<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { Offering, CreateOfferingParams } from '$lib/services/api';
	import { updateProviderOffering, hexEncode } from '$lib/services/api';
	import { signRequest } from '$lib/services/auth-api';
	import type { Ed25519KeyIdentity } from '@dfinity/identity';

	interface Props {
		open?: boolean;
		offering: Offering | null;
		identity: Ed25519KeyIdentity | null;
		pubkeyBytes: Uint8Array | null;
	}

	let { open = $bindable(false), offering, identity, pubkeyBytes }: Props = $props();

	const dispatch = createEventDispatcher<{
		success: void;
		close: void;
	}>();

	let saving = $state(false);
	let error = $state<string | null>(null);

	// Editable fields
	let offerName = $state('');
	let description = $state('');
	let monthlyPrice = $state(0);
	let setupFee = $state(0);
	let stockStatus = $state('in_stock');
	let visibility = $state('public');
	let templateName = $state('');

	// Check if template name is non-numeric (won't auto-generate provisioner_config)
	let templateWarning = $derived(
		templateName.trim() !== '' && !/^\d+$/.test(templateName.trim())
	);

	$effect(() => {
		if (offering && open) {
			offerName = offering.offer_name;
			description = offering.description || '';
			monthlyPrice = offering.monthly_price;
			setupFee = offering.setup_fee;
			stockStatus = offering.stock_status;
			visibility = offering.visibility;
			templateName = offering.template_name || '';
			error = null;
		}
	});

	async function handleSave() {
		if (!identity || !pubkeyBytes || !offering) {
			error = 'Missing authentication or offering data';
			return;
		}

		if (!offerName.trim()) {
			error = 'Offer name is required';
			return;
		}

		saving = true;
		error = null;

		try {
			const pubkeyHex = hexEncode(pubkeyBytes);
			const path = `/api/v1/providers/${pubkeyHex}/offerings/${offering.id}`;

			// Build update params - must include ALL fields
			const params: CreateOfferingParams = {
				offering_id: offering.offering_id,
				offer_name: offerName.trim(),
				description: description.trim() || undefined,
				product_page_url: offering.product_page_url || undefined,
				currency: offering.currency,
				monthly_price: monthlyPrice,
				setup_fee: setupFee,
				visibility,
				product_type: offering.product_type,
				virtualization_type: offering.virtualization_type || undefined,
				billing_interval: offering.billing_interval,
				stock_status: stockStatus,
				processor_brand: offering.processor_brand || undefined,
				processor_amount: offering.processor_amount || undefined,
				processor_cores: offering.processor_cores || undefined,
				processor_speed: offering.processor_speed || undefined,
				processor_name: offering.processor_name || undefined,
				memory_error_correction: offering.memory_error_correction || undefined,
				memory_type: offering.memory_type || undefined,
				memory_amount: offering.memory_amount || undefined,
				hdd_amount: offering.hdd_amount || undefined,
				total_hdd_capacity: offering.total_hdd_capacity || undefined,
				ssd_amount: offering.ssd_amount || undefined,
				total_ssd_capacity: offering.total_ssd_capacity || undefined,
				unmetered_bandwidth: offering.unmetered_bandwidth,
				uplink_speed: offering.uplink_speed || undefined,
				traffic: offering.traffic || undefined,
				datacenter_country: offering.datacenter_country,
				datacenter_city: offering.datacenter_city,
				datacenter_latitude: offering.datacenter_latitude || undefined,
				datacenter_longitude: offering.datacenter_longitude || undefined,
				control_panel: offering.control_panel || undefined,
				gpu_name: offering.gpu_name || undefined,
				gpu_count: offering.gpu_count || undefined,
				gpu_memory_gb: offering.gpu_memory_gb || undefined,
				min_contract_hours: undefined,
				max_contract_hours: undefined,
				payment_methods: offering.payment_methods || undefined,
				features: offering.features || undefined,
				operating_systems: offering.operating_systems || undefined,
				trust_score: undefined,
				has_critical_flags: undefined,
				is_example: offering.is_example,
				offering_source: offering.offering_source || undefined,
				external_checkout_url: offering.external_checkout_url || undefined,
				reseller_name: undefined,
				reseller_commission_percent: undefined,
				owner_username: undefined,
				provisioner_type: offering.provisioner_type || undefined,
				provisioner_config: offering.provisioner_config || undefined,
				template_name: templateName.trim() || undefined,
				agent_pool_id: offering.agent_pool_id || undefined,
				provider_online: undefined,
				// Subscription fields
				billing_unit: offering.billing_unit || 'month',
				pricing_model: offering.pricing_model || undefined,
				price_per_unit: offering.price_per_unit || undefined,
				included_units: offering.included_units || undefined,
				overage_price_per_unit: offering.overage_price_per_unit || undefined,
				stripe_metered_price_id: offering.stripe_metered_price_id || undefined,
				is_subscription: offering.is_subscription || false,
				subscription_interval_days: offering.subscription_interval_days || undefined
			};

			// Sign the request - this returns the exact JSON body that was signed
			const signed = await signRequest(identity, 'PUT', path, params);

			if (!signed.body) {
				throw new Error('Failed to sign request: signed body is empty');
			}

			if (offering.id === undefined) {
				throw new Error('Offering ID is required for update');
			}

			// CRITICAL: Use signed.body (the exact string that was signed) not params
			await updateProviderOffering(pubkeyBytes, offering.id, signed.body, signed.headers);

			dispatch('success');
			handleClose();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Update failed';
			console.error('Update error:', e);
		} finally {
			saving = false;
		}
	}

	function handleClose() {
		open = false;
		error = null;
		dispatch('close');
	}
</script>

{#if open && offering}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-base/70 backdrop-blur-sm p-4"
		onclick={(e) => e.target === e.currentTarget && handleClose()}
		role="dialog"
		aria-modal="true"
		tabindex="-1"
	>
		<div
			class="bg-gradient-to-br from-slate-900 to-slate-800  shadow-2xl border border-neutral-800 w-full max-w-2xl max-h-[90vh] overflow-y-auto"
		>
			<!-- Header -->
			<div class="flex items-center justify-between p-6 border-b border-neutral-800">
				<div>
					<h2 class="text-2xl font-bold text-white">Quick Edit Offering</h2>
					<p class="text-neutral-500 text-sm mt-1">
						Edit key fields (use CSV import for full editing)
					</p>
				</div>
				<button
					onclick={handleClose}
					class="text-neutral-500 hover:text-white transition-colors"
					aria-label="Close dialog"
				>
					<svg class="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M6 18L18 6M6 6l12 12"
						/>
					</svg>
				</button>
			</div>

			<!-- Content -->
			<div class="p-6 space-y-6">
				<!-- Offer Name -->
				<div>
					<label for="offer-name" class="block text-white font-medium mb-2">
						Offer Name <span class="text-red-400">*</span>
					</label>
					<input
						id="offer-name"
						type="text"
						bind:value={offerName}
						class="w-full px-4 py-3 bg-surface-elevated border border-neutral-800  text-white placeholder-white/40 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent"
						placeholder="e.g. Basic Virtual Machine"
						disabled={saving}
					/>
				</div>

				<!-- Description -->
				<div>
					<label for="description" class="block text-white font-medium mb-2">
						Description
					</label>
					<textarea
						id="description"
						bind:value={description}
						rows="3"
						class="w-full px-4 py-3 bg-surface-elevated border border-neutral-800  text-white placeholder-white/40 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent resize-none"
						placeholder="Describe your offering..."
						disabled={saving}
					></textarea>
				</div>

				<!-- Pricing -->
				<div class="grid grid-cols-2 gap-4">
					<div>
						<label for="monthly-price" class="block text-white font-medium mb-2">
							Monthly Price ({offering.currency})
						</label>
						<input
							id="monthly-price"
							type="number"
							bind:value={monthlyPrice}
							step="0.01"
							min="0"
							class="w-full px-4 py-3 bg-surface-elevated border border-neutral-800  text-white placeholder-white/40 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent"
							disabled={saving}
						/>
					</div>
					<div>
						<label for="setup-fee" class="block text-white font-medium mb-2">
							Setup Fee ({offering.currency})
						</label>
						<input
							id="setup-fee"
							type="number"
							bind:value={setupFee}
							step="0.01"
							min="0"
							class="w-full px-4 py-3 bg-surface-elevated border border-neutral-800  text-white placeholder-white/40 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent"
							disabled={saving}
						/>
					</div>
				</div>

				<!-- Status and Visibility -->
				<div class="grid grid-cols-2 gap-4">
					<div>
						<label for="stock-status" class="block text-white font-medium mb-2">
							Stock Status
						</label>
						<select
							id="stock-status"
							bind:value={stockStatus}
							class="w-full px-4 py-3 bg-surface-elevated border border-neutral-800  text-white focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent"
							disabled={saving}
						>
							<option value="in_stock">In Stock</option>
							<option value="out_of_stock">Out of Stock</option>
							<option value="discontinued">Discontinued</option>
						</select>
					</div>
					<div>
						<label for="visibility" class="block text-white font-medium mb-2">
							Visibility
						</label>
						<select
							id="visibility"
							bind:value={visibility}
							class="w-full px-4 py-3 bg-surface-elevated border border-neutral-800  text-white focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent"
							disabled={saving}
						>
							<option value="public">Public</option>
							<option value="private">Private</option>
						</select>
					</div>
				</div>

				<!-- VM Template for Instant Provisioning -->
				<div>
					<label for="template-name" class="block text-white font-medium mb-2">
						VM Template
					</label>
					<input
						id="template-name"
						type="text"
						bind:value={templateName}
						class="w-full px-4 py-3 bg-surface-elevated border border-neutral-800 text-white placeholder-white/40 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent"
						class:border-yellow-500={templateWarning}
						placeholder="e.g. 9000"
						disabled={saving}
					/>
					{#if templateWarning}
						<p class="text-yellow-500 text-sm mt-2">
							⚠ Non-numeric template names require manual provisioner_config setup. Use a numeric Proxmox VMID for automatic configuration.
						</p>
					{:else}
						<p class="text-neutral-500 text-sm mt-2">
							Proxmox template VMID for instant provisioning. Leave empty to use the default template from your dc-agent config.
						</p>
					{/if}
				</div>

				<!-- Error Display -->
				{#if error}
					<div class="bg-red-500/20 border border-red-500/30  p-4">
						<p class="text-red-400 font-semibold">Error</p>
						<p class="text-red-400/80 text-sm mt-1">{error}</p>
					</div>
				{/if}

				<!-- Info Note -->
				<div class="bg-primary-500/10 border border-primary-500/30  p-4">
					<p class="text-primary-400 text-sm">
						<strong>Note:</strong> This quick editor only shows common fields. To edit all fields
						(hardware specs, location, etc.), use the "Edit Offerings" spreadsheet editor.
					</p>
				</div>
			</div>

			<!-- Footer Actions -->
			<div class="flex items-center justify-end gap-3 p-6 border-t border-neutral-800">
				<button
					onclick={handleClose}
					class="px-6 py-3 bg-surface-elevated  font-medium hover:bg-surface-elevated transition-all"
					disabled={saving}
				>
					Cancel
				</button>
				<button
					onclick={handleSave}
					class="px-6 py-3 bg-gradient-to-r from-primary-500 to-primary-600  font-semibold hover:brightness-110 hover:scale-105 transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100"
					disabled={saving}
				>
					{#if saving}
						<span class="flex items-center gap-2">
							<svg class="animate-spin h-5 w-5" viewBox="0 0 24 24">
								<circle
									class="opacity-25"
									cx="12"
									cy="12"
									r="10"
									stroke="currentColor"
									stroke-width="4"
									fill="none"
								/>
								<path
									class="opacity-75"
									fill="currentColor"
									d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
								/>
							</svg>
							Saving...
						</span>
					{:else}
						Save Changes
					{/if}
				</button>
			</div>
		</div>
	</div>
{/if}
