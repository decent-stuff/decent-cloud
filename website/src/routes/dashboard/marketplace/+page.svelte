<script lang="ts">
	import { onMount, tick } from "svelte";
	import { page } from "$app/stores";
	import { searchOfferings, fetchIcpPrice, type Offering } from "$lib/services/api";
	import RentalRequestDialog from "$lib/components/RentalRequestDialog.svelte";
	import AuthPromptModal from "$lib/components/AuthPromptModal.svelte";
	import TrustBadge from "$lib/components/TrustBadge.svelte";
	import Icon, { type IconName } from "$lib/components/Icons.svelte";
	import { authStore } from "$lib/stores/auth";
	import { truncatePubkey } from "$lib/utils/identity";
	import { addToComparison, removeFromComparison, COMPARE_MAX_ERROR } from "$lib/utils/compare";

	let offerings = $state<Offering[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let icpPriceUsd = $state<number | null>(null);
	let searchQuery = $state("");
	let selectedOffering = $state<Offering | null>(null);
	let successMessage = $state<string | null>(null);
	let isAuthenticated = $state(false);
	const PROVIDER_CTA_KEY = 'dc-provider-cta-dismissed';
	let providerCtaDismissed = $state(false);
	let showAuthModal = $state(false);
	let expandedRow = $state<number | null>(null);
	let sortDir = $state<"asc" | "desc">("asc");
	let sortField = $state<"price" | "trust">("price");
	let showFilters = $state(false);
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;

	// Filters
	let selectedTypes = $state<Set<string>>(new Set());
	let minPrice = $state<number | null>(null);
	let maxPrice = $state<number | null>(null);
	let selectedRegion = $state<string>("");
	let selectedCountry = $state<string>("");
	let selectedCity = $state<string>("");
	let minCores = $state<number | null>(null);
	let minMemoryGb = $state<number | null>(null);
	let minSsdGb = $state<number | null>(null);
	let selectedVirt = $state<string>("");
	let unmeteredOnly = $state(false);
	let minTrust = $state<number | null>(null);
	let showDemoOfferings = $state(false);
	let showOfflineOfferings = $state(false);
	let recipesOnly = $state(false);

	// Region definitions (matching dc-agent geolocation.rs)
	const REGIONS = [
		{ code: "europe", name: "Europe" },
		{ code: "na", name: "North America" },
		{ code: "latam", name: "Latin America" },
		{ code: "apac", name: "Asia Pacific" },
		{ code: "mena", name: "Middle East & North Africa" },
		{ code: "ssa", name: "Sub-Saharan Africa" },
		{ code: "cis", name: "CIS (Russia & neighbors)" },
	] as const;

	// Country to region mapping (subset of most common countries)
	const COUNTRY_TO_REGION: Record<string, string> = {
		// Europe
		DE: "europe", FR: "europe", GB: "europe", UK: "europe", NL: "europe", PL: "europe",
		IT: "europe", ES: "europe", SE: "europe", NO: "europe", FI: "europe", DK: "europe",
		AT: "europe", BE: "europe", CH: "europe", IE: "europe", PT: "europe", GR: "europe",
		CZ: "europe", HU: "europe", RO: "europe", BG: "europe", HR: "europe", SI: "europe",
		// North America
		US: "na", CA: "na", MX: "na",
		// Latin America
		BR: "latam", AR: "latam", CL: "latam", CO: "latam", PE: "latam", VE: "latam",
		// Asia Pacific
		CN: "apac", JP: "apac", SG: "apac", AU: "apac", NZ: "apac", IN: "apac",
		KR: "apac", TH: "apac", MY: "apac", PH: "apac", ID: "apac", VN: "apac",
		// MENA
		AE: "mena", SA: "mena", IL: "mena", TR: "mena", EG: "mena",
		// CIS
		RU: "cis", UA: "cis", BY: "cis", KZ: "cis",
	};

	// Derived: unique virtualization types from offerings
	let virtTypes = $derived(
		[
			...new Set(
				offerings
					.map((o) => o.virtualization_type)
					.filter((v): v is string => !!v),
			),
		].sort(),
	);

	// Keep a stable list of all countries (doesn't change with filtering)
	let allCountries = $state<string[]>([]);

	// Derived: countries for current region filter
	let countries = $derived(() => {
		if (!selectedRegion) return allCountries;
		// Filter countries by selected region
		return allCountries.filter(c => COUNTRY_TO_REGION[c] === selectedRegion);
	});

	let cities = $derived(() => {
		const filtered = selectedCountry
			? offerings.filter((o) => o.datacenter_country === selectedCountry)
			: offerings;
		return [
			...new Set(filtered.map((o) => o.datacenter_city).filter(Boolean)),
		].sort();
	});

	// Derived: filtered and sorted offerings
	let filteredOfferings = $derived.by(() => {
		let result = [...offerings];

		// Client-side type filter (multi-select)
		if (selectedTypes.size > 0) {
			result = result.filter((o) => {
				const type = o.product_type.toLowerCase();
				for (const t of selectedTypes) {
					if (type.includes(t)) return true;
				}
				return false;
			});
		}

		// Client-side region filter
		if (selectedRegion) {
			result = result.filter((o) => {
				const country = o.datacenter_country;
				return country && COUNTRY_TO_REGION[country] === selectedRegion;
			});
		}

		// Client-side city filter
		if (selectedCity) {
			result = result.filter((o) => o.datacenter_city === selectedCity);
		}

		// Client-side cores filter (null = 0, no minimum)
		const coresThreshold = minCores ?? 0;
		if (coresThreshold > 0) {
			result = result.filter(
				(o) => (o.processor_cores ?? 0) >= coresThreshold,
			);
		}

		// Client-side memory filter (null = 0, no minimum)
		const memoryThreshold = minMemoryGb ?? 0;
		if (memoryThreshold > 0) {
			result = result.filter((o) => {
				const mem = o.memory_amount;
				if (!mem) return false;
				const match = mem.match(/(\d+)/);
				if (!match) return false;
				return parseInt(match[1], 10) >= memoryThreshold;
			});
		}

		// Client-side SSD filter (null = 0, no minimum)
		const ssdThreshold = minSsdGb ?? 0;
		if (ssdThreshold > 0) {
			result = result.filter((o) => {
				const ssd = o.total_ssd_capacity;
				if (!ssd) return false;
				const match = ssd.match(/(\d+)/);
				if (!match) return false;
				let value = parseInt(match[1], 10);
				// Convert TB to GB if needed
				if (ssd.toLowerCase().includes("tb")) value *= 1000;
				return value >= ssdThreshold;
			});
		}

		// Client-side virtualization type filter
		if (selectedVirt) {
			result = result.filter(
				(o) =>
					o.virtualization_type?.toLowerCase() ===
					selectedVirt.toLowerCase(),
			);
		}

		// Client-side unmetered filter
		if (unmeteredOnly) {
			result = result.filter((o) => o.unmetered_bandwidth);
		}

		// Client-side trust filter (null = 0, no minimum)
		const trustThreshold = minTrust ?? 0;
		if (trustThreshold > 0) {
			result = result.filter((o) => (o.trust_score ?? 0) >= trustThreshold);
		}

		// Hide demo offerings if toggle is off (non-demo always shown)
		if (!showDemoOfferings) {
			result = result.filter((o) => !o.is_example);
		}

		// Hide offline offerings if toggle is off (online always shown)
		if (!showOfflineOfferings) {
			result = result.filter((o) => o.provider_online);
		}

		// Sort by selected field
		result.sort((a, b) => {
			if (sortField === "trust") {
				const ta = a.trust_score ?? -1;
				const tb = b.trust_score ?? -1;
				return tb - ta; // descending always (highest trust first)
			}
			const priceA = a.monthly_price ?? Infinity;
			const priceB = b.monthly_price ?? Infinity;
			return sortDir === "asc" ? priceA - priceB : priceB - priceA;
		});

		return result;
	});

	authStore.isAuthenticated.subscribe((value) => {
		isAuthenticated = value;
	});

	async function fetchOfferings() {
		try {
			loading = true;
			error = null;
			offerings = await searchOfferings({
				limit: 100,
				in_stock_only: true,
				has_recipe: recipesOnly || undefined,
				q: searchQuery.trim() || undefined,
				country: selectedCountry || undefined,
				min_price_monthly: minPrice ?? undefined,
				max_price_monthly: maxPrice ?? undefined,
			});

			// Update stable country list on initial load or when not filtering by country
			// This keeps all countries available in the dropdown even when filtering
			if (!selectedCountry || allCountries.length === 0) {
				const uniqueCountries = [...new Set(
					offerings.map((o) => o.datacenter_country).filter(Boolean)
				)].sort();
				// Only update if we got more countries (initial load or expansion)
				if (uniqueCountries.length >= allCountries.length) {
					allCountries = uniqueCountries;
				}
			}
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load offerings";
		} finally {
			loading = false;
		}
	}

	onMount(async () => {
		providerCtaDismissed = localStorage.getItem(PROVIDER_CTA_KEY) === '1';
		[, icpPriceUsd] = await Promise.all([fetchOfferings(), fetchIcpPrice()]);
		const offeringParam = $page.url.searchParams.get("offering");
		if (offeringParam) {
			const id = parseInt(offeringParam, 10);
			if (!isNaN(id)) {
				expandedRow = id;
				await tick();
				document.getElementById(`offering-${id}`)?.scrollIntoView({ behavior: "smooth", block: "center" });
			}
		}
	});

	function handleSearchInput() {
		if (debounceTimer) clearTimeout(debounceTimer);
		debounceTimer = setTimeout(() => fetchOfferings(), 300);
	}

	function handleFilterChange() {
		fetchOfferings();
	}

	function toggleType(type: string) {
		const newSet = new Set(selectedTypes);
		if (newSet.has(type)) newSet.delete(type);
		else newSet.add(type);
		selectedTypes = newSet;
	}

	function clearFilters() {
		selectedTypes = new Set();
		minPrice = null;
		maxPrice = null;
		selectedRegion = "";
		selectedCountry = "";
		selectedCity = "";
		minCores = null;
		minMemoryGb = null;
		minSsdGb = null;
		selectedVirt = "";
		unmeteredOnly = false;
		minTrust = null;
		showDemoOfferings = false;
		showOfflineOfferings = false;
		recipesOnly = false;
		searchQuery = "";
		fetchOfferings();
	}

	function handleRentClick(e: Event, offering: Offering) {
		e.stopPropagation();
		if (!isAuthenticated) {
			showAuthModal = true;
			return;
		}
		selectedOffering = offering;
	}

	let copyLinkFeedback = $state<number | null>(null);

	let compareIds = $state(new Set<number>());
	let showCompareModal = $state(false);
	let compareWarning = $state<string | null>(null);
	let compareOfferings = $derived(
		offerings.filter((o) => o.id !== undefined && compareIds.has(o.id)),
	);

	function toggleCompare(e: Event, id: number) {
		e.stopPropagation();
		if (compareIds.has(id)) {
			compareIds = removeFromComparison(compareIds, id);
		} else {
			try {
				compareIds = addToComparison(compareIds, id);
			} catch {
				compareWarning = COMPARE_MAX_ERROR;
				setTimeout(() => {
					compareWarning = null;
				}, 2500);
			}
		}
	}

	function toggleRow(id: number | undefined) {
		if (id === undefined) return;
		expandedRow = expandedRow === id ? null : id;
		// Sync URL query param without navigation
		const url = new URL(window.location.href);
		if (expandedRow !== null) {
			url.searchParams.set("offering", String(expandedRow));
		} else {
			url.searchParams.delete("offering");
		}
		history.replaceState(history.state, "", url.toString());
	}

	function copyOfferingLink(offeringId: number | undefined, event: Event) {
		if (offeringId === undefined) return;
		event.stopPropagation();
		const url = new URL(window.location.href);
		url.search = "";
		url.searchParams.set("offering", String(offeringId));
		navigator.clipboard.writeText(url.toString());
		copyLinkFeedback = offeringId;
		setTimeout(() => { copyLinkFeedback = null; }, 2000);
	}

	function setSortPrice(dir: "asc" | "desc") {
		sortField = "price";
		sortDir = dir;
	}

	function setSortTrust() {
		sortField = "trust";
	}

	function handleRentalSuccess(contractId: string) {
		selectedOffering = null;
		successMessage = `Rental request created! Contract ID: ${contractId}`;
		setTimeout(() => (successMessage = null), 5000);
	}

	function getTypeIcon(productType: string): IconName {
		const type = productType.toLowerCase();
		if (type.includes("gpu")) return "gpu";
		if (type.includes("compute") || type.includes("vm")) return "cpu";
		if (type.includes("storage")) return "hard-drive";
		if (type.includes("network") || type.includes("cdn")) return "globe";
		return "package";
	}

	function formatPrice(offering: Offering): string {
		// If offering has a reseller, calculate price with commission
		if (offering.reseller_commission_percent && offering.monthly_price) {
			const basePrice = offering.monthly_price;
			const commission =
				basePrice * (offering.reseller_commission_percent / 100);
			const totalPrice = basePrice + commission;
			return `${totalPrice.toFixed(2)} ${offering.currency}`;
		}
		if (offering.monthly_price)
			return `${offering.monthly_price.toFixed(2)} ${offering.currency}`;
		return "On request";
	}

	function formatUsdEquivalent(offering: Offering): string | null {
		if (!icpPriceUsd || !offering.monthly_price) return null;
		const currency = offering.currency?.toUpperCase();
		if (currency !== "ICP") return null;
		let price = offering.monthly_price;
		if (offering.reseller_commission_percent) {
			price += price * (offering.reseller_commission_percent / 100);
		}
		const usd = price * icpPriceUsd;
		return `≈ $${usd.toFixed(2)}/mo`;
	}

	function hasReseller(offering: Offering): boolean {
		return !!(
			offering.reseller_name && offering.reseller_commission_percent
		);
	}

	function getResellerBadgeText(offering: Offering): string {
		if (!offering.reseller_name) return "";
		const commission = offering.reseller_commission_percent || 0;
		return `Via ${offering.reseller_name} (+${commission}%)`;
	}

	function formatSpecs(offering: Offering): string {
		const type = offering.product_type.toLowerCase();
		if (type.includes("gpu")) {
			const parts = [
				offering.gpu_name,
				offering.gpu_count ? `${offering.gpu_count}x` : null,
				offering.gpu_memory_gb ? `${offering.gpu_memory_gb}GB` : null,
			].filter(Boolean);
			return parts.join(" ") || "—";
		}
		const parts = [
			offering.processor_cores
				? `${offering.processor_cores} vCPU`
				: null,
			offering.memory_amount,
			offering.total_ssd_capacity
				? `${offering.total_ssd_capacity} SSD`
				: offering.total_hdd_capacity
					? `${offering.total_hdd_capacity} HDD`
					: null,
		].filter(Boolean);
		return parts.join(" · ") || "—";
	}

	function formatLocation(offering: Offering): string {
		if (offering.datacenter_city && offering.datacenter_country) {
			return `${offering.datacenter_city}, ${offering.datacenter_country}`;
		}
		return offering.datacenter_country || "—";
	}

	const typeOptions: { key: string; label: string; icon: IconName }[] = [
		{ key: "compute", label: "Compute", icon: "cpu" },
		{ key: "gpu", label: "GPU", icon: "gpu" },
		{ key: "storage", label: "Storage", icon: "hard-drive" },
		{ key: "network", label: "Network", icon: "globe" },
	];

	function formatContractTerms(offering: Offering): string {
		const parts: string[] = [];
		if (offering.min_contract_hours) {
			const hours = offering.min_contract_hours;
			if (hours >= 720) parts.push(`Min ${Math.round(hours / 720)}mo`);
			else if (hours >= 24) parts.push(`Min ${Math.round(hours / 24)}d`);
			else parts.push(`Min ${hours}h`);
		}
		if (offering.max_contract_hours) {
			const hours = offering.max_contract_hours;
			if (hours >= 720) parts.push(`Max ${Math.round(hours / 720)}mo`);
			else if (hours >= 24) parts.push(`Max ${Math.round(hours / 24)}d`);
			else parts.push(`Max ${hours}h`);
		}
		return parts.length > 0 ? parts.join(" · ") : "—";
	}

	function formatBilling(offering: Offering): string {
		const interval = offering.billing_interval?.toLowerCase() || "";
		if (interval.includes("hour")) return "Hourly";
		if (interval.includes("day")) return "Daily";
		if (interval.includes("month")) return "Monthly";
		if (interval.includes("year")) return "Yearly";
		return offering.billing_interval || "—";
	}

	function getSubscriptionBadge(offering: Offering): string | null {
		if (!offering.is_subscription) return null;
		const days = offering.subscription_interval_days;
		if (!days) return "Recurring";
		if (days <= 31) return "Monthly";
		if (days <= 93) return "Quarterly";
		if (days <= 366) return "Yearly";
		return `${days}d`;
	}

	let activeFilterChips = $derived.by(() => {
		const chips: Array<{ label: string; remove: () => void }> = [];

		for (const t of selectedTypes) {
			const opt = typeOptions.find((o) => o.key === t);
			const label = opt ? opt.label : t;
			chips.push({ label, remove: () => { const s = new Set(selectedTypes); s.delete(t); selectedTypes = s; } });
		}
		if (searchQuery) {
			chips.push({ label: `Search: ${searchQuery}`, remove: () => { searchQuery = ""; fetchOfferings(); } });
		}
		if (selectedRegion) {
			const region = REGIONS.find((r) => r.code === selectedRegion);
			chips.push({ label: `Region: ${region?.name ?? selectedRegion}`, remove: () => { selectedRegion = ""; selectedCountry = ""; selectedCity = ""; } });
		}
		if (selectedCountry) {
			chips.push({ label: `Country: ${selectedCountry}`, remove: () => { selectedCountry = ""; selectedCity = ""; handleFilterChange(); } });
		}
		if (selectedCity) {
			chips.push({ label: `City: ${selectedCity}`, remove: () => { selectedCity = ""; } });
		}
		if (minPrice !== null) {
			chips.push({ label: `Min price: ${minPrice} ICP`, remove: () => { minPrice = null; handleFilterChange(); } });
		}
		if (maxPrice !== null) {
			chips.push({ label: `Max price: ${maxPrice} ICP`, remove: () => { maxPrice = null; handleFilterChange(); } });
		}
		if (minCores !== null) {
			chips.push({ label: `Min cores: ${minCores}`, remove: () => { minCores = null; } });
		}
		if (minMemoryGb !== null) {
			chips.push({ label: `Min RAM: ${minMemoryGb}GB`, remove: () => { minMemoryGb = null; } });
		}
		if (minSsdGb !== null) {
			chips.push({ label: `Min SSD: ${minSsdGb}GB`, remove: () => { minSsdGb = null; } });
		}
		if (selectedVirt) {
			chips.push({ label: `Virt: ${selectedVirt.toUpperCase()}`, remove: () => { selectedVirt = ""; } });
		}
		if (unmeteredOnly) {
			chips.push({ label: "Unmetered", remove: () => { unmeteredOnly = false; } });
		}
		if (minTrust !== null) {
			chips.push({ label: `Trust ≥ ${minTrust}`, remove: () => { minTrust = null; } });
		}
		if (showDemoOfferings) {
			chips.push({ label: "Showing demos", remove: () => { showDemoOfferings = false; } });
		}
		if (showOfflineOfferings) {
			chips.push({ label: "Showing offline", remove: () => { showOfflineOfferings = false; } });
		}
		if (recipesOnly) {
			chips.push({ label: "Recipes only", remove: () => { recipesOnly = false; handleFilterChange(); } });
		}

		return chips;
	});
</script>

<div class="space-y-4">
	<div>
		<h1 class="text-2xl font-bold text-white tracking-tight">Marketplace</h1>
		<p class="text-neutral-500 text-sm mt-1">Find and rent cloud resources</p>
	</div>

	{#if successMessage}
		<div class="bg-success/10 border border-success/20 p-3 text-success text-sm">
			{successMessage}
		</div>
	{/if}

	{#if error}
		<div class="bg-danger/10 border border-danger/20 p-3 text-danger text-sm">
			{error}
		</div>
	{/if}

	{#if isAuthenticated && !providerCtaDismissed}
		<div class="bg-primary-500/10 border border-primary-500/30 p-4 flex items-center justify-between gap-4">
			<div class="flex items-center gap-3">
				<Icon name="server" size={20} class="text-primary-400 shrink-0" />
				<p class="text-sm text-neutral-300">
					Have infrastructure to share? <a href="/dashboard/provider/support" class="text-primary-400 hover:text-primary-300 font-medium">Become a provider</a> and earn by renting out your resources.
				</p>
			</div>
			<button
				onclick={() => { providerCtaDismissed = true; localStorage.setItem(PROVIDER_CTA_KEY, '1'); }}
				class="text-neutral-500 hover:text-white transition-colors shrink-0"
				aria-label="Dismiss"
			>
				<Icon name="x" size={16} />
			</button>
		</div>
	{/if}

	<div class="flex flex-col md:flex-row gap-6">
		<!-- Filters: collapsible on mobile, sidebar on desktop -->
		<aside class="w-full md:w-56 shrink-0">
			<div class="card p-4">
				<div class="flex items-center justify-between">
					<button
						onclick={() => (showFilters = !showFilters)}
						class="flex items-center gap-2 md:cursor-default"
					>
						<span class="text-white font-medium text-sm">Filters</span>
						<span class="md:hidden">
							{#if showFilters}
								<Icon name="chevron-up" size={20} class="text-neutral-500" />
							{:else}
								<Icon name="chevron-down" size={20} class="text-neutral-500" />
							{/if}
						</span>
					</button>
					<button
						onclick={clearFilters}
						class="text-xs text-primary-400 hover:text-primary-300"
					>Clear</button>
				</div>

				<div
					class="space-y-4 mt-4 {showFilters
						? ''
						: 'hidden'} md:block"
				>
					<!-- Type Filter -->
					<div>
						<div
							class="data-label mb-2"
						>
							Type
						</div>
						<div class="space-y-1">
							{#each typeOptions as opt}
								<label
									class="flex items-center gap-2 cursor-pointer group"
								>
									<input
										type="checkbox"
										checked={selectedTypes.has(opt.key)}
										onchange={() => toggleType(opt.key)}
										class="border-neutral-700 bg-base text-primary-500 focus:ring-primary-500"
									/>
									<span
										class="flex items-center gap-1.5 text-sm text-neutral-300 group-hover:text-white"
										><Icon name={opt.icon} size={20} /> {opt.label}</span
									>
								</label>
							{/each}
						</div>
					</div>

					<!-- Price Filter -->
					<div>
						<div
							class="data-label mb-2"
						>
							Price ($/mo)
						</div>
						<div class="flex gap-2">
							<input
								type="number"
								placeholder="Min"
								bind:value={minPrice}
								onchange={handleFilterChange}
								class="w-full px-2 py-1.5 text-sm input focus:outline-none focus:border-primary-400"
							/>
							<input
								type="number"
								placeholder="Max"
								bind:value={maxPrice}
								onchange={handleFilterChange}
								class="w-full px-2 py-1.5 text-sm input focus:outline-none focus:border-primary-400"
							/>
						</div>
					</div>

					<!-- Region Filter -->
					<div>
						<div
							class="data-label mb-2"
						>
							Region
						</div>
						<select
							bind:value={selectedRegion}
							onchange={() => {
								selectedCountry = "";
								selectedCity = "";
							}}
							class="w-full px-2 py-1.5 text-sm input focus:outline-none focus:border-primary-400"
						>
							<option value="">All regions</option>
							{#each REGIONS as region}
								<option value={region.code}>{region.name}</option>
							{/each}
						</select>
					</div>

					<!-- Country Filter -->
					<div>
						<div
							class="data-label mb-2"
						>
							Country
						</div>
						<select
							bind:value={selectedCountry}
							onchange={() => {
								selectedCity = "";
								handleFilterChange();
							}}
							class="w-full px-2 py-1.5 text-sm input focus:outline-none focus:border-primary-400"
						>
							<option value="">All countries</option>
							{#each countries() as country}
								<option value={country}>{country}</option>
							{/each}
						</select>
					</div>

					<!-- City Filter -->
					<div>
						<div
							class="data-label mb-2"
						>
							City
						</div>
						<select
							bind:value={selectedCity}
							class="w-full px-2 py-1.5 text-sm input focus:outline-none focus:border-primary-400"
						>
							<option value="">All cities</option>
							{#each cities() as city}
								<option value={city}>{city}</option>
							{/each}
						</select>
					</div>

					<!-- CPU Cores Filter -->
					<div>
						<div
							class="data-label mb-2"
						>
							Min CPU Cores
						</div>
						<input
							type="number"
							placeholder="e.g., 4"
							bind:value={minCores}
							min="1"
							class="w-full px-2 py-1.5 text-sm input focus:outline-none focus:border-primary-400"
						/>
					</div>

					<!-- Memory Filter -->
					<div>
						<div
							class="data-label mb-2"
						>
							Min Memory (GB)
						</div>
						<input
							type="number"
							placeholder="e.g., 8"
							bind:value={minMemoryGb}
							min="1"
							class="w-full px-2 py-1.5 text-sm input focus:outline-none focus:border-primary-400"
						/>
					</div>

					<!-- SSD Filter -->
					<div>
						<div
							class="data-label mb-2"
						>
							Min SSD (GB)
						</div>
						<input
							type="number"
							placeholder="e.g., 100"
							bind:value={minSsdGb}
							min="1"
							class="w-full px-2 py-1.5 text-sm input focus:outline-none focus:border-primary-400"
						/>
					</div>

					<!-- Virtualization Type Filter -->
					{#if virtTypes.length > 0}
						<div>
							<div
								class="data-label mb-2"
							>
								Virtualization
							</div>
							<select
								bind:value={selectedVirt}
								class="w-full px-2 py-1.5 text-sm input focus:outline-none focus:border-primary-400"
							>
								<option value="">All types</option>
								{#each virtTypes as vt}
									<option value={vt}>{vt.toUpperCase()}</option>
								{/each}
							</select>
						</div>
					{/if}

					<!-- Min Trust Filter -->
					<div>
						<div
							class="data-label mb-2"
						>
							Min Trust Score
						</div>
						<input
							type="number"
							placeholder="0-100"
							bind:value={minTrust}
							min="0"
							max="100"
							class="w-full px-2 py-1.5 text-sm input focus:outline-none focus:border-primary-400"
						/>
					</div>

					<!-- Unmetered Bandwidth Filter -->
					<div>
						<label
							class="flex items-center gap-2 cursor-pointer group"
						>
							<input
								type="checkbox"
								bind:checked={unmeteredOnly}
								class="border-neutral-700 bg-base text-primary-500 focus:ring-primary-500"
							/>
							<span
								class="text-sm text-neutral-400 group-hover:text-white"
								>Unmetered bandwidth only</span
							>
						</label>
					</div>

					<!-- Show Demo Offerings Filter -->
					<div>
						<label
							class="flex items-center gap-2 cursor-pointer group"
						>
							<input
								type="checkbox"
								bind:checked={showDemoOfferings}
								class="border-neutral-700 bg-base text-primary-500 focus:ring-primary-500"
							/>
							<span
								class="text-sm text-neutral-400 group-hover:text-white"
								>Show demo offerings</span
							>
						</label>
					</div>

					<!-- Show Offline Offerings Filter -->
					<div>
						<label
							class="flex items-center gap-2 cursor-pointer group"
						>
							<input
								type="checkbox"
								bind:checked={showOfflineOfferings}
								class="border-neutral-700 bg-base text-primary-500 focus:ring-primary-500"
							/>
							<span
								class="text-sm text-neutral-400 group-hover:text-white"
								>Show offline offerings</span
							>
						</label>
					</div>

					<!-- Recipes Only Filter -->
					<div>
						<label
							class="flex items-center gap-2 cursor-pointer group"
						>
							<input
								type="checkbox"
								bind:checked={recipesOnly}
								onchange={handleFilterChange}
								class="border-neutral-700 bg-base text-primary-500 focus:ring-primary-500"
							/>
							<span
								class="text-sm text-neutral-400 group-hover:text-white"
								>Recipes only</span
							>
						</label>
					</div>
				</div>
			</div>
		</aside>

		<!-- Main Content -->
		<div class="flex-1 min-w-0 space-y-4">
			<!-- Search Bar with Icon -->
			<div class="relative">
				<div class="absolute left-4 top-1/2 -translate-y-1/2 pointer-events-none">
					<Icon name="search" size={20} class="text-neutral-500" />
				</div>
				<input
					type="text"
					placeholder="Search by name, description, or type..."
					bind:value={searchQuery}
					oninput={handleSearchInput}
					class="w-full pl-11 pr-4 py-3 bg-surface-elevated border border-neutral-800 text-white placeholder-neutral-500 focus:outline-none focus:border-primary-400 transition-colors"
				/>
			</div>

			<!-- Active Filter Chips -->
			{#if activeFilterChips.length > 0}
				<div class="flex flex-wrap items-center gap-2">
					<span class="text-xs text-neutral-500 shrink-0">Active filters:</span>
					{#each activeFilterChips as chip}
						<button
							onclick={chip.remove}
							class="inline-flex items-center gap-1 px-2 py-1 text-xs bg-primary-500/20 text-primary-400 border border-primary-500/30 rounded hover:bg-primary-500/30 transition-colors"
						>
							{chip.label}
							<span class="text-primary-300 hover:text-white transition-colors leading-none">&times;</span>
						</button>
					{/each}
					<button
						onclick={clearFilters}
						class="text-xs text-neutral-500 hover:text-white transition-colors"
					>Clear all</button>
				</div>
			{/if}

			<!-- Results bar with count and sort -->
			<div class="flex items-center justify-between">
				<div class="text-neutral-500 text-sm">
					{filteredOfferings.length} offerings found
				</div>
				<div class="hidden md:flex items-center gap-1">
					<button
						onclick={() => setSortPrice("asc")}
						class="px-2 py-1 text-xs rounded transition-colors {sortField === 'price' && sortDir === 'asc' ? 'bg-primary-500/20 text-primary-400 border border-primary-500/30' : 'text-neutral-500 hover:text-white'}"
					>Price ↑</button>
					<button
						onclick={() => setSortPrice("desc")}
						class="px-2 py-1 text-xs rounded transition-colors {sortField === 'price' && sortDir === 'desc' ? 'bg-primary-500/20 text-primary-400 border border-primary-500/30' : 'text-neutral-500 hover:text-white'}"
					>Price ↓</button>
					<button
						onclick={setSortTrust}
						class="px-2 py-1 text-xs rounded transition-colors {sortField === 'trust' ? 'bg-primary-500/20 text-primary-400 border border-primary-500/30' : 'text-neutral-500 hover:text-white'}"
					>Trust ↓</button>
				</div>
			</div>

			{#if loading}
				<div class="flex justify-center py-12">
					<div
						class="animate-spin rounded-full h-10 w-10 border-t-2 border-b-2 border-primary-400"
					></div>
				</div>
			{:else if filteredOfferings.length === 0}
				<div class="text-center py-12">
					<div class="flex justify-center mb-3">
						<Icon name="search" size={48} class="text-neutral-600" />
					</div>
					<p class="text-neutral-500 mb-2">No offerings found</p>
					{#if selectedTypes.size > 0 || minPrice !== null || maxPrice !== null || selectedRegion || selectedCountry || selectedCity || minCores !== null || minMemoryGb !== null || minSsdGb !== null || selectedVirt || unmeteredOnly || minTrust !== null || !showDemoOfferings || showOfflineOfferings || recipesOnly || searchQuery}
						<p class="text-neutral-600 text-sm mb-4">Your active filters are narrowing the results.</p>
						<button onclick={clearFilters} class="px-4 py-2 bg-primary-600 hover:bg-primary-500 text-white text-sm font-medium transition-colors">
							Clear all filters
						</button>
					{/if}
				</div>
			{:else}
				<!-- Desktop Table -->
				<div class="hidden md:block overflow-x-auto">
					<table class="w-full text-sm">
						<thead>
							<tr
								class="text-left text-neutral-500 border-b border-neutral-800"
							>
								<th class="pb-3 font-medium">Offering</th>
								<th class="pb-3 font-medium">Type</th>
								<th class="pb-3 font-medium">Specs</th>
								<th class="pb-3 font-medium">Location</th>
								<th class="pb-3 font-medium">
									<span class="inline-flex items-center gap-1">Price</span>
								</th>
								<th class="pb-3 font-medium"></th>
								<th class="pb-3 font-medium text-right text-neutral-500">Compare</th>
							</tr>
						</thead>
						<tbody>
							{#each filteredOfferings as offering (offering.id)}
								{@const isExpanded =
									expandedRow === offering.id}
								<tr
									id="offering-{offering.id}"
									class="border-b border-neutral-800/60 hover:bg-surface-elevated cursor-pointer transition-colors"
									onclick={() => toggleRow(offering.id)}
								>
									<td class="py-3 pr-4">
										<div class="flex items-center gap-2">
											<a
												href="/dashboard/marketplace/{offering.id}"
												onclick={(e) => e.stopPropagation()}
												class="font-medium text-white hover:text-primary-400 transition-colors"
											>{offering.offer_name}</a
											>
											{#if !offering.provider_online}
												<span
													class="flex items-center gap-1 px-1.5 py-0.5 text-xs bg-red-500/20 text-red-400 rounded"
													title="Provider agent is offline - provisioning may be delayed"
												>
													<span class="h-1.5 w-1.5 rounded-full bg-red-400"></span>
													Offline
												</span>
											{/if}
											{#if offering.trust_score !== undefined}
												<TrustBadge
													score={offering.trust_score}
													hasFlags={offering.has_critical_flags ??
														false}
													compact={true}
												/>
											{/if}
											{#if hasReseller(offering)}
												<span
													class="px-1.5 py-0.5 text-xs bg-primary-500/20 text-primary-400 rounded"
													>{getResellerBadgeText(
														offering,
													)}</span
												>
											{:else if offering.offering_source === "self_provisioned"}
												<span
													class="px-1.5 py-0.5 text-xs bg-emerald-500/20 text-emerald-400 rounded"
													>Self-Hosted</span
												>
											{:else if offering.offering_source === "seeded"}
												<span
													class="px-1.5 py-0.5 text-xs bg-purple-500/20 text-purple-400 rounded"
													>External</span
												>
											{:else if offering.is_example}
												<span
													class="px-1.5 py-0.5 text-xs bg-amber-500/20 text-amber-400 rounded"
													>Demo</span
												>
											{/if}
											{#if getSubscriptionBadge(offering)}
												<span
													class="inline-flex items-center gap-1 px-1.5 py-0.5 text-xs bg-purple-500/20 text-purple-400 rounded"
													title="Recurring subscription"
													><Icon name="repeat" size={20} class="text-purple-400" /> {getSubscriptionBadge(offering)}</span
												>
											{/if}
											{#if offering.post_provision_script}
												<span
													class="px-1.5 py-0.5 text-xs bg-blue-500/20 text-blue-400 rounded"
													>Recipe</span
												>
											{/if}
										</div>
										<a
											href="/dashboard/providers/{offering.owner_username ||
												offering.pubkey}"
											onclick={(e) => e.stopPropagation()}
											class="text-xs text-neutral-500 hover:text-primary-400 {offering.owner_username
												? ''
												: 'font-mono'}"
											>{offering.owner_username
												? `@${offering.owner_username}`
												: truncatePubkey(
														offering.pubkey,
													)}</a
										>
									</td>
									<td class="py-3 pr-4">
										<span class="inline-flex items-center gap-1.5 whitespace-nowrap text-neutral-300"
											><Icon name={getTypeIcon(offering.product_type)} size={20} />
											{offering.product_type}</span
										>
									</td>
									<td class="py-3 pr-4 text-neutral-300"
										>{formatSpecs(offering)}</td
									>
									<td class="py-3 pr-4 text-neutral-300"
										>{formatLocation(offering)}</td
									>
									<td class="py-3 pr-4">
										<div class="font-medium text-white">{formatPrice(offering)}</div>
										{#if formatUsdEquivalent(offering)}
											<div class="text-xs text-neutral-500">{formatUsdEquivalent(offering)}</div>
										{/if}
									</td>
									<td class="py-3">
										{#if hasReseller(offering)}
											<button
												onclick={(e) =>
													handleRentClick(
														e,
														offering,
													)}
												class="px-3 py-1.5 bg-primary-600 hover:bg-primary-500 rounded text-xs font-medium whitespace-nowrap"
												>Rent</button
											>
										{:else if offering.offering_source === "seeded" && offering.external_checkout_url}
											<a
												href={offering.external_checkout_url}
												target="_blank"
												rel="noopener noreferrer"
												onclick={(e) =>
													e.stopPropagation()}
												class="inline-flex items-center gap-1 px-3 py-1.5 bg-primary-600 hover:bg-primary-500 rounded text-xs font-medium whitespace-nowrap"
												>Visit Provider <Icon name="external" size={20} class="text-white" /></a
											>
										{:else if offering.is_example}
											<span title="Demo only — not available for rent" class="px-3 py-1.5 bg-neutral-700 text-neutral-500 rounded text-xs font-medium cursor-not-allowed whitespace-nowrap">Demo only</span>
										{:else}
											<button
												onclick={(e) =>
													handleRentClick(
														e,
														offering,
													)}
												class="px-3 py-1.5 bg-primary-600 hover:bg-primary-500 rounded text-xs font-medium whitespace-nowrap"
												>Rent</button
											>
										{/if}
									</td>
									<td class="py-3 text-right">
										{#if offering.id !== undefined}
											{@const inCompare = compareIds.has(offering.id)}
											<button
												onclick={(e) => toggleCompare(e, offering.id!)}
												title={inCompare
													? "Remove from comparison"
													: "Add to comparison"}
												class="px-2 py-1 text-xs border rounded transition-colors {inCompare
													? 'bg-primary-500/20 text-primary-300 border-primary-400/50 hover:bg-primary-500/10'
													: 'bg-neutral-800 text-neutral-400 border-neutral-700 hover:bg-neutral-700 hover:text-white'}"
											>{inCompare ? "✓ Compare" : "+ Compare"}</button
											>
										{/if}
									</td>
								</tr>
								{#if isExpanded}
									<tr class="bg-surface-elevated">
										<td colspan="7" class="p-4">
											<div
												class="grid grid-cols-3 gap-4 text-sm"
											>
												<div>
													<div
														class="text-neutral-500 text-xs uppercase mb-1"
													>
														Description
													</div>
													<div class="text-neutral-300">
														{offering.description ||
															"No description"}
													</div>
													{#if offering.post_provision_script}
														<details class="mt-3">
															<summary class="text-xs text-blue-400 cursor-pointer hover:text-blue-300">View recipe script</summary>
															<pre class="mt-2 p-3 bg-base/50 border border-neutral-800 text-xs text-neutral-300 font-mono overflow-x-auto max-h-48 overflow-y-auto whitespace-pre-wrap">{offering.post_provision_script}</pre>
														</details>
													{/if}
												</div>
												<div class="space-y-2">
													{#if offering.processor_name || offering.processor_brand}
														<div>
															<span
																class="text-neutral-500"
																>CPU:</span
															>
															<span
																class="text-neutral-300"
																>{offering.processor_name ||
																	offering.processor_brand}{offering.processor_speed
																	? ` @ ${offering.processor_speed}`
																	: ""}</span
															>
														</div>
													{/if}
													{#if offering.memory_amount}
														<div>
															<span
																class="text-neutral-500"
																>Memory:</span
															>
															<span
																class="text-neutral-300"
																>{offering.memory_amount}{offering.memory_type
																	? ` ${offering.memory_type}`
																	: ""}{offering.memory_error_correction
																	? ` (${offering.memory_error_correction})`
																	: ""}</span
															>
														</div>
													{/if}
													{#if offering.total_ssd_capacity || offering.total_hdd_capacity}
														<div>
															<span
																class="text-neutral-500"
																>Storage:</span
															>
															<span
																class="text-neutral-300"
																>{[
																	offering.total_ssd_capacity
																		? `${offering.total_ssd_capacity} SSD`
																		: null,
																	offering.total_hdd_capacity
																		? `${offering.total_hdd_capacity} HDD`
																		: null,
																]
																	.filter(
																		Boolean,
																	)
																	.join(
																		" + ",
																	)}</span
															>
														</div>
													{/if}
													{#if offering.uplink_speed || offering.unmetered_bandwidth}
														<div>
															<span
																class="text-neutral-500"
																>Network:</span
															>
															<span
																class="text-neutral-300"
																>{offering.uplink_speed ||
																	""}{offering.unmetered_bandwidth
																	? " (Unmetered)"
																	: offering.traffic
																		? ` (${offering.traffic} TB)`
																		: ""}</span
															>
														</div>
													{/if}
													{#if offering.virtualization_type}
														<div>
															<span
																class="text-neutral-500"
																>Platform:</span
															>
															<span
																class="text-neutral-300"
																>{offering.virtualization_type}</span
															>
														</div>
													{/if}
												</div>
												<div class="space-y-2">
													<div>
														<span
															class="text-neutral-500"
															>Billing:</span
														>
														<span
															class="text-neutral-300"
															>{formatBilling(
																offering,
															)}</span
														>
													</div>
													{#if offering.setup_fee > 0}
														<div>
															<span
																class="text-neutral-500"
																>Setup Fee:</span
															>
															<span
																class="text-neutral-300"
																>{offering.setup_fee.toFixed(
																	2,
																)}
																{offering.currency}</span
															>
														</div>
													{/if}
													{#if offering.min_contract_hours || offering.max_contract_hours}
														<div>
															<span
																class="text-neutral-500"
																>Contract:</span
															>
															<span
																class="text-neutral-300"
																>{formatContractTerms(
																	offering,
																)}</span
															>
														</div>
													{/if}
													{#if offering.operating_systems}
														<div>
															<span
																class="text-neutral-500"
																>OS:</span
															>
															<span
																class="text-neutral-300"
																>{offering.operating_systems}</span
															>
														</div>
													{/if}
													{#if offering.features}
														<div>
															<span
																class="text-neutral-500"
																>Features:</span
															>
															<span
																class="text-neutral-300"
																>{offering.features}</span
															>
														</div>
													{/if}
													{#if offering.control_panel}
														<div>
															<span
																class="text-neutral-500"
																>Control Panel:</span
															>
															<span
																class="text-neutral-300"
																>{offering.control_panel}</span
															>
														</div>
													{/if}
												</div>
												<div class="mt-3 flex flex-col gap-2">
													<button
														onclick={(e) => copyOfferingLink(offering.id, e)}
														class="inline-flex items-center gap-1.5 text-xs text-neutral-500 hover:text-primary-400 transition-colors"
													>
														{#if copyLinkFeedback === offering.id}
															<Icon name="check" size={14} class="text-green-400" /> Copied!
														{:else}
															<Icon name="link" size={14} /> Copy link
														{/if}
													</button>
													<a
														href="/dashboard/providers/{offering.owner_username || offering.pubkey}"
														onclick={(e) => e.stopPropagation()}
														class="inline-flex items-center gap-1.5 text-xs text-neutral-500 hover:text-primary-400 transition-colors"
													>
														<Icon name="user" size={14} /> View provider profile
													</a>
												</div>
											</div>
										</td>
									</tr>
								{/if}
							{/each}
						</tbody>
					</table>
				</div>

				<!-- Mobile Cards -->
				<div class="md:hidden space-y-3">
					{#each filteredOfferings as offering (offering.id)}
						<div
							id="offering-{offering.id}"
							role="button"
							tabindex="0"
							class="bg-surface-elevated  p-4 border border-neutral-800"
							onclick={() => toggleRow(offering.id)}
							onkeydown={(e) =>
								e.key === "Enter" && toggleRow(offering.id)}
						>
							<div class="flex items-start justify-between mb-2">
								<div>
									<div class="flex items-center gap-2 flex-wrap">
										<a
											href="/dashboard/marketplace/{offering.id}"
											onclick={(e) => e.stopPropagation()}
											class="font-medium text-white hover:text-primary-400 transition-colors"
										>{offering.offer_name}</a
										>
										{#if !offering.provider_online}
											<span
												class="flex items-center gap-1 px-1.5 py-0.5 text-xs bg-red-500/20 text-red-400 rounded"
												title="Provider agent is offline - provisioning may be delayed"
											>
												<span class="h-1.5 w-1.5 rounded-full bg-red-400"></span>
												Offline
											</span>
										{/if}
										{#if hasReseller(offering)}
											<span
												class="px-1.5 py-0.5 text-xs bg-primary-500/20 text-primary-400 rounded"
												>{getResellerBadgeText(
													offering,
												)}</span
											>
										{:else if offering.offering_source === "self_provisioned"}
											<span
												class="px-1.5 py-0.5 text-xs bg-emerald-500/20 text-emerald-400 rounded"
												>Self-Hosted</span
											>
										{:else if offering.offering_source === "seeded"}
											<span
												class="px-1.5 py-0.5 text-xs bg-purple-500/20 text-purple-400 rounded"
												>External</span
											>
										{:else if offering.is_example}
											<span
												class="px-1.5 py-0.5 text-xs bg-amber-500/20 text-amber-400 rounded"
												>Demo</span
											>
										{/if}
										{#if getSubscriptionBadge(offering)}
											<span
												class="inline-flex items-center gap-1 px-1.5 py-0.5 text-xs bg-purple-500/20 text-purple-400 rounded"
												title="Recurring subscription"
												><Icon name="repeat" size={20} class="text-purple-400" /> {getSubscriptionBadge(offering)}</span
											>
										{/if}
										{#if offering.post_provision_script}
											<span
												class="px-1.5 py-0.5 text-xs bg-blue-500/20 text-blue-400 rounded"
												>Recipe</span
											>
										{/if}
									</div>
									<div class="flex items-center gap-1 text-xs text-neutral-400">
										<Icon name={getTypeIcon(offering.product_type)} size={20} />
										{offering.product_type}
									</div>
									<a
										href="/dashboard/providers/{offering.owner_username ||
											offering.pubkey}"
										onclick={(e) => e.stopPropagation()}
										class="text-xs text-neutral-500 hover:text-primary-400 {offering.owner_username
											? ''
											: 'font-mono'}"
										>{offering.owner_username
											? `@${offering.owner_username}`
											: truncatePubkey(
													offering.pubkey,
												)}</a
									>
								</div>
								{#if offering.trust_score !== undefined}
									<TrustBadge
										score={offering.trust_score}
										hasFlags={offering.has_critical_flags ??
											false}
										compact={true}
									/>
								{/if}
							</div>
							<div class="text-sm text-neutral-400 mb-2">
								{formatSpecs(offering)}
							</div>
							<div class="flex items-center justify-between">
								<div>
									<div class="text-white font-medium">
										{formatPrice(offering)}
									</div>
									{#if formatUsdEquivalent(offering)}
										<div class="text-xs text-neutral-500">{formatUsdEquivalent(offering)}</div>
									{/if}
									<div class="text-xs text-neutral-500">
										{formatLocation(offering)}
									</div>
								</div>
								{#if hasReseller(offering)}
									<button
										onclick={(e) =>
											handleRentClick(e, offering)}
										class="px-3 py-1.5 bg-primary-600 hover:bg-primary-500 rounded text-xs font-medium"
										>Rent</button
									>
								{:else if offering.offering_source === "seeded" && offering.external_checkout_url}
									<a
										href={offering.external_checkout_url}
										target="_blank"
										rel="noopener noreferrer"
										onclick={(e) => e.stopPropagation()}
										class="inline-flex items-center gap-1 px-3 py-1.5 bg-primary-600 hover:bg-primary-500 rounded text-xs font-medium"
										>Visit Provider <Icon name="external" size={20} class="text-white" /></a
									>
								{:else if offering.is_example}
									<span title="Demo only — not available for rent" class="px-3 py-1.5 bg-neutral-700 text-neutral-500 rounded text-xs font-medium cursor-not-allowed">Demo only</span>
								{:else}
									<button
										onclick={(e) =>
											handleRentClick(e, offering)}
										class="px-3 py-1.5 bg-primary-600 hover:bg-primary-500 rounded text-xs font-medium"
										>Rent</button
									>
								{/if}
							</div>
							{#if expandedRow === offering.id}
								<div
									class="mt-3 pt-3 border-t border-neutral-800 text-sm space-y-2"
								>
									<div class="text-neutral-400">
										{offering.description ||
											"No description"}
									</div>
									{#if offering.post_provision_script}
										<details class="mt-1">
											<summary class="text-xs text-blue-400 cursor-pointer hover:text-blue-300">View recipe script</summary>
											<pre class="mt-2 p-2 bg-base/50 border border-neutral-800 text-xs text-neutral-300 font-mono overflow-x-auto max-h-36 overflow-y-auto whitespace-pre-wrap">{offering.post_provision_script}</pre>
										</details>
									{/if}
									<div class="grid grid-cols-2 gap-2 text-xs">
										{#if offering.processor_name || offering.processor_brand}
											<div>
												<span class="text-neutral-500"
													>CPU:</span
												>
												<span class="text-neutral-400"
													>{offering.processor_name ||
														offering.processor_brand}</span
												>
											</div>
										{/if}
										{#if offering.memory_amount}
											<div>
												<span class="text-neutral-500"
													>Memory:</span
												>
												<span class="text-neutral-400"
													>{offering.memory_amount}</span
												>
											</div>
										{/if}
										{#if offering.total_ssd_capacity || offering.total_hdd_capacity}
											<div>
												<span class="text-neutral-500"
													>Storage:</span
												>
												<span class="text-neutral-400"
													>{offering.total_ssd_capacity ||
														offering.total_hdd_capacity}</span
												>
											</div>
										{/if}
										{#if offering.virtualization_type}
											<div>
												<span class="text-neutral-500"
													>Platform:</span
												>
												<span class="text-neutral-400"
													>{offering.virtualization_type}</span
												>
											</div>
										{/if}
										<div>
											<span class="text-neutral-500"
												>Billing:</span
											>
											<span class="text-neutral-400"
												>{formatBilling(offering)}</span
											>
										</div>
										{#if offering.setup_fee > 0}
											<div>
												<span class="text-neutral-500"
													>Setup:</span
												>
												<span class="text-neutral-400"
													>{offering.setup_fee.toFixed(
														2,
													)}
													{offering.currency}</span
												>
											</div>
										{/if}
										{#if offering.min_contract_hours || offering.max_contract_hours}
											<div>
												<span class="text-neutral-500"
													>Contract:</span
												>
												<span class="text-neutral-400"
													>{formatContractTerms(
														offering,
													)}</span
												>
											</div>
										{/if}
										{#if offering.unmetered_bandwidth}
											<div>
												<span class="text-neutral-500"
													>Bandwidth:</span
												>
												<span class="text-neutral-400"
													>Unmetered</span
												>
											</div>
										{/if}
									</div>
									<div class="mt-2 flex flex-col gap-2">
										<button
											onclick={(e) => copyOfferingLink(offering.id, e)}
											class="inline-flex items-center gap-1.5 text-xs text-neutral-500 hover:text-primary-400 transition-colors"
										>
											{#if copyLinkFeedback === offering.id}
												<Icon name="check" size={14} class="text-green-400" /> Copied!
											{:else}
												<Icon name="link" size={14} /> Copy link
											{/if}
										</button>
										<a
											href="/dashboard/providers/{offering.owner_username || offering.pubkey}"
											onclick={(e) => e.stopPropagation()}
											class="inline-flex items-center gap-1.5 text-xs text-neutral-500 hover:text-primary-400 transition-colors"
										>
											<Icon name="user" size={14} /> View provider profile
										</a>
									</div>
								</div>
							{/if}
						</div>
					{/each}
				</div>
			{/if}
		</div>
	</div>
</div>

<RentalRequestDialog
	offering={selectedOffering}
	onClose={() => (selectedOffering = null)}
	onSuccess={handleRentalSuccess}
/>

<AuthPromptModal
	isOpen={showAuthModal}
	onClose={() => (showAuthModal = false)}
	message="Create an account or login to rent cloud resources"
/>

<!-- Compare max warning toast -->
{#if compareWarning}
	<div class="fixed bottom-24 left-1/2 -translate-x-1/2 z-50 px-4 py-2 bg-neutral-900 border border-amber-500/50 text-amber-400 text-sm rounded shadow-lg">
		{compareWarning}
	</div>
{/if}

<!-- Sticky compare bar -->
{#if compareIds.size > 0}
	<div class="fixed bottom-0 inset-x-0 z-40 bg-surface-elevated border-t border-neutral-700 shadow-2xl">
		<div class="max-w-screen-xl mx-auto px-4 py-3 flex items-center justify-between gap-4">
			<span class="text-sm text-neutral-300">
				Comparing <span class="font-semibold text-white">{compareIds.size}/3</span> offerings
			</span>
			<div class="flex items-center gap-2">
				<button
					onclick={() => {
						compareIds = new Set();
					}}
					class="px-3 py-1.5 text-xs border border-neutral-600 text-neutral-400 hover:text-white hover:border-neutral-500 rounded transition-colors"
				>Clear</button>
				<button
					onclick={() => {
						showCompareModal = true;
					}}
					disabled={compareIds.size < 2}
					class="px-4 py-1.5 text-xs font-medium rounded transition-colors {compareIds.size >= 2
						? 'bg-primary-600 hover:bg-primary-500 text-white'
						: 'bg-neutral-700 text-neutral-500 cursor-not-allowed'}"
				>Compare</button>
			</div>
		</div>
	</div>
{/if}

<!-- Comparison modal -->
{#if showCompareModal}
	<div
		class="fixed inset-0 bg-base/80 backdrop-blur-sm z-50 flex items-start justify-center p-4 overflow-y-auto"
		onclick={(e) => e.target === e.currentTarget && (showCompareModal = false)}
		role="button"
		tabindex="0"
		onkeydown={(e) => e.key === 'Escape' && (showCompareModal = false)}
	>
		<div class="bg-surface-elevated border border-neutral-800 shadow-2xl w-full max-w-5xl mt-8 mb-8">
			<!-- Header -->
			<div class="flex items-center justify-between px-6 py-4 border-b border-neutral-800">
				<h2 class="text-lg font-semibold text-white">Compare Offerings</h2>
				<button
					onclick={() => (showCompareModal = false)}
					class="text-neutral-500 hover:text-white transition-colors"
					aria-label="Close comparison"
				>
					<Icon name="x" size={20} />
				</button>
			</div>

			<!-- Comparison table -->
			<div class="overflow-x-auto">
				<table class="w-full text-sm">
					<thead>
						<tr class="border-b border-neutral-800">
							<th class="px-4 py-3 text-left text-neutral-500 font-medium w-32">Spec</th>
							{#each compareOfferings as offering}
								<th class="px-4 py-3 text-left">
									<a
										href="/dashboard/marketplace/{offering.id}"
										class="font-semibold text-white hover:text-primary-400 transition-colors"
									>{offering.offer_name}</a>
									<div class="text-xs text-neutral-500 font-normal mt-0.5">
										{offering.owner_username
											? `@${offering.owner_username}`
											: truncatePubkey(offering.pubkey)}
									</div>
								</th>
							{/each}
						</tr>
					</thead>
					<tbody>
						<!-- Price -->
						<tr class="border-b border-neutral-800/50 hover:bg-neutral-800/20">
							<td class="px-4 py-3 text-neutral-500">Price</td>
							{#each compareOfferings as offering}
								<td class="px-4 py-3">
									<span class="text-white font-medium">{formatPrice(offering)}</span>
									{#if formatUsdEquivalent(offering)}
										<span class="text-xs text-neutral-500 ml-1">{formatUsdEquivalent(offering)}</span>
									{/if}
								</td>
							{/each}
						</tr>
						<!-- CPU -->
						<tr class="border-b border-neutral-800/50 hover:bg-neutral-800/20">
							<td class="px-4 py-3 text-neutral-500">CPU Cores</td>
							{#each compareOfferings as offering}
								<td class="px-4 py-3 text-neutral-300">{offering.processor_cores ?? '—'}</td>
							{/each}
						</tr>
						<!-- RAM -->
						<tr class="border-b border-neutral-800/50 hover:bg-neutral-800/20">
							<td class="px-4 py-3 text-neutral-500">RAM</td>
							{#each compareOfferings as offering}
								<td class="px-4 py-3 text-neutral-300">{offering.memory_amount ?? '—'}</td>
							{/each}
						</tr>
						<!-- SSD -->
						<tr class="border-b border-neutral-800/50 hover:bg-neutral-800/20">
							<td class="px-4 py-3 text-neutral-500">SSD</td>
							{#each compareOfferings as offering}
								<td class="px-4 py-3 text-neutral-300">
									{offering.total_ssd_capacity ?? offering.total_hdd_capacity ?? '—'}
								</td>
							{/each}
						</tr>
						<!-- Location -->
						<tr class="border-b border-neutral-800/50 hover:bg-neutral-800/20">
							<td class="px-4 py-3 text-neutral-500">Location</td>
							{#each compareOfferings as offering}
								<td class="px-4 py-3 text-neutral-300">{formatLocation(offering)}</td>
							{/each}
						</tr>
						<!-- Virtualization -->
						<tr class="border-b border-neutral-800/50 hover:bg-neutral-800/20">
							<td class="px-4 py-3 text-neutral-500">Virtualization</td>
							{#each compareOfferings as offering}
								<td class="px-4 py-3 text-neutral-300">{offering.virtualization_type ?? '—'}</td>
							{/each}
						</tr>
						<!-- Bandwidth -->
						<tr class="border-b border-neutral-800/50 hover:bg-neutral-800/20">
							<td class="px-4 py-3 text-neutral-500">Bandwidth</td>
							{#each compareOfferings as offering}
								<td class="px-4 py-3 text-neutral-300">
									{offering.unmetered_bandwidth ? 'Unmetered' : (offering.uplink_speed ?? '—')}
								</td>
							{/each}
						</tr>
						<!-- Contract term -->
						<tr class="border-b border-neutral-800/50 hover:bg-neutral-800/20">
							<td class="px-4 py-3 text-neutral-500">Contract</td>
							{#each compareOfferings as offering}
								<td class="px-4 py-3 text-neutral-300">{formatContractTerms(offering)}</td>
							{/each}
						</tr>
						<!-- Trust score -->
						<tr class="border-b border-neutral-800/50 hover:bg-neutral-800/20">
							<td class="px-4 py-3 text-neutral-500">Trust Score</td>
							{#each compareOfferings as offering}
								<td class="px-4 py-3">
									{#if offering.trust_score !== undefined}
										<TrustBadge
											score={offering.trust_score}
											hasFlags={offering.has_critical_flags ?? false}
											compact={true}
										/>
									{:else}
										<span class="text-neutral-500">—</span>
									{/if}
								</td>
							{/each}
						</tr>
						<!-- Provider -->
						<tr class="hover:bg-neutral-800/20">
							<td class="px-4 py-3 text-neutral-500">Provider</td>
							{#each compareOfferings as offering}
								<td class="px-4 py-3">
									<a
										href="/dashboard/providers/{offering.owner_username || offering.pubkey}"
										class="text-primary-400 hover:text-primary-300 text-xs transition-colors {offering.owner_username ? '' : 'font-mono'}"
									>{offering.owner_username ? `@${offering.owner_username}` : truncatePubkey(offering.pubkey)}</a>
								</td>
							{/each}
						</tr>
					</tbody>
				</table>
			</div>

			<!-- Action row -->
			<div class="px-6 py-4 border-t border-neutral-800 flex gap-3">
				{#each compareOfferings as offering}
					<div class="flex-1">
						{#if offering.offering_source === 'seeded' && offering.external_checkout_url}
							<a
								href={offering.external_checkout_url}
								target="_blank"
								rel="noopener noreferrer"
								class="inline-flex items-center gap-1 w-full justify-center px-3 py-2 bg-primary-600 hover:bg-primary-500 text-white text-xs font-medium rounded transition-colors"
							>Visit Provider <Icon name="external" size={14} class="text-white" /></a>
						{:else if offering.is_example}
							<span class="inline-flex w-full justify-center px-3 py-2 bg-neutral-700 text-neutral-500 text-xs font-medium rounded cursor-not-allowed">Demo only</span>
						{:else}
							<button
								onclick={() => {
									showCompareModal = false;
									handleRentClick(new MouseEvent('click'), offering);
								}}
								class="w-full px-3 py-2 bg-primary-600 hover:bg-primary-500 text-white text-xs font-medium rounded transition-colors"
							>Rent {offering.offer_name}</button>
						{/if}
					</div>
				{/each}
			</div>
		</div>
	</div>
{/if}
