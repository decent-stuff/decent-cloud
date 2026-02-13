<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { Offering, CreateOfferingParams, AllowlistEntry } from '$lib/services/api';
	import { updateProviderOffering, hexEncode, getOfferingAllowlist, addToAllowlist, removeFromAllowlist } from '$lib/services/api';
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
	let postProvisionScript = $state('');

	// Usage-based billing fields
	let billingUnit = $state('month');
	let pricingModel = $state<string | undefined>(undefined);
	let pricePerUnit = $state<number | undefined>(undefined);
	let includedUnits = $state<number | undefined>(undefined);
	let overagePricePerUnit = $state<number | undefined>(undefined);

	// Allowlist management
	let allowlistEntries = $state<AllowlistEntry[]>([]);
	let loadingAllowlist = $state(false);
	let newAllowedPubkey = $state('');
	let addingToAllowlist = $state(false);
	let removingPubkey = $state<string | null>(null);

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
			postProvisionScript = offering.post_provision_script || '';
			// Usage-based billing
			billingUnit = offering.billing_unit || 'month';
			pricingModel = offering.pricing_model || undefined;
			pricePerUnit = offering.price_per_unit || undefined;
			includedUnits = offering.included_units || undefined;
			overagePricePerUnit = offering.overage_price_per_unit || undefined;
			error = null;
			// Load allowlist if visibility is shared
			if (offering.visibility.toLowerCase() === 'shared') {
				loadAllowlist();
			} else {
				allowlistEntries = [];
			}
		}
	});

	// Load allowlist when visibility changes to shared
	$effect(() => {
		if (visibility.toLowerCase() === 'shared' && offering && open) {
			loadAllowlist();
		}
	});

	async function loadAllowlist() {
		if (!identity || !pubkeyBytes || !offering?.id) return;

		loadingAllowlist = true;
		try {
			const pubkeyHex = hexEncode(pubkeyBytes);
			const path = `/api/v1/providers/${pubkeyHex}/offerings/${offering.id}/allowlist`;
			const signed = await signRequest(identity, 'GET', path);
			allowlistEntries = await getOfferingAllowlist(pubkeyBytes, offering.id, signed.headers);
		} catch (e) {
			console.error('Failed to load allowlist:', e);
			// Don't show error - allowlist might just be empty
		} finally {
			loadingAllowlist = false;
		}
	}

	async function handleAddToAllowlist() {
		if (!identity || !pubkeyBytes || !offering?.id || !newAllowedPubkey.trim()) return;

		// Validate hex format (should be 64 characters for 32-byte Ed25519 key)
		const trimmed = newAllowedPubkey.trim();
		if (!/^[0-9a-fA-F]{64}$/.test(trimmed)) {
			error = 'Invalid public key format. Must be 64 hex characters (32-byte Ed25519 public key).';
			return;
		}

		addingToAllowlist = true;
		error = null;
		try {
			const pubkeyHex = hexEncode(pubkeyBytes);
			const path = `/api/v1/providers/${pubkeyHex}/offerings/${offering.id}/allowlist`;
			const params = { allowedPubkey: trimmed.toLowerCase() };
			const signed = await signRequest(identity, 'POST', path, params);
			if (!signed.body) throw new Error('Failed to sign request');
			await addToAllowlist(pubkeyBytes, offering.id, trimmed.toLowerCase(), signed.headers, signed.body);
			newAllowedPubkey = '';
			await loadAllowlist();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to add to allowlist';
		} finally {
			addingToAllowlist = false;
		}
	}

	async function handleRemoveFromAllowlist(allowedPubkey: string) {
		if (!identity || !pubkeyBytes || !offering?.id) return;

		removingPubkey = allowedPubkey;
		error = null;
		try {
			const pubkeyHex = hexEncode(pubkeyBytes);
			const path = `/api/v1/providers/${pubkeyHex}/offerings/${offering.id}/allowlist/${allowedPubkey}`;
			const signed = await signRequest(identity, 'DELETE', path);
			await removeFromAllowlist(pubkeyBytes, offering.id, allowedPubkey, signed.headers);
			await loadAllowlist();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to remove from allowlist';
		} finally {
			removingPubkey = null;
		}
	}

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
				post_provision_script: postProvisionScript.trim() || undefined,
				provider_online: undefined,
				// Usage-based billing fields (editable)
				billing_unit: billingUnit,
				pricing_model: pricingModel || undefined,
				price_per_unit: pricePerUnit || undefined,
				included_units: includedUnits || undefined,
				overage_price_per_unit: overagePricePerUnit || undefined,
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

				<!-- Usage-Based Billing -->
				<div class="border-t border-neutral-800 pt-4">
					<div class="flex items-center justify-between mb-3">
						<h3 class="text-white font-medium">Usage-Based Billing</h3>
						<label class="flex items-center gap-2 cursor-pointer">
							<input
								type="checkbox"
								checked={pricingModel === 'usage_overage'}
								onchange={(e) => pricingModel = e.currentTarget.checked ? 'usage_overage' : undefined}
								class="w-4 h-4 rounded border-neutral-600 bg-surface-elevated text-primary-500 focus:ring-primary-500"
								disabled={saving}
							/>
							<span class="text-sm text-neutral-400">Enable usage tracking</span>
						</label>
					</div>

					{#if pricingModel === 'usage_overage'}
						<div class="grid grid-cols-2 gap-4 mb-4">
							<div>
								<label for="billing-unit" class="block text-neutral-400 text-sm mb-1">
									Billing Unit
								</label>
								<select
									id="billing-unit"
									bind:value={billingUnit}
									class="w-full px-3 py-2 bg-surface-elevated border border-neutral-700 text-white text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent"
									disabled={saving}
								>
									<option value="minute">Minute</option>
									<option value="hour">Hour</option>
									<option value="day">Day</option>
									<option value="month">Month</option>
								</select>
							</div>
							<div>
								<label for="included-units" class="block text-neutral-400 text-sm mb-1">
									Included Units
								</label>
								<input
									id="included-units"
									type="number"
									bind:value={includedUnits}
									min="0"
									placeholder="0"
									class="w-full px-3 py-2 bg-surface-elevated border border-neutral-700 text-white placeholder-white/30 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent"
									disabled={saving}
								/>
							</div>
						</div>
						<div class="grid grid-cols-2 gap-4">
							<div>
								<label for="price-per-unit" class="block text-neutral-400 text-sm mb-1">
									Price per Unit ({offering.currency})
								</label>
								<input
									id="price-per-unit"
									type="number"
									bind:value={pricePerUnit}
									step="0.01"
									min="0"
									placeholder="0.00"
									class="w-full px-3 py-2 bg-surface-elevated border border-neutral-700 text-white placeholder-white/30 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent"
									disabled={saving}
								/>
							</div>
							<div>
								<label for="overage-price" class="block text-neutral-400 text-sm mb-1">
									Overage Price ({offering.currency})
								</label>
								<input
									id="overage-price"
									type="number"
									bind:value={overagePricePerUnit}
									step="0.01"
									min="0"
									placeholder="0.00"
									class="w-full px-3 py-2 bg-surface-elevated border border-neutral-700 text-white placeholder-white/30 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent"
									disabled={saving}
								/>
							</div>
						</div>
						<p class="text-neutral-500 text-xs mt-2">
							Usage beyond included units will be charged at the overage price.
						</p>
					{/if}
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
							<option value="public">Public (marketplace)</option>
							<option value="shared">Shared (allowlist only)</option>
							<option value="private">Private (owner only)</option>
						</select>
					</div>
				</div>

				<!-- Allowlist Management (shown when visibility is shared) -->
				{#if visibility.toLowerCase() === 'shared'}
					<div class="bg-blue-500/10 border border-blue-500/30 p-4 space-y-4">
						<div class="flex items-center justify-between">
							<h3 class="text-blue-400 font-semibold">Allowlist</h3>
							<span class="text-blue-400/60 text-sm">
								{allowlistEntries.length} user{allowlistEntries.length !== 1 ? 's' : ''} allowed
							</span>
						</div>
						<p class="text-blue-400/80 text-sm">
							Only users in this allowlist can see and rent this offering.
						</p>

						<!-- Add to allowlist -->
						<div class="flex gap-2">
							<input
								type="text"
								bind:value={newAllowedPubkey}
								placeholder="Enter user's public key (64 hex characters)"
								class="flex-1 px-3 py-2 bg-surface-elevated border border-neutral-700 text-white placeholder-white/40 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
								disabled={addingToAllowlist || saving}
							/>
							<button
								onclick={handleAddToAllowlist}
								class="px-4 py-2 bg-blue-500/20 border border-blue-500/30 text-blue-400 font-medium hover:bg-blue-500/30 transition-all disabled:opacity-50"
								disabled={addingToAllowlist || saving || !newAllowedPubkey.trim()}
							>
								{#if addingToAllowlist}
									Adding...
								{:else}
									Add
								{/if}
							</button>
						</div>

						<!-- Allowlist entries -->
						{#if loadingAllowlist}
							<div class="text-blue-400/60 text-sm text-center py-2">
								Loading allowlist...
							</div>
						{:else if allowlistEntries.length === 0}
							<div class="text-blue-400/60 text-sm text-center py-2">
								No users in allowlist yet. Add public keys to allow access.
							</div>
						{:else}
							<div class="space-y-2 max-h-32 overflow-y-auto">
								{#each allowlistEntries as entry}
									<div class="flex items-center justify-between bg-surface-elevated/50 px-3 py-2">
										<code class="text-xs text-blue-400/80 font-mono truncate flex-1 mr-2" title={entry.allowed_pubkey}>
											{entry.allowed_pubkey.slice(0, 16)}...{entry.allowed_pubkey.slice(-8)}
										</code>
										<button
											onclick={() => handleRemoveFromAllowlist(entry.allowed_pubkey)}
											class="text-red-400 hover:text-red-300 text-sm px-2 py-1 hover:bg-red-500/20 transition-colors"
											disabled={removingPubkey === entry.allowed_pubkey || saving}
											title="Remove from allowlist"
										>
											{#if removingPubkey === entry.allowed_pubkey}
												...
											{:else}
												×
											{/if}
										</button>
									</div>
								{/each}
							</div>
						{/if}
					</div>
				{/if}

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

				<!-- Post-Provision Script -->
				<div>
					<label for="post-provision-script" class="block text-white font-medium mb-2">
						Post-Provision Script
					</label>
					<textarea
						id="post-provision-script"
						bind:value={postProvisionScript}
						rows="6"
						class="w-full px-4 py-3 bg-surface-elevated border border-neutral-800 text-white placeholder-white/40 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent font-mono text-sm"
						placeholder="#!/bin/bash&#10;# Script to run after VM is provisioned&#10;apt-get update && apt-get install -y docker.io"
						disabled={saving}
					></textarea>
					<p class="text-neutral-500 text-sm mt-2">
						Script to execute via SSH after VM provisioning. Include a shebang (e.g., <code class="text-primary-400">#!/bin/bash</code>, <code class="text-primary-400">#!/usr/bin/env python3</code>) to specify the interpreter.
					</p>
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
