<script lang="ts">
	import type { Offering } from "$lib/services/api";
	import { createRentalRequest, type RentalRequestParams } from "$lib/services/api";
	import { signRequest } from "$lib/services/auth-api";
	import { authStore } from "$lib/stores/auth";
	import type { Ed25519KeyIdentity } from "@dfinity/identity";

	interface Props {
		offering: Offering | null;
		onClose: () => void;
		onSuccess: (contractId: string) => void;
	}

	let { offering, onClose, onSuccess }: Props = $props();

	let sshKey = $state("");
	let contactMethod = $state("");
	let durationHours = $state(720); // Default: 30 days
	let memo = $state("");
	let loading = $state(false);
	let error = $state<string | null>(null);

	function calculatePrice(): string {
		if (!offering) return "0.00";
		const price = (offering.monthly_price * durationHours) / 720;
		return price.toFixed(2);
	}

	async function handleSubmit() {
		if (!offering) return;

		const signingIdentityInfo = await authStore.getSigningIdentity();
		if (!signingIdentityInfo) {
			error = "You must be logged in to rent resources";
			return;
		}

		loading = true;
		error = null;

		try {
			const params: RentalRequestParams = {
				offering_db_id: offering.id!,
				ssh_pubkey: sshKey || undefined,
				contact_method: contactMethod || undefined,
				request_memo: memo || undefined,
				duration_hours: durationHours,
			};

			const signed = await signRequest(
				signingIdentityInfo.identity as Ed25519KeyIdentity,
				"POST",
				"/api/v1/contracts",
				params
			);

			const response = await createRentalRequest(params, signed.headers);
			onSuccess(response.contract_id);
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to create rental request";
			console.error("Rental request error:", e);
		} finally {
			loading = false;
		}
	}
</script>

{#if offering}
	<!-- Backdrop -->
	<div
		class="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center p-4"
		onclick={(e) => e.target === e.currentTarget && onClose()}
		role="button"
		tabindex="0"
		onkeydown={(e) => e.key === "Escape" && onClose()}
	>
		<!-- Dialog -->
		<div
			class="bg-gradient-to-br from-gray-900 to-gray-800 rounded-2xl max-w-2xl w-full border border-white/20 shadow-2xl max-h-[90vh] overflow-y-auto"
		>
			<!-- Header -->
			<div
				class="flex items-center justify-between p-6 border-b border-white/10"
			>
				<div>
					<h2 class="text-2xl font-bold text-white">
						Rent Resource
					</h2>
					<p class="text-white/60 text-sm mt-1">
						{offering.offer_name}
					</p>
				</div>
				<button
					onclick={onClose}
					class="text-white/60 hover:text-white transition-colors"
					aria-label="Close dialog"
				>
					<svg
						class="w-6 h-6"
						fill="none"
						stroke="currentColor"
						viewBox="0 0 24 24"
					>
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
				<!-- Resource Summary -->
				<div
					class="bg-white/5 rounded-lg p-4 border border-white/10"
				>
					<h3 class="text-sm font-semibold text-white/70 mb-3">
						Resource Details
					</h3>
					<div class="space-y-2 text-sm">
						<div class="flex justify-between">
							<span class="text-white/60">Type</span>
							<span class="text-white font-medium"
								>{offering.product_type}</span
							>
						</div>
						{#if offering.processor_cores}
							<div class="flex justify-between">
								<span class="text-white/60">CPU</span>
								<span class="text-white font-medium"
									>{offering.processor_cores} cores</span
								>
							</div>
						{/if}
						{#if offering.memory_amount}
							<div class="flex justify-between">
								<span class="text-white/60">RAM</span>
								<span class="text-white font-medium"
									>{offering.memory_amount}</span
								>
							</div>
						{/if}
						{#if offering.total_ssd_capacity}
							<div class="flex justify-between">
								<span class="text-white/60">Storage</span>
								<span class="text-white font-medium"
									>{offering.total_ssd_capacity} SSD</span
								>
							</div>
						{/if}
						{#if offering.datacenter_country}
							<div class="flex justify-between">
								<span class="text-white/60">Location</span>
								<span class="text-white font-medium"
									>{offering.datacenter_city}, {offering.datacenter_country}</span
								>
							</div>
						{/if}
					</div>
				</div>

				<!-- Duration -->
				<div>
					<label
						for="duration"
						class="block text-sm font-medium text-white mb-2"
					>
						Rental Duration
					</label>
					<select
						id="duration"
						bind:value={durationHours}
						class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white focus:outline-none focus:border-blue-400 transition-colors"
					>
						<option value={24}>1 Day (24 hours)</option>
						<option value={168}>1 Week (7 days)</option>
						<option value={720}>1 Month (30 days)</option>
						<option value={2160}>3 Months (90 days)</option>
						<option value={4320}>6 Months (180 days)</option>
						<option value={8760}>1 Year (365 days)</option>
					</select>
				</div>

				<!-- SSH Key -->
				<div>
					<label
						for="ssh-key"
						class="block text-sm font-medium text-white mb-2"
					>
						SSH Public Key <span class="text-white/50"
							>(optional)</span
						>
					</label>
					<textarea
						id="ssh-key"
						bind:value={sshKey}
						placeholder="ssh-ed25519 AAAA..."
						rows="3"
						class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:border-blue-400 transition-colors font-mono text-sm"
					></textarea>
					<p class="text-xs text-white/50 mt-1">
						Provide your SSH public key for server access after
						provisioning
					</p>
				</div>

				<!-- Contact Method -->
				<div>
					<label
						for="contact"
						class="block text-sm font-medium text-white mb-2"
					>
						Contact Method <span class="text-white/50"
							>(optional)</span
						>
					</label>
					<input
						id="contact"
						type="text"
						bind:value={contactMethod}
						placeholder="email:you@example.com or matrix:@user:server"
						class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:border-blue-400 transition-colors"
					/>
					<p class="text-xs text-white/50 mt-1">
						How the provider should reach you (e.g.,
						email:you@example.com)
					</p>
				</div>

				<!-- Memo -->
				<div>
					<label
						for="memo"
						class="block text-sm font-medium text-white mb-2"
					>
						Notes <span class="text-white/50">(optional)</span>
					</label>
					<textarea
						id="memo"
						bind:value={memo}
						placeholder="Any special requirements or notes for the provider..."
						rows="3"
						class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:border-blue-400 transition-colors"
					></textarea>
				</div>

				<!-- Price Summary -->
				<div
					class="bg-blue-500/10 rounded-lg p-4 border border-blue-500/30"
				>
					<div class="flex justify-between items-center">
						<div>
							<span class="text-white/70 text-sm"
								>Estimated Cost</span
							>
							<p class="text-xs text-white/50 mt-1">
								{durationHours} hours @ {offering.monthly_price.toFixed(
									2
								)}
								{offering.currency}/mo
							</p>
						</div>
						<div class="text-right">
							<span class="text-2xl font-bold text-white">
								{calculatePrice()}
								<span class="text-lg">{offering.currency}</span>
							</span>
						</div>
					</div>
				</div>

				<!-- Error Message -->
				{#if error}
					<div
						class="bg-red-500/20 border border-red-500/30 rounded-lg p-4 text-red-400"
					>
						<p class="font-semibold">Error</p>
						<p class="text-sm mt-1">{error}</p>
					</div>
				{/if}
			</div>

			<!-- Footer -->
			<div
				class="flex gap-3 p-6 border-t border-white/10 bg-white/5"
			>
				<button
					onclick={onClose}
					disabled={loading}
					class="flex-1 px-4 py-3 bg-white/10 text-white rounded-lg font-semibold hover:bg-white/20 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
				>
					Cancel
				</button>
				<button
					onclick={handleSubmit}
					disabled={loading}
					class="flex-1 px-4 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold hover:brightness-110 hover:scale-105 transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100"
				>
					{#if loading}
						<span class="flex items-center justify-center gap-2">
							<span
								class="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin"
							></span>
							Submitting...
						</span>
					{:else}
						Submit Request
					{/if}
				</button>
			</div>
		</div>
	</div>
{/if}
