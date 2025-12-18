<script lang="ts">
	import type { Offering } from "$lib/services/api";
	import {
		createRentalRequest,
		updateIcpayTransactionId,
		type RentalRequestParams,
	} from "$lib/services/api";
	import { signRequest } from "$lib/services/auth-api";
	import { authStore } from "$lib/stores/auth";
	import type { Ed25519KeyIdentity } from "@dfinity/identity";
	import { loadStripe, type Stripe } from "@stripe/stripe-js";
	import { onMount, onDestroy } from "svelte";
	import { isStripeSupportedCurrency } from "$lib/utils/stripe-currencies";
	import {
		getIcpay,
		getWalletSelect,
		isIcpayConfigured,
	} from "$lib/utils/icpay";

	interface Props {
		offering: Offering | null;
		onClose: () => void;
		onSuccess: (contractId: string) => void;
	}

	let { offering, onClose, onSuccess }: Props = $props();

	let sshKey = $state("");
	let contactMethod = $state("");
	let buyerAddress = $state("");
	let durationHours = $state(720); // Default: 30 days
	let memo = $state("");
	let loading = $state(false);
	let processingPayment = $state(false);
	let error = $state<string | null>(null);
	let paymentMethod = $state<"icpay" | "stripe">("icpay");
	let stripe: Stripe | null = null;
	let walletConnected = $state(false);
	let pendingContractId = $state<string | null>(null);
	let icpayEventUnsubscribe: (() => void) | null = null;

	// Check if Stripe is supported for this offering's currency
	let isStripeAvailable = $derived(
		offering ? isStripeSupportedCurrency(offering.currency) : false,
	);

	onMount(async () => {
		const publishableKey = import.meta.env.VITE_STRIPE_PUBLISHABLE_KEY;
		if (publishableKey) {
			stripe = await loadStripe(publishableKey);
		}

		// Set up ICPay event listeners
		if (isIcpayConfigured()) {
			const icpay = getIcpay();
			if (icpay) {
				icpayEventUnsubscribe = icpay.on(
					"icpay-sdk-transaction-completed",
					handleIcpaySuccess,
				);
			}
		}
	});

	onDestroy(() => {
		if (icpayEventUnsubscribe) {
			icpayEventUnsubscribe();
		}
	});

	function calculatePrice(): string {
		if (!offering) return "0.00";
		const price = (offering.monthly_price * durationHours) / 720;
		return price.toFixed(2);
	}

	async function connectWallet() {
		try {
			const walletSelect = getWalletSelect();
			// For now, we'll try to connect with Internet Identity
			// In the future, this could show a wallet selection dialog
			await walletSelect.connect("ii");
			walletConnected = true;
			error = null;
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to connect wallet";
			console.error("Wallet connection error:", e);
		}
	}

	async function handleIcpaySuccess(detail: any) {
		if (!pendingContractId) {
			console.warn("ICPay payment completed but no pending contract ID");
			return;
		}

		processingPayment = false;
		loading = false;

		// Transaction completed successfully
		console.log("ICPay payment completed:", detail);

		// Record the transaction ID in the backend for audit trail
		const transactionId = detail?.transactionId || detail?.id || detail?.txId;
		if (transactionId) {
			try {
				const signingIdentityInfo = await authStore.getSigningIdentity();
				if (signingIdentityInfo) {
					const signed = await signRequest(
						signingIdentityInfo.identity as Ed25519KeyIdentity,
						"PUT",
						`/api/v1/contracts/${pendingContractId}/icpay-transaction`,
						{ transaction_id: transactionId }
					);
					await updateIcpayTransactionId(pendingContractId, transactionId, signed.headers);
					console.log("ICPay transaction ID recorded:", transactionId);
				}
			} catch (e) {
				// Log but don't fail - payment succeeded, just audit trail update failed
				console.warn("Failed to record ICPay transaction ID:", e);
			}
		}

		onSuccess(pendingContractId);
	}

	async function handleSubmit() {
		if (!offering) return;

		const signingIdentityInfo = await authStore.getSigningIdentity();
		if (!signingIdentityInfo) {
			error = "You must be logged in to rent resources";
			return;
		}

		// SSH key is required for server access
		if (!sshKey.trim()) {
			error = "SSH public key is required for server access after provisioning";
			return;
		}

		if (paymentMethod === "icpay" && !isIcpayConfigured()) {
			error = "ICPay is not configured. Please contact support.";
			return;
		}

		if (paymentMethod === "icpay" && !walletConnected) {
			error = "Please connect your wallet first";
			return;
		}

		loading = true;
		processingPayment = false;
		error = null;

		try {
			const params: RentalRequestParams = {
				offering_db_id: offering.id!,
				ssh_pubkey: sshKey.trim(), // Required
				contact_method: contactMethod || undefined,
				request_memo: memo || undefined,
				duration_hours: durationHours,
				payment_method: paymentMethod,
				buyer_address: buyerAddress || undefined,
			};

			const signed = await signRequest(
				signingIdentityInfo.identity as Ed25519KeyIdentity,
				"POST",
				"/api/v1/contracts",
				params,
			);

			const response = await createRentalRequest(params, signed.headers);

			// If ICPay payment, process crypto payment
			if (paymentMethod === "icpay") {
				const icpay = getIcpay();
				if (!icpay) {
					error = "Failed to initialize ICPay SDK";
					loading = false;
					return;
				}

				processingPayment = true;
				pendingContractId = response.contractId;

				try {
					const usdAmount = parseFloat(calculatePrice());
					// Event listener will handle completion via handleIcpaySuccess
					await icpay.createPaymentUsd({
						usdAmount,
						tokenShortcode: "ic_icp",
						metadata: { contractId: response.contractId },
					});
					// Don't set processingPayment = false here; let the event handler do it
				} catch (icpayError) {
					error =
						icpayError instanceof Error
							? icpayError.message
							: "ICPay payment failed";
					console.error("ICPay payment error:", icpayError);
					loading = false;
					processingPayment = false;
					pendingContractId = null;
					return;
				}
			}

			// If Stripe payment, redirect to Checkout
			if (paymentMethod === "stripe" && response.checkoutUrl) {
				// Redirect to Stripe Checkout
				window.location.href = response.checkoutUrl;
				return;
			}

			// For ICPay, success is handled via event listener
			if (paymentMethod === "icpay") {
				return;
			}

			// Fallback for other payment methods
			onSuccess(response.contractId);
		} catch (e) {
			error =
				e instanceof Error
					? e.message
					: "Failed to create rental request";
			console.error("Rental request error:", e);
		} finally {
			loading = false;
			processingPayment = false;
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
					<h2 class="text-2xl font-bold text-white">Rent Resource</h2>
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
				<!-- Trust Warning -->
				{#if offering.has_critical_flags || (offering.trust_score !== undefined && offering.trust_score < 50)}
					<div
						class="bg-red-500/20 border border-red-500/30 rounded-lg p-4"
					>
						<div class="flex items-start gap-3">
							<span class="text-2xl">&#x26A0;</span>
							<div>
								<h4 class="text-red-400 font-semibold">
									Provider Risk Warning
								</h4>
								<p class="text-red-300/80 text-sm mt-1">
									{#if offering.has_critical_flags}
										This provider has critical reliability
										flags. Review their
										<a
											href="/dashboard/reputation/{offering.pubkey}"
											target="_blank"
											class="underline hover:text-red-200"
										>
											trust metrics
										</a>
										before proceeding.
									{:else if offering.trust_score !== undefined && offering.trust_score < 50}
										This provider has a low trust score ({offering.trust_score}/100).
										Consider reviewing their
										<a
											href="/dashboard/reputation/{offering.pubkey}"
											target="_blank"
											class="underline hover:text-red-200"
										>
											reputation
										</a>
										before renting.
									{/if}
								</p>
							</div>
						</div>
					</div>
				{/if}

				<!-- Resource Summary -->
				<div class="bg-white/5 rounded-lg p-4 border border-white/10">
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

				<!-- Payment Method -->
				<fieldset>
					<legend class="block text-sm font-medium text-white mb-2">
						Payment Method
					</legend>
					<div class="grid grid-cols-2 gap-3">
						<button
							type="button"
							onclick={() => (paymentMethod = "icpay")}
							class="px-4 py-3 rounded-lg font-semibold transition-all border-2 {paymentMethod ===
							'icpay'
								? 'bg-blue-500/20 border-blue-500 text-white'
								: 'bg-white/10 border-white/20 text-white/60 hover:border-white/40'}"
						>
							Crypto (ICPay)
						</button>
						<button
							type="button"
							onclick={() =>
								isStripeAvailable && (paymentMethod = "stripe")}
							disabled={!isStripeAvailable}
							class="px-4 py-3 rounded-lg font-semibold transition-all border-2 {paymentMethod ===
							'stripe'
								? 'bg-blue-500/20 border-blue-500 text-white'
								: isStripeAvailable
									? 'bg-white/10 border-white/20 text-white/60 hover:border-white/40'
									: 'bg-white/5 border-white/10 text-white/30 cursor-not-allowed'}"
							title={!isStripeAvailable
								? `Stripe does not support ${offering?.currency} currency`
								: ""}
						>
							Credit Card
						</button>
					</div>
					{#if !isStripeAvailable}
						<p class="text-xs text-yellow-400/80 mt-2">
							Stripe payment is not available for {offering?.currency}
							currency
						</p>
					{/if}
				</fieldset>

				<!-- ICPay Payment Section -->
				{#if paymentMethod === "icpay"}
					<div
						class="bg-white/5 rounded-lg p-4 border border-white/10"
					>
						<h3 class="text-sm font-semibold text-white/70 mb-2">
							Crypto Payment via ICPay
						</h3>
						<p class="text-sm text-white/60 mb-3">
							Connect your wallet (Internet Identity, Plug, etc.)
							to complete the payment with ICP or other supported
							tokens.
						</p>
						{#if !walletConnected}
							<button
								type="button"
								onclick={connectWallet}
								class="w-full px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-lg font-semibold transition-colors"
							>
								Connect Wallet
							</button>
						{:else}
							<div class="flex items-center gap-2 text-green-400">
								<span class="w-2 h-2 bg-green-400 rounded-full"
								></span>
								<span class="text-sm font-medium"
									>Wallet Connected</span
								>
							</div>
						{/if}
					</div>
				{/if}

				<!-- Stripe Payment Info -->
				{#if paymentMethod === "stripe"}
					<div
						class="bg-white/5 rounded-lg p-4 border border-white/10"
					>
						<h3 class="text-sm font-semibold text-white/70 mb-2">
							Credit Card Payment via Stripe
						</h3>
						<p class="text-sm text-white/60">
							You will be redirected to Stripe's secure checkout
							page to complete your payment. Tax will be
							calculated automatically based on your location.<br
							/>
							While the platform is in test mode, you can use the following
							test cards:<br />
							A card number of 4242 4242 4242 4242 results in a successful
							payment that is immediately processed and does not require
							authentication.<br />
							The card number 4000 0025 0000 3155 requires 3D Secure
							2 authentication for the payment to succeed.<br />
							A payment using the card number 4000 0000 0000 9995 is
							declined with the code insufficient_funds, simulating
							a lack of available funds.<br />
							The card number 4000 0000 0000 0002 is used to simulate
							a declined payment, often resulting in a generic decline.<br
							/>
							A card number of 4000 0000 0000 0069 simulates an expired
							card, leading to a decline.<br />
							The card number 4000 0000 0000 0127 is used to test an
							incorrect CVC input, resulting in a decline.
						</p>
					</div>
				{/if}

				<!-- SSH Key (Required) -->
				<div>
					<label
						for="ssh-key"
						class="block text-sm font-medium text-white mb-2"
					>
						SSH Public Key <span class="text-red-400">*</span>
					</label>
					<textarea
						id="ssh-key"
						bind:value={sshKey}
						placeholder="ssh-ed25519 AAAA..."
						rows="3"
						required
						class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:border-blue-400 transition-colors font-mono text-sm {!sshKey.trim() ? 'border-red-500/50' : ''}"
					></textarea>
					<p class="text-xs text-white/50 mt-1">
						Required for server access after provisioning
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

				<!-- Billing Address (for B2B invoices) -->
				<div>
					<label
						for="buyer-address"
						class="block text-sm font-medium text-white mb-2"
					>
						Billing Address <span class="text-white/50"
							>(optional, for invoices)</span
						>
					</label>
					<textarea
						id="buyer-address"
						bind:value={buyerAddress}
						placeholder="Company Name&#10;Street Address&#10;City, Postal Code&#10;Country"
						rows="3"
						class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:border-blue-400 transition-colors"
					></textarea>
					<p class="text-xs text-white/50 mt-1">
						Required for B2B invoices with VAT
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
									2,
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
			<div class="flex gap-3 p-6 border-t border-white/10 bg-white/5">
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
					{#if processingPayment}
						<span class="flex items-center justify-center gap-2">
							<span
								class="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin"
							></span>
							Processing payment...
						</span>
					{:else if loading}
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
