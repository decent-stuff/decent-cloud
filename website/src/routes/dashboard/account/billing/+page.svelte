<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { page } from "$app/stores";
	import { authStore } from "$lib/stores/auth";
	import { navigateToLogin } from "$lib/utils/navigation";
	import { signRequest } from "$lib/services/auth-api";
	import { getBillingSettings, updateBillingSettings, validateVatId, type BillingSettings } from "$lib/services/api";
	import type { IdentityInfo } from "$lib/stores/auth";

	let currentIdentity = $state<IdentityInfo | null>(null);
	let isAuthenticated = $state(false);
	let unsubscribe: (() => void) | null = null;
	let unsubscribeAuth: (() => void) | null = null;

	// Form state
	let billingAddress = $state("");
	let billingVatId = $state("");
	let billingCountryCode = $state("");
	let loading = $state(false);
	let saving = $state(false);
	let validating = $state(false);
	let error = $state("");
	let success = $state("");
	let vatValidationResult = $state<{ valid: boolean; name?: string; error?: string } | null>(null);

	// EU country codes for VAT
	const euCountries = [
		{ code: "AT", name: "Austria" },
		{ code: "BE", name: "Belgium" },
		{ code: "BG", name: "Bulgaria" },
		{ code: "HR", name: "Croatia" },
		{ code: "CY", name: "Cyprus" },
		{ code: "CZ", name: "Czech Republic" },
		{ code: "DK", name: "Denmark" },
		{ code: "EE", name: "Estonia" },
		{ code: "FI", name: "Finland" },
		{ code: "FR", name: "France" },
		{ code: "DE", name: "Germany" },
		{ code: "GR", name: "Greece" },
		{ code: "HU", name: "Hungary" },
		{ code: "IE", name: "Ireland" },
		{ code: "IT", name: "Italy" },
		{ code: "LV", name: "Latvia" },
		{ code: "LT", name: "Lithuania" },
		{ code: "LU", name: "Luxembourg" },
		{ code: "MT", name: "Malta" },
		{ code: "NL", name: "Netherlands" },
		{ code: "PL", name: "Poland" },
		{ code: "PT", name: "Portugal" },
		{ code: "RO", name: "Romania" },
		{ code: "SK", name: "Slovakia" },
		{ code: "SI", name: "Slovenia" },
		{ code: "ES", name: "Spain" },
		{ code: "SE", name: "Sweden" },
	];

	onMount(() => {
		unsubscribeAuth = authStore.isAuthenticated.subscribe((isAuth) => {
			isAuthenticated = isAuth;
		});

		unsubscribe = authStore.activeIdentity.subscribe(async (value) => {
			currentIdentity = value;
			if (value?.identity) {
				await loadBillingSettings();
			}
		});
	});

	onDestroy(() => {
		unsubscribe?.();
		unsubscribeAuth?.();
	});

	async function loadBillingSettings() {
		if (!currentIdentity?.identity) return;
		loading = true;
		error = "";
		try {
			const { headers } = await signRequest(currentIdentity.identity, "GET", "/api/v1/accounts/billing");
			const settings = await getBillingSettings(headers);
			billingAddress = settings.billingAddress || "";
			billingVatId = settings.billingVatId || "";
			billingCountryCode = settings.billingCountryCode || "";
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load billing settings";
		} finally {
			loading = false;
		}
	}

	async function handleSave() {
		if (!currentIdentity?.identity) return;
		saving = true;
		error = "";
		success = "";
		try {
			const settings: BillingSettings = {
				billingAddress: billingAddress || undefined,
				billingVatId: billingVatId || undefined,
				billingCountryCode: billingCountryCode || undefined,
			};
			const { headers } = await signRequest(
				currentIdentity.identity,
				"PUT",
				"/api/v1/accounts/billing",
				settings
			);
			await updateBillingSettings(settings, headers);
			success = "Billing settings saved";
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to save billing settings";
		} finally {
			saving = false;
		}
	}

	async function handleValidateVat() {
		if (!billingCountryCode || !billingVatId) {
			vatValidationResult = { valid: false, error: "Country and VAT ID required" };
			return;
		}
		validating = true;
		vatValidationResult = null;
		try {
			const result = await validateVatId(billingCountryCode, billingVatId);
			vatValidationResult = result;
		} catch (e) {
			vatValidationResult = { valid: false, error: e instanceof Error ? e.message : "Validation failed" };
		} finally {
			validating = false;
		}
	}

	function handleLogin() {
		navigateToLogin($page.url.pathname);
	}
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">Billing Settings</h1>
		<p class="text-white/60">Manage your billing address and VAT information for invoices</p>
	</div>

	{#if !isAuthenticated}
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-8 border border-white/20 text-center">
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">💳</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-white/70">
					Create an account or login to manage your billing information.
				</p>
				<button
					onclick={handleLogin}
					class="px-8 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 hover:scale-105 transition-all"
				>
					Login / Create Account
				</button>
			</div>
		</div>
	{:else if loading}
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-8 border border-white/20 text-center">
			<p class="text-white/60">Loading billing settings...</p>
		</div>
	{:else}
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
			<h2 class="text-xl font-semibold text-white mb-6">Invoice Information</h2>
			<p class="text-white/60 text-sm mb-6">
				This information will be used on your invoices. For businesses, provide your company details and VAT ID to receive proper B2B invoices.
			</p>

			{#if error}
				<div class="mb-4 p-3 bg-red-500/20 border border-red-500/50 rounded-lg text-red-200 text-sm">
					{error}
				</div>
			{/if}

			{#if success}
				<div class="mb-4 p-3 bg-green-500/20 border border-green-500/50 rounded-lg text-green-200 text-sm">
					{success}
				</div>
			{/if}

			<div class="space-y-4">
				<div>
					<label for="billingAddress" class="block text-white/70 text-sm mb-1">
						Billing Address
					</label>
					<textarea
						id="billingAddress"
						bind:value={billingAddress}
						rows="3"
						placeholder="Company Name&#10;Street Address&#10;City, Postal Code&#10;Country"
						class="w-full px-4 py-2 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/40 focus:outline-none focus:border-blue-500"
					></textarea>
					<p class="text-white/40 text-xs mt-1">
						Include company name (if applicable), street, city, postal code, and country
					</p>
				</div>

				<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
					<div class="min-w-0">
						<label for="billingCountryCode" class="block text-white/70 text-sm mb-1">
							Country (for VAT)
						</label>
						<select
							id="billingCountryCode"
							bind:value={billingCountryCode}
							class="w-full px-4 py-2 bg-white/10 border border-white/20 rounded-lg text-white focus:outline-none focus:border-blue-500"
						>
							<option value="">Select country...</option>
							{#each euCountries as country}
								<option value={country.code}>{country.code} - {country.name}</option>
							{/each}
							<option value="OTHER">Other (non-EU)</option>
						</select>
					</div>

					<div class="min-w-0">
						<label for="billingVatId" class="block text-white/70 text-sm mb-1">
							VAT ID (optional)
						</label>
						<div class="flex gap-2">
							<input
								id="billingVatId"
								type="text"
								bind:value={billingVatId}
								placeholder="123456789"
								class="flex-1 px-4 py-2 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/40 focus:outline-none focus:border-blue-500"
							/>
							<button
								onclick={handleValidateVat}
								disabled={validating || !billingCountryCode || !billingVatId}
								class="px-4 py-2 bg-white/10 border border-white/20 rounded-lg text-white hover:bg-white/20 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
							>
								{validating ? "..." : "Verify"}
							</button>
						</div>
						<p class="text-white/40 text-xs mt-1">
							EU businesses: Enter VAT number without country prefix
						</p>
					</div>
				</div>

				{#if vatValidationResult}
					<div class="p-3 rounded-lg {vatValidationResult.valid ? 'bg-green-500/20 border border-green-500/50' : 'bg-yellow-500/20 border border-yellow-500/50'}">
						{#if vatValidationResult.valid}
							<p class="text-green-200 text-sm font-medium">VAT ID is valid</p>
							{#if vatValidationResult.name}
								<p class="text-green-200/70 text-xs mt-1">{vatValidationResult.name}</p>
							{/if}
						{:else}
							<p class="text-yellow-200 text-sm">{vatValidationResult.error || "VAT ID could not be validated"}</p>
						{/if}
					</div>
				{/if}

				<div class="pt-4 border-t border-white/10">
					<button
						onclick={handleSave}
						disabled={saving}
						class="px-6 py-2 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 disabled:opacity-50 disabled:cursor-not-allowed transition-all"
					>
						{saving ? "Saving..." : "Save Billing Settings"}
					</button>
				</div>
			</div>
		</div>

		<div class="bg-white/5 backdrop-blur-lg rounded-xl p-6 border border-white/10">
			<h3 class="text-lg font-semibold text-white/80 mb-3">About VAT and Invoices</h3>
			<ul class="text-white/60 text-sm space-y-2">
				<li>Your billing information will appear on all invoices for your purchases.</li>
				<li>EU businesses with valid VAT IDs may qualify for reverse charge (0% VAT).</li>
				<li>VAT validation uses the official EU VIES database.</li>
				<li>Changes apply to future invoices only.</li>
			</ul>
		</div>
	{/if}
</div>
