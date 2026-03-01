<script lang="ts">
	import type { Offering } from "$lib/services/api";
	import {
		createRentalRequest,
		updateIcpayTransactionId,
		type RentalRequestParams,
	} from "$lib/services/api";
	import { signRequest } from "$lib/services/auth-api";
	import { getExternalKeys, UserApiClient } from "$lib/services/user-api";
	import { authStore } from "$lib/stores/auth";
	import { get } from "svelte/store";
	import type { Ed25519KeyIdentity } from "@dfinity/identity";
	import { loadStripe, type Stripe } from "@stripe/stripe-js";
	import { onMount, onDestroy } from "svelte";
	import { isStripeSupportedCurrency } from "$lib/utils/stripe-currencies";
	import {
		getIcpay,
		getWalletSelect,
		isIcpayConfigured,
	} from "$lib/utils/icpay";
	import type { AccountExternalKey } from "$lib/types/generated/AccountExternalKey";

	interface Props {
		offering: Offering | null;
		onClose: () => void;
		onSuccess: (contractId: string) => void;
	}

	let { offering, onClose, onSuccess }: Props = $props();

	let sshKey = $state("");
	let savedSshKeys = $state<AccountExternalKey[]>([]);
	let sshKeygenTab = $state<'unix' | 'powershell' | 'putty'>('unix');
	let copiedSshKeygen = $state<string | null>(null);

	function copySshKeygenCmd(cmd: string, key: string) {
		navigator.clipboard.writeText(cmd);
		copiedSshKeygen = key;
		setTimeout(() => { copiedSshKeygen = null; }, 2000);
	}
	let contactMethod = $state("");
	let buyerAddress = $state("");
	let durationHours = $state(720); // Default: 30 days
	let memo = $state("");
	let loading = $state(false);
	let processingPayment = $state(false);
	let error = $state<string | null>(null);
	let sshKeyError = $state<string | null>(null);
	let saveKeyToProfile = $state(false);

	// OS selection from offering
	let selectedOperatingSystem = $state("");

	// Parse operating systems from offering (comma-separated string)
	let availableOperatingSystems = $derived(
		offering?.operating_systems
			? offering.operating_systems.split(',').map((os) => os.trim()).filter(Boolean)
			: []
	);
	let hasOperatingSystems = $derived(availableOperatingSystems.length > 0);

	// Validate SSH public key format
	function validateSshKey(key: string): string | null {
		if (!key.trim()) {
			return "SSH public key is required for server access";
		}
		// Pattern: ssh-(rsa|ed25519|ecdsa|dss) <base64data> [optional comment]
		const sshKeyPattern = /^ssh-(rsa|ed25519|ecdsa|dss)\s+[A-Za-z0-9+/]+={0,3}(\s+.*)?$/;
		if (!sshKeyPattern.test(key.trim())) {
			return "Invalid SSH key format. Expected: ssh-ed25519 AAAA... or ssh-rsa AAAA...";
		}
		return null;
	}

	// Reactive validation
	let sshKeyValidation = $derived(validateSshKey(sshKey));
	let isCustomKey = $derived(
		sshKey.trim().length > 0 &&
		!savedSshKeys.some((k) => k.keyData === sshKey.trim())
	);

	// Check if Stripe is supported for this offering's currency (used for default)
	let isStripeAvailable = $derived(
		offering ? isStripeSupportedCurrency(offering.currency) : false,
	);

	// Default to Stripe for fiat currencies (USD, EUR, etc.) when available,
	// otherwise default to ICPay for crypto-only offerings
	let paymentMethod = $state<"icpay" | "stripe">(
		offering && isStripeSupportedCurrency(offering.currency) ? "stripe" : "icpay"
	);

	// Payment is required when a payment method is selected (icpay or stripe)
	let paymentRequired = $derived(paymentMethod === "icpay" || paymentMethod === "stripe");

	// Subscription offering helpers
	let isSubscriptionOffering = $derived(offering?.is_subscription ?? false);
	let subscriptionIntervalLabel = $derived(() => {
		const days = offering?.subscription_interval_days;
		if (!days) return "Recurring";
		if (days <= 31) return "Monthly";
		if (days <= 93) return "Quarterly";
		if (days <= 366) return "Yearly";
		return `Every ${days} days`;
	});
	let stripe: Stripe | null = null;
	let walletConnected = $state(false);
	let pendingContractId = $state<string | null>(null);
	let icpayEventUnsubscribe: (() => void) | null = null;

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

		// Fetch user's saved SSH keys from profile
		const unsubscribe = authStore.activeIdentity.subscribe(async (identity) => {
			if (identity?.account?.username) {
				try {
					const keys = await getExternalKeys(identity.account.username);
					// Filter to SSH keys only (ssh-ed25519, ssh-rsa, etc.)
					savedSshKeys = keys.filter((k) => k.keyType.startsWith("ssh-"));
					// Pre-populate with first SSH key if user hasn't entered one
					if (savedSshKeys.length > 0 && !sshKey.trim()) {
						sshKey = savedSshKeys[0].keyData;
					}
				} catch (e) {
					console.warn("Failed to fetch saved SSH keys:", e);
				}
			}
		});
		// Unsubscribe after first call (we only need the initial value)
		unsubscribe();
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
				}
			} catch (e) {
				// Log but don't fail - payment succeeded, just audit trail update failed
				console.warn("Failed to record ICPay transaction ID:", e);
			}
		}

		await maybeSaveKeyToProfile();
		onSuccess(pendingContractId);
	}

	async function maybeSaveKeyToProfile() {
		if (!saveKeyToProfile || !isCustomKey || sshKeyValidation) return;
		const activeIdentity = get(authStore.activeIdentity);
		if (!activeIdentity?.account?.username || !activeIdentity.identity) return;
		try {
			const client = new UserApiClient(activeIdentity.identity);
			const keyType = sshKey.trim().split(" ")[0];
			await client.addExternalKey(activeIdentity.account.username, {
				keyType,
				keyData: sshKey.trim(),
			});
		} catch (e) {
			console.warn("Failed to save SSH key to profile:", e);
		}
	}

	async function handleSubmit() {
		if (!offering) return;

		const signingIdentityInfo = await authStore.getSigningIdentity();
		if (!signingIdentityInfo) {
			error = "You must be logged in to rent resources";
			return;
		}

		// SSH key is required for server access - validate format
		const sshValidationError = validateSshKey(sshKey);
		if (sshValidationError) {
			error = sshValidationError;
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
				operating_system: selectedOperatingSystem || undefined,
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
				// Persist SSH key save intent in localStorage so the rentals page
				// can complete the save after returning from Stripe Checkout.
				if (saveKeyToProfile && isCustomKey && !sshKeyValidation && sshKey.trim()) {
					localStorage.setItem('dc_pending_ssh_save', sshKey.trim());
				}
				window.location.href = response.checkoutUrl;
				return;
			}

			// For ICPay, success is handled via event listener
			if (paymentMethod === "icpay") {
				return;
			}

			// Fallback for other payment methods
			await maybeSaveKeyToProfile();
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
		class="fixed inset-0 bg-base/80 backdrop-blur-sm z-50 flex items-center justify-center p-4"
		onclick={(e) => e.target === e.currentTarget && onClose()}
		role="button"
		tabindex="0"
		onkeydown={(e) => e.key === "Escape" && onClose()}
	>
		<!-- Dialog -->
		<div
			class="bg-gradient-to-br from-base to-gray-800  max-w-2xl w-full border border-neutral-800 shadow-2xl max-h-[90vh] overflow-y-auto"
		>
			<!-- Header -->
			<div
				class="flex items-center justify-between p-6 border-b border-neutral-800"
			>
				<div>
					<h2 class="text-2xl font-bold text-white">Rent Resource</h2>
					<p class="text-neutral-500 text-sm mt-1">
						{offering.offer_name}
					</p>
				</div>
				<button
					onclick={onClose}
					class="text-neutral-500 hover:text-white transition-colors"
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
						class="bg-red-500/20 border border-red-500/30  p-4"
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
				<div class="bg-surface-elevated  p-4 border border-neutral-800">
					<h3 class="text-sm font-semibold text-neutral-400 mb-3">
						Resource Details
					</h3>
					<div class="space-y-2 text-sm">
						<div class="flex justify-between">
							<span class="text-neutral-500">Type</span>
							<span class="text-white font-medium"
								>{offering.product_type}</span
							>
						</div>
						{#if offering.processor_cores}
							<div class="flex justify-between">
								<span class="text-neutral-500">CPU</span>
								<span class="text-white font-medium"
									>{offering.processor_cores} cores</span
								>
							</div>
						{/if}
						{#if offering.memory_amount}
							<div class="flex justify-between">
								<span class="text-neutral-500">RAM</span>
								<span class="text-white font-medium"
									>{offering.memory_amount}</span
								>
							</div>
						{/if}
						{#if offering.total_ssd_capacity}
							<div class="flex justify-between">
								<span class="text-neutral-500">Storage</span>
								<span class="text-white font-medium"
									>{offering.total_ssd_capacity} SSD</span
								>
							</div>
						{/if}
						{#if offering.datacenter_country}
							<div class="flex justify-between">
								<span class="text-neutral-500">Location</span>
								<span class="text-white font-medium"
									>{offering.datacenter_city}, {offering.datacenter_country}</span
								>
							</div>
						{/if}
					</div>
				</div>

				<!-- Price Summary -->
				<div
					class="bg-primary-500/10  p-4 border border-primary-500/30"
				>
					<div class="flex justify-between items-center mb-3">
						<div>
							{#if isSubscriptionOffering}
								<span class="text-neutral-400 text-sm">{subscriptionIntervalLabel()} Subscription</span>
								<p class="text-xs text-neutral-500 mt-1">
									Billed {subscriptionIntervalLabel().toLowerCase()}
								</p>
							{:else}
								<span class="text-neutral-400 text-sm">One-Time Payment</span>
								<p class="text-xs text-neutral-500 mt-1">
									{durationHours} hours @ {offering.monthly_price.toFixed(2)} {offering.currency}/mo
								</p>
							{/if}
						</div>
						<div class="text-right">
							<span class="text-2xl font-bold text-white">
								{#if isSubscriptionOffering}
									{offering.monthly_price.toFixed(2)}
								{:else}
									{calculatePrice()}
								{/if}
								<span class="text-lg">{offering.currency}</span>
							</span>
							{#if isSubscriptionOffering}
								<p class="text-xs text-neutral-500">/{subscriptionIntervalLabel().toLowerCase().replace('ly', '')}</p>
							{/if}
						</div>
					</div>
					<div class="text-xs text-neutral-500 border-t border-neutral-800 pt-2 space-y-1">
						{#if isSubscriptionOffering}
							<p>Recurring subscription. Cancel anytime from your rentals dashboard.</p>
						{:else}
							<p>No recurring charges. You can cancel anytime for a prorated refund.</p>
						{/if}
					</div>
				</div>

				<!-- Duration (hidden for subscription offerings) -->
				{#if !isSubscriptionOffering}
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
							class="w-full px-4 py-3 bg-surface-elevated border border-neutral-800  text-white focus:outline-none focus:border-primary-400 transition-colors"
						>
							<option value={24}>1 Day (24 hours)</option>
							<option value={168}>1 Week (7 days)</option>
							<option value={720}>1 Month (30 days)</option>
							<option value={2160}>3 Months (90 days)</option>
							<option value={4320}>6 Months (180 days)</option>
							<option value={8760}>1 Year (365 days)</option>
						</select>
						<p class="text-xs text-neutral-500 mt-1">
							Ends: {new Date(Date.now() + durationHours * 60 * 60 * 1000).toLocaleString()}
						</p>
					</div>
				{/if}

				<!-- Operating System Selection -->
				{#if hasOperatingSystems}
					<div>
						<label
							for="operating-system"
							class="block text-sm font-medium text-white mb-2"
						>
							Operating System
						</label>
						<select
							id="operating-system"
							bind:value={selectedOperatingSystem}
							class="w-full px-4 py-3 bg-surface-elevated border border-neutral-800  text-white focus:outline-none focus:border-primary-400 transition-colors"
						>
							<option value="">Select an OS...</option>
							{#each availableOperatingSystems as os}
								<option value={os}>{os}</option>
							{/each}
						</select>
						<p class="text-xs text-neutral-500 mt-1">
							Choose the operating system for your server
						</p>
					</div>
				{/if}

				<!-- Payment Method -->
				<fieldset>
					<legend class="block text-sm font-medium text-white mb-2">
						Payment Method
					</legend>
					<div class="grid grid-cols-2 gap-3">
						<button
							type="button"
							onclick={() => (paymentMethod = "icpay")}
							class="px-4 py-3  font-semibold transition-all border-2 {paymentMethod ===
							'icpay'
								? 'bg-primary-500/20 border-primary-500 text-white'
								: 'bg-surface-elevated border-neutral-800 text-neutral-500 hover:border-white/40'}"
						>
							Crypto (ICPay)
						</button>
						<button
							type="button"
							onclick={() =>
								isStripeAvailable && (paymentMethod = "stripe")}
							disabled={!isStripeAvailable}
							class="px-4 py-3  font-semibold transition-all border-2 {paymentMethod ===
							'stripe'
								? 'bg-primary-500/20 border-primary-500 text-white'
								: isStripeAvailable
									? 'bg-surface-elevated border-neutral-800 text-neutral-500 hover:border-white/40'
									: 'bg-surface-elevated border-neutral-800 text-neutral-700 cursor-not-allowed'}"
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
						class="bg-surface-elevated  p-4 border border-neutral-800"
					>
						<h3 class="text-sm font-semibold text-neutral-400 mb-2">
							Crypto Payment via ICPay
						</h3>
						<p class="text-sm text-neutral-500 mb-3">
							Connect your wallet (Internet Identity, Plug, etc.)
							to complete the payment with ICP or other supported
							tokens.
						</p>
						{#if !walletConnected}
							<button
								type="button"
								onclick={connectWallet}
								class="w-full px-4 py-2 bg-primary-500 hover:bg-primary-600 text-white  font-semibold transition-colors"
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
						class="bg-surface-elevated  p-4 border border-neutral-800"
					>
						<h3 class="text-sm font-semibold text-neutral-400 mb-2">
							Credit Card Payment via Stripe
						</h3>
						<p class="text-sm text-neutral-500">
							You will be redirected to Stripe's secure checkout
							page to complete your payment. Tax will be
							calculated automatically based on your location.
						</p>
						{#if import.meta.env.DEV}
							<div class="mt-3 p-3 bg-yellow-500/10 border border-yellow-500/30 text-xs text-yellow-300 space-y-1">
								<p class="font-semibold">Test mode — sample card numbers:</p>
								<p>4242 4242 4242 4242 — succeeds immediately, no authentication</p>
								<p>4000 0025 0000 3155 — requires 3D Secure 2 authentication</p>
								<p>4000 0000 0000 9995 — declined: insufficient_funds</p>
								<p>4000 0000 0000 0002 — declined: generic</p>
								<p>4000 0000 0000 0069 — declined: expired card</p>
								<p>4000 0000 0000 0127 — declined: incorrect CVC</p>
							</div>
						{/if}
					</div>
				{/if}

				<!-- Contact Method -->
				<div>
					<label
						for="contact"
						class="block text-sm font-medium text-white mb-2"
					>
						Contact Method <span class="text-neutral-500"
							>(optional)</span
						>
					</label>
					<input
						id="contact"
						type="text"
						bind:value={contactMethod}
						placeholder="email:you@example.com or matrix:@user:server"
						class="w-full px-4 py-3 bg-surface-elevated border border-neutral-800  text-white placeholder-white/50 focus:outline-none focus:border-primary-400 transition-colors"
					/>
					<p class="text-xs text-neutral-500 mt-1">
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
						Billing Address <span class="text-neutral-500"
							>(optional, for invoices)</span
						>
					</label>
					<textarea
						id="buyer-address"
						bind:value={buyerAddress}
						placeholder="Company Name&#10;Street Address&#10;City, Postal Code&#10;Country"
						rows="3"
						class="w-full px-4 py-3 bg-surface-elevated border border-neutral-800  text-white placeholder-white/50 focus:outline-none focus:border-primary-400 transition-colors"
					></textarea>
					<p class="text-xs text-neutral-500 mt-1">
						Required for B2B invoices with VAT
					</p>
				</div>

				<!-- Memo -->
				<div>
					<label
						for="memo"
						class="block text-sm font-medium text-white mb-2"
					>
						Notes <span class="text-neutral-500">(optional)</span>
					</label>
					<textarea
						id="memo"
						bind:value={memo}
						placeholder="Any special requirements or notes for the provider..."
						rows="3"
						class="w-full px-4 py-3 bg-surface-elevated border border-neutral-800  text-white placeholder-white/50 focus:outline-none focus:border-primary-400 transition-colors"
					></textarea>
				</div>

				<!-- SSH Key (Required) -->
				<div>
					<label
						for="ssh-key"
						class="block text-sm font-medium text-white mb-2"
					>
						SSH Public Key <span class="text-red-400">*</span>
					</label>
					{#if savedSshKeys.length > 0}
						<select
							id="ssh-key-select"
							onchange={(e) => {
								const value = (e.target as HTMLSelectElement).value;
								if (value !== "__custom__") {
									sshKey = value;
								}
							}}
							class="w-full px-4 py-3 bg-surface-elevated border border-neutral-800  text-white focus:outline-none focus:border-primary-400 transition-colors mb-2"
						>
							{#each savedSshKeys as key}
								<option value={key.keyData} selected={sshKey === key.keyData}>
									{key.label || key.keyType} ({key.keyData.slice(0, 30)}...)
								</option>
							{/each}
							<option value="__custom__">Enter a different key...</option>
						</select>
					{/if}
					<textarea
						id="ssh-key"
						bind:value={sshKey}
						placeholder="ssh-ed25519 AAAA..."
						rows="3"
						required
						class="w-full px-4 py-3 bg-surface-elevated border  text-white placeholder-white/50 focus:outline-none transition-colors font-mono text-sm {sshKeyValidation ? 'border-red-500/50' : sshKey.trim() ? 'border-green-500/50' : 'border-neutral-800'} {!sshKeyValidation && sshKey.trim() ? 'focus:border-green-400' : 'focus:border-primary-400'}"
					></textarea>
					{#if sshKeyValidation && sshKey.trim()}
						<p class="text-xs text-red-400 mt-1">
							{sshKeyValidation}
						</p>
					{:else if !sshKeyValidation && sshKey.trim()}
						<p class="text-xs text-green-400 mt-1">
							Valid SSH key format
						</p>
					{:else}
						<p class="text-xs text-neutral-500 mt-1">
							Required for server access after provisioning
						</p>
					{/if}
					<details class="text-sm text-neutral-400 mt-1">
						<summary class="cursor-pointer hover:text-neutral-200 select-none">How to generate an SSH key?</summary>
						<div class="mt-2 p-3 bg-neutral-800 rounded text-xs">
							<div class="flex gap-1 mb-3">
								{#each ([['unix', 'macOS / Linux'], ['powershell', 'Windows (PowerShell)'], ['putty', 'Windows (PuTTY)']] as const) as [id, label]}
									<button
										type="button"
										onclick={() => sshKeygenTab = id}
										class="text-xs px-2 py-1 border transition-colors {sshKeygenTab === id ? 'bg-green-500/20 border-green-500/50 text-green-300' : 'bg-surface-elevated border-neutral-700 text-neutral-400 hover:text-white'}"
									>{label}</button>
								{/each}
							</div>
							{#if sshKeygenTab === 'unix'}
								<ol class="text-neutral-300 space-y-2 list-decimal list-inside">
									<li>Open Terminal</li>
									<li>Generate key:
										<div class="flex items-center justify-between mt-1 font-mono bg-black/30 px-3 py-2 rounded">
											<code class="text-green-300 select-all">ssh-keygen -t ed25519 -C "your-email@example.com"</code>
											<button type="button" onclick={() => copySshKeygenCmd('ssh-keygen -t ed25519 -C "your-email@example.com"', 'unix-gen')} class="ml-2 shrink-0 px-2 py-0.5 bg-surface-elevated text-neutral-400 border border-neutral-700 hover:text-white transition-colors">{copiedSshKeygen === 'unix-gen' ? 'Copied!' : 'Copy'}</button>
										</div>
									</li>
									<li>Press Enter when prompted to accept defaults</li>
									<li>Copy public key:
										<div class="flex items-center justify-between mt-1 font-mono bg-black/30 px-3 py-2 rounded">
											<code class="text-green-300 select-all">cat ~/.ssh/id_ed25519.pub</code>
											<button type="button" onclick={() => copySshKeygenCmd('cat ~/.ssh/id_ed25519.pub', 'unix-cat')} class="ml-2 shrink-0 px-2 py-0.5 bg-surface-elevated text-neutral-400 border border-neutral-700 hover:text-white transition-colors">{copiedSshKeygen === 'unix-cat' ? 'Copied!' : 'Copy'}</button>
										</div>
									</li>
									<li>Paste the output into the SSH Public Key field above</li>
								</ol>
							{:else if sshKeygenTab === 'powershell'}
								<ol class="text-neutral-300 space-y-2 list-decimal list-inside">
									<li>Open Windows Terminal or PowerShell</li>
									<li>Generate key:
										<div class="flex items-center justify-between mt-1 font-mono bg-black/30 px-3 py-2 rounded">
											<code class="text-green-300 select-all">ssh-keygen -t ed25519 -C "your-email@example.com"</code>
											<button type="button" onclick={() => copySshKeygenCmd('ssh-keygen -t ed25519 -C "your-email@example.com"', 'ps-gen')} class="ml-2 shrink-0 px-2 py-0.5 bg-surface-elevated text-neutral-400 border border-neutral-700 hover:text-white transition-colors">{copiedSshKeygen === 'ps-gen' ? 'Copied!' : 'Copy'}</button>
										</div>
									</li>
									<li>Press Enter when prompted to accept defaults</li>
									<li>Copy public key:
										<div class="flex items-center justify-between mt-1 font-mono bg-black/30 px-3 py-2 rounded">
											<code class="text-green-300 select-all">Get-Content "$env:USERPROFILE\.ssh\id_ed25519.pub"</code>
											<button type="button" onclick={() => copySshKeygenCmd('Get-Content "$env:USERPROFILE\\.ssh\\id_ed25519.pub"', 'ps-cat')} class="ml-2 shrink-0 px-2 py-0.5 bg-surface-elevated text-neutral-400 border border-neutral-700 hover:text-white transition-colors">{copiedSshKeygen === 'ps-cat' ? 'Copied!' : 'Copy'}</button>
										</div>
									</li>
									<li>Paste the output into the SSH Public Key field above</li>
								</ol>
							{:else}
								<ol class="text-neutral-300 space-y-2 list-decimal list-inside">
									<li>Download PuTTYgen from <a href="https://putty.org" target="_blank" rel="noopener" class="text-green-400 hover:underline">putty.org</a></li>
									<li>Open PuTTYgen → Select <strong>EdDSA</strong> → Click <strong>Generate</strong></li>
									<li>Move mouse over the blank area to generate randomness</li>
									<li>Click <strong>Save public key</strong> and <strong>Save private key</strong></li>
									<li>Copy the text from the "Public key for pasting" box at the top</li>
									<li>Paste it into the SSH Public Key field above</li>
								</ol>
							{/if}
						</div>
					</details>
					{#if isCustomKey && !sshKeyValidation && sshKey.trim()}
						<label class="flex items-center gap-2 text-sm text-neutral-400 mt-2 cursor-pointer">
							<input type="checkbox" bind:checked={saveKeyToProfile} class="w-4 h-4 accent-primary-500" />
							Save this key to my profile for future rentals
						</label>
					{/if}
				</div>

				<!-- Error Message -->
				{#if error}
					<div
						class="bg-red-500/20 border border-red-500/30  p-4 text-red-400"
					>
						<p class="font-semibold">Error</p>
						<p class="text-sm mt-1">{error}</p>
					</div>
				{/if}
			</div>

			<!-- Footer -->
			<div class="flex gap-3 p-6 border-t border-neutral-800 bg-surface-elevated">
				<button
					onclick={onClose}
					disabled={loading}
					class="flex-1 px-4 py-3 bg-surface-elevated text-white  font-semibold hover:bg-surface-elevated transition-all disabled:opacity-50 disabled:cursor-not-allowed"
				>
					Cancel
				</button>
				<button
					onclick={handleSubmit}
					disabled={loading}
					class="flex-1 px-4 py-3 bg-gradient-to-r from-primary-500 to-primary-600  font-semibold hover:brightness-110 hover:scale-105 transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100"
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
						{paymentRequired ? "Pay now" : "Submit Request"}
					{/if}
				</button>
			</div>
		</div>
	</div>
{/if}
