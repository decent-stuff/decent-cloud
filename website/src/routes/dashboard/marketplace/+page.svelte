<script lang="ts">
	import { onMount, tick } from "svelte";
	import { page } from "$app/stores";
	import { goto } from "$app/navigation";
	import { searchOfferings, fetchIcpPrice, getSavedOfferingIds, saveOffering, unsaveOffering, hexEncode, fetchTrendingOfferings, fetchNewProviders, type Offering, type TrendingOffering, type NewProvider } from "$lib/services/api";
	import { toggleSavedId } from "$lib/services/saved-offerings";
	import RentalRequestDialog from "$lib/components/RentalRequestDialog.svelte";
	import AuthPromptModal from "$lib/components/AuthPromptModal.svelte";
	import TrustBadge from "$lib/components/TrustBadge.svelte";
	import Icon, { type IconName } from "$lib/components/Icons.svelte";
	import { authStore } from "$lib/stores/auth";
	import { signRequest } from "$lib/services/auth-api";
	import { Ed25519KeyIdentity } from '@dfinity/identity';
	import { truncatePubkey } from "$lib/utils/identity";
	import { addToComparison, removeFromComparison, COMPARE_MAX_ERROR } from "$lib/utils/compare";
	import { getRecentlyViewed } from "$lib/utils/recently-viewed";

	let offerings = $state<Offering[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let icpPriceUsd = $state<number | null>(null);
	let searchQuery = $state("");
	let selectedOffering = $state<Offering | null>(null);
	let successMessage = $state<string | null>(null);
	let isAuthenticated = $state(false);
	let savedIds = $state(new Set<number>());
	const PROVIDER_CTA_KEY = 'dc-provider-cta-dismissed';
	const FIRST_TIME_HINT_KEY = 'dc-marketplace-hint-visits';
	const ADVANCED_FILTERS_KEY = 'dc-marketplace-advanced-filters';
	let providerCtaDismissed = $state(false);
	let showFirstTimeHint = $state(false);
	let showAdvancedFilters = $state(false);
	let showAuthModal = $state(false);
	let expandedRow = $state<number | null>(null);
	let sortDir = $state<"asc" | "desc">("asc");
	let sortField = $state<"price" | "trust" | "newest">("price");
	let quickFilter = $state<"newest" | "trusted" | null>(null);
	let selectedPreset = $state<"gpu" | "budget" | "na" | "europe" | null>(null);
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
	let inStockOnly = $state(true);
	let providerFilter = $state<string>('');
	let recentlyViewedIds = $state<number[]>([]);
	let trendingOfferings = $state<TrendingOffering[]>([]);
	let newProviders = $state<NewProvider[]>([]);

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

	// 7-day cutoff in nanoseconds for "Recently Added" filter
	const RECENT_CUTOFF_NS = $derived(quickFilter === "newest"
		? (Date.now() - 7 * 24 * 60 * 60 * 1000) * 1_000_000
		: null);

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

		// Recently Added: filter to last 7 days
		if (RECENT_CUTOFF_NS !== null) {
			result = result.filter((o) => (o.created_at_ns ?? 0) >= RECENT_CUTOFF_NS!);
		}

		// Provider filter (from ?provider= URL param or "View all from provider" link)
		if (providerFilter) {
			result = result.filter(o => o.pubkey === providerFilter);
		}

		// Sort by selected field
		result.sort((a, b) => {
			if (sortField === "trust") {
				const ta = a.trust_score ?? -1;
				const tb = b.trust_score ?? -1;
				return tb - ta; // descending always (highest trust first)
			}
			if (sortField === "newest") {
				const ta = a.created_at_ns ?? 0;
				const tb = b.created_at_ns ?? 0;
				return tb - ta; // descending: newest first
			}
			const priceA = a.monthly_price ?? Infinity;
			const priceB = b.monthly_price ?? Infinity;
			return sortDir === "asc" ? priceA - priceB : priceB - priceA;
		});

		return result;
	});

	let recentlyViewedOfferings = $derived(
		recentlyViewedIds
			.map(id => offerings.find(o => o.id === id))
			.filter((o): o is Offering => o !== undefined)
			.slice(0, 5)
	);

	authStore.isAuthenticated.subscribe((value) => {
		isAuthenticated = value;
	});

	async function loadSavedIds() {
		const info = await authStore.getSigningIdentity();
		if (!info || !(info.identity instanceof Ed25519KeyIdentity)) return;
		const pubkeyHex = hexEncode(info.publicKeyBytes);
		const { headers } = await signRequest(info.identity, 'GET', `/api/v1/users/${pubkeyHex}/saved-offering-ids`);
		const ids = await getSavedOfferingIds(headers, pubkeyHex);
		savedIds = new Set(ids);
	}

	async function toggleBookmark(e: Event, offeringId: number) {
		e.stopPropagation();
		if (!isAuthenticated) {
			showAuthModal = true;
			return;
		}
		const info = await authStore.getSigningIdentity();
		if (!info || !(info.identity instanceof Ed25519KeyIdentity)) return;
		const pubkeyHex = hexEncode(info.publicKeyBytes);
		const isSaved = savedIds.has(offeringId);
		savedIds = toggleSavedId(savedIds, offeringId);
		try {
			if (isSaved) {
				const { headers } = await signRequest(info.identity, 'DELETE', `/api/v1/users/${pubkeyHex}/saved-offerings/${offeringId}`);
				await unsaveOffering(headers, pubkeyHex, offeringId);
			} else {
				const { headers } = await signRequest(info.identity, 'POST', `/api/v1/users/${pubkeyHex}/saved-offerings/${offeringId}`);
				await saveOffering(headers, pubkeyHex, offeringId);
			}
		} catch (err) {
			// Revert optimistic update on error
			savedIds = toggleSavedId(savedIds, offeringId);
			console.error('Failed to toggle bookmark:', err);
		}
	}

	async function fetchOfferings() {
		try {
			loading = true;
			error = null;
			offerings = await searchOfferings({
				limit: 100,
				in_stock_only: inStockOnly || undefined,
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

	function readFiltersFromUrl(url: URL) {
		const p = url.searchParams;
		searchQuery = p.get('q') ?? '';
		const typesStr = p.get('types');
		selectedTypes = typesStr ? new Set(typesStr.split(',').filter(Boolean)) : new Set();
		minPrice = p.has('minPrice') ? Number(p.get('minPrice')) : null;
		maxPrice = p.has('maxPrice') ? Number(p.get('maxPrice')) : null;
		selectedRegion = p.get('region') ?? '';
		selectedCountry = p.get('country') ?? '';
		selectedCity = p.get('city') ?? '';
		minCores = p.has('minCores') ? Number(p.get('minCores')) : null;
		minMemoryGb = p.has('minMemoryGb') ? Number(p.get('minMemoryGb')) : null;
		minSsdGb = p.has('minSsdGb') ? Number(p.get('minSsdGb')) : null;
		selectedVirt = p.get('virt') ?? '';
		unmeteredOnly = p.get('unmetered') === '1';
		minTrust = p.has('minTrust') ? Number(p.get('minTrust')) : null;
		showDemoOfferings = p.get('demo') === '1';
		showOfflineOfferings = p.get('offline') === '1';
		recipesOnly = p.get('recipes') === '1';
		inStockOnly = p.get('inStock') !== '0';
		sortField = (p.get('sort') as 'price' | 'trust' | 'newest') ?? 'price';
		sortDir = (p.get('dir') as 'asc' | 'desc') ?? 'asc';
		quickFilter = (p.get('quick') as 'newest' | 'trusted' | null) ?? null;
		selectedPreset = (p.get('preset') as 'gpu' | 'budget' | 'na' | 'europe' | null) ?? null;
		providerFilter = p.get('provider') ?? '';
	}

	function syncFiltersToUrl() {
		const params = new URLSearchParams();
		if (searchQuery) params.set('q', searchQuery);
		if (selectedTypes.size > 0) params.set('types', [...selectedTypes].join(','));
		if (minPrice != null) params.set('minPrice', String(minPrice));
		if (maxPrice != null) params.set('maxPrice', String(maxPrice));
		if (selectedRegion) params.set('region', selectedRegion);
		if (selectedCountry) params.set('country', selectedCountry);
		if (selectedCity) params.set('city', selectedCity);
		if (minCores != null) params.set('minCores', String(minCores));
		if (minMemoryGb != null) params.set('minMemoryGb', String(minMemoryGb));
		if (minSsdGb != null) params.set('minSsdGb', String(minSsdGb));
		if (selectedVirt) params.set('virt', selectedVirt);
		if (unmeteredOnly) params.set('unmetered', '1');
		if (minTrust != null) params.set('minTrust', String(minTrust));
		if (showDemoOfferings) params.set('demo', '1');
		if (showOfflineOfferings) params.set('offline', '1');
		if (recipesOnly) params.set('recipes', '1');
		if (!inStockOnly) params.set('inStock', '0');
		if (sortField !== 'price') params.set('sort', sortField);
		if (sortDir !== 'asc') params.set('dir', sortDir);
		if (quickFilter) params.set('quick', quickFilter);
		if (selectedPreset) params.set('preset', selectedPreset);
		if (providerFilter) params.set('provider', providerFilter);
		// Preserve expanded offering deep-link if present
		if (expandedRow !== null) params.set('offering', String(expandedRow));
		const url = params.toString() ? `?${params.toString()}` : '/dashboard/marketplace';
		goto(url, { replaceState: true, keepFocus: true, noScroll: true });
	}

	onMount(async () => {
		recentlyViewedIds = getRecentlyViewed();
		providerCtaDismissed = localStorage.getItem(PROVIDER_CTA_KEY) === '1';
		showAdvancedFilters = localStorage.getItem(ADVANCED_FILTERS_KEY) === '1';
		const hintVisits = parseInt(localStorage.getItem(FIRST_TIME_HINT_KEY) ?? '0', 10);
		if (hintVisits < 3) {
			showFirstTimeHint = true;
			localStorage.setItem(FIRST_TIME_HINT_KEY, String(hintVisits + 1));
		}
		readFiltersFromUrl($page.url);
		const fetches: Promise<unknown>[] = [fetchOfferings(), fetchIcpPrice()];
		if (isAuthenticated) fetches.push(loadSavedIds().catch((err) => console.error('Failed to load saved offerings:', err)));
		[, icpPriceUsd] = await Promise.all(fetches) as [unknown, number | null];
		// Load trending independently — failure must not block the main marketplace
		fetchTrendingOfferings(6).then(t => { trendingOfferings = t; }).catch(err => console.error('Failed to load trending offerings:', err));
		fetchNewProviders(6).then(p => { newProviders = p; }).catch(err => console.error('Failed to load new providers:', err));
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
		debounceTimer = setTimeout(() => { fetchOfferings(); syncFiltersToUrl(); }, 300);
	}

	function handleFilterChange() {
		selectedPreset = null;
		fetchOfferings();
		syncFiltersToUrl();
	}

	function toggleType(type: string) {
		const newSet = new Set(selectedTypes);
		if (newSet.has(type)) newSet.delete(type);
		else newSet.add(type);
		selectedTypes = newSet;
		selectedPreset = null;
		syncFiltersToUrl();
	}

	function setPreset(preset: "gpu" | "budget" | "na" | "europe") {
		if (selectedPreset === preset) {
			// Toggle off: clear only the filters this preset set
			selectedPreset = null;
			if (preset === "gpu") selectedTypes = new Set();
			else if (preset === "budget") { maxPrice = null; fetchOfferings(); }
			else if (preset === "na" || preset === "europe") { selectedRegion = ""; selectedCountry = ""; selectedCity = ""; }
			syncFiltersToUrl();
			return;
		}
		// Clear all preset-controlled filters first, then apply
		selectedTypes = new Set();
		maxPrice = null;
		selectedRegion = "";
		selectedCountry = "";
		selectedCity = "";
		if (preset === "gpu") selectedTypes = new Set(["gpu"]);
		else if (preset === "budget") { maxPrice = 20; fetchOfferings(); }
		else if (preset === "na") selectedRegion = "na";
		else if (preset === "europe") selectedRegion = "europe";
		selectedPreset = preset;
		syncFiltersToUrl();
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
		inStockOnly = true;
		searchQuery = "";
		quickFilter = null;
		selectedPreset = null;
		sortField = "price";
		sortDir = "asc";
		providerFilter = '';
		fetchOfferings();
		syncFiltersToUrl();
	}

	function toggleAdvancedFilters() {
		showAdvancedFilters = !showAdvancedFilters;
		localStorage.setItem(ADVANCED_FILTERS_KEY, showAdvancedFilters ? '1' : '0');
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
	let compareWarning = $state<string | null>(null);


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
		syncFiltersToUrl();
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
		quickFilter = null;
		syncFiltersToUrl();
	}

	function setSortTrust() {
		sortField = "trust";
		quickFilter = null;
		syncFiltersToUrl();
	}

	function toggleQuickFilter(filter: "newest" | "trusted") {
		if (quickFilter === filter) {
			quickFilter = null;
			sortField = "price";
			sortDir = "asc";
		} else {
			quickFilter = filter;
			sortField = filter === "newest" ? "newest" : "trust";
		}
		syncFiltersToUrl();
	}

	function handleRentalSuccess(contractId: string) {
		selectedOffering = null;
		// Navigate to contract detail page with welcome state
		goto(`/dashboard/rentals/${contractId}?welcome=true`);
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

	let hasActiveFilters = $derived(
		selectedTypes.size > 0 || searchQuery !== '' || selectedRegion !== '' || selectedCountry !== '' ||
		selectedCity !== '' || minPrice !== null || maxPrice !== null || minCores !== null ||
		minMemoryGb !== null || minSsdGb !== null || selectedVirt !== '' || unmeteredOnly ||
		minTrust !== null || showDemoOfferings || showOfflineOfferings || recipesOnly || !inStockOnly ||
		quickFilter !== null || selectedPreset !== null
	);

	let activeFilterChips = $derived.by(() => {
		const chips: Array<{ label: string; remove: () => void }> = [];

		for (const t of selectedTypes) {
			const opt = typeOptions.find((o) => o.key === t);
			const label = opt ? opt.label : t;
			chips.push({ label, remove: () => { const s = new Set(selectedTypes); s.delete(t); selectedTypes = s; syncFiltersToUrl(); } });
		}
		if (searchQuery) {
			chips.push({ label: `Search: ${searchQuery}`, remove: () => { searchQuery = ""; fetchOfferings(); syncFiltersToUrl(); } });
		}
		if (selectedRegion) {
			const region = REGIONS.find((r) => r.code === selectedRegion);
			chips.push({ label: `Region: ${region?.name ?? selectedRegion}`, remove: () => { selectedRegion = ""; selectedCountry = ""; selectedCity = ""; syncFiltersToUrl(); } });
		}
		if (selectedCountry) {
			chips.push({ label: `Country: ${selectedCountry}`, remove: () => { selectedCountry = ""; selectedCity = ""; handleFilterChange(); } });
		}
		if (selectedCity) {
			chips.push({ label: `City: ${selectedCity}`, remove: () => { selectedCity = ""; syncFiltersToUrl(); } });
		}
		if (minPrice !== null) {
			chips.push({ label: `Min price: ${minPrice} ICP`, remove: () => { minPrice = null; handleFilterChange(); } });
		}
		if (maxPrice !== null) {
			chips.push({ label: `Max price: ${maxPrice} ICP`, remove: () => { maxPrice = null; handleFilterChange(); } });
		}
		if (minCores !== null) {
			chips.push({ label: `Min cores: ${minCores}`, remove: () => { minCores = null; syncFiltersToUrl(); } });
		}
		if (minMemoryGb !== null) {
			chips.push({ label: `Min RAM: ${minMemoryGb}GB`, remove: () => { minMemoryGb = null; syncFiltersToUrl(); } });
		}
		if (minSsdGb !== null) {
			chips.push({ label: `Min SSD: ${minSsdGb}GB`, remove: () => { minSsdGb = null; syncFiltersToUrl(); } });
		}
		if (selectedVirt) {
			chips.push({ label: `Virt: ${selectedVirt.toUpperCase()}`, remove: () => { selectedVirt = ""; syncFiltersToUrl(); } });
		}
		if (unmeteredOnly) {
			chips.push({ label: "Unmetered", remove: () => { unmeteredOnly = false; syncFiltersToUrl(); } });
		}
		if (minTrust !== null) {
			chips.push({ label: `Trust ≥ ${minTrust}`, remove: () => { minTrust = null; syncFiltersToUrl(); } });
		}
		if (showDemoOfferings) {
			chips.push({ label: "Showing demos", remove: () => { showDemoOfferings = false; syncFiltersToUrl(); } });
		}
		if (showOfflineOfferings) {
			chips.push({ label: "Showing offline", remove: () => { showOfflineOfferings = false; syncFiltersToUrl(); } });
		}
		if (recipesOnly) {
			chips.push({ label: "Recipes only", remove: () => { recipesOnly = false; handleFilterChange(); } });
		}
		if (!inStockOnly) {
			chips.push({ label: "Including out-of-stock", remove: () => { inStockOnly = true; handleFilterChange(); } });
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
								selectedPreset = null;
								syncFiltersToUrl();
							}}
							class="w-full px-2 py-1.5 text-sm input focus:outline-none focus:border-primary-400"
						>
							<option value="">All regions</option>
							{#each REGIONS as region}
								<option value={region.code}>{region.name}</option>
							{/each}
						</select>
					</div>

					<!-- More Filters Toggle -->
					<button
						onclick={toggleAdvancedFilters}
						class="flex items-center gap-1.5 text-xs text-primary-400 hover:text-primary-300 transition-colors w-full"
					>
						<Icon name={showAdvancedFilters ? "chevron-up" : "chevron-down"} size={14} />
						{showAdvancedFilters ? 'Fewer filters' : 'More filters'}
					</button>

					<!-- Advanced Filters (collapsible) -->
					<div
						class="space-y-4 overflow-hidden transition-all duration-200 {showAdvancedFilters
							? 'max-h-[2000px] opacity-100'
							: 'max-h-0 opacity-0'}"
					>
						<!-- Country Filter -->
						<div>
							<div class="data-label mb-2">Country</div>
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
							<div class="data-label mb-2">City</div>
							<select
								bind:value={selectedCity}
								onchange={() => syncFiltersToUrl()}
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
							<div class="data-label mb-2">Min CPU Cores</div>
							<input
								type="number"
								placeholder="e.g., 4"
								bind:value={minCores}
								min="1"
								onchange={() => syncFiltersToUrl()}
								class="w-full px-2 py-1.5 text-sm input focus:outline-none focus:border-primary-400"
							/>
						</div>

						<!-- Memory Filter -->
						<div>
							<div class="data-label mb-2">Min Memory (GB)</div>
							<input
								type="number"
								placeholder="e.g., 8"
								bind:value={minMemoryGb}
								min="1"
								onchange={() => syncFiltersToUrl()}
								class="w-full px-2 py-1.5 text-sm input focus:outline-none focus:border-primary-400"
							/>
						</div>

						<!-- SSD Filter -->
						<div>
							<div class="data-label mb-2">Min SSD (GB)</div>
							<input
								type="number"
								placeholder="e.g., 100"
								bind:value={minSsdGb}
								min="1"
								onchange={() => syncFiltersToUrl()}
								class="w-full px-2 py-1.5 text-sm input focus:outline-none focus:border-primary-400"
							/>
						</div>

						<!-- Virtualization Type Filter -->
						{#if virtTypes.length > 0}
							<div>
								<div class="data-label mb-2">Virtualization</div>
								<select
									bind:value={selectedVirt}
									onchange={() => syncFiltersToUrl()}
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
							<div class="data-label mb-2">Min Trust Score</div>
							<input
								type="number"
								placeholder="0-100"
								bind:value={minTrust}
								min="0"
								max="100"
								onchange={() => syncFiltersToUrl()}
								class="w-full px-2 py-1.5 text-sm input focus:outline-none focus:border-primary-400"
							/>
						</div>

						<!-- Unmetered Bandwidth Filter -->
						<div>
							<label class="flex items-center gap-2 cursor-pointer group">
								<input
									type="checkbox"
									bind:checked={unmeteredOnly}
									onchange={() => syncFiltersToUrl()}
									class="border-neutral-700 bg-base text-primary-500 focus:ring-primary-500"
								/>
								<span class="text-sm text-neutral-400 group-hover:text-white"
									>Unmetered bandwidth only</span
								>
							</label>
						</div>

						<!-- Show Demo Offerings Filter -->
						<div>
							<label class="flex items-center gap-2 cursor-pointer group">
								<input
									type="checkbox"
									bind:checked={showDemoOfferings}
									onchange={() => syncFiltersToUrl()}
									class="border-neutral-700 bg-base text-primary-500 focus:ring-primary-500"
								/>
								<span class="text-sm text-neutral-400 group-hover:text-white"
									>Show demo offerings</span
								>
							</label>
						</div>

						<!-- Show Offline Offerings Filter -->
						<div>
							<label class="flex items-center gap-2 cursor-pointer group">
								<input
									type="checkbox"
									bind:checked={showOfflineOfferings}
									onchange={() => syncFiltersToUrl()}
									class="border-neutral-700 bg-base text-primary-500 focus:ring-primary-500"
								/>
								<span class="text-sm text-neutral-400 group-hover:text-white"
									>Show offline offerings</span
								>
							</label>
						</div>

						<!-- Recipes Only Filter -->
						<div>
							<label class="flex items-center gap-2 cursor-pointer group">
								<input
									type="checkbox"
									bind:checked={recipesOnly}
									onchange={handleFilterChange}
									class="border-neutral-700 bg-base text-primary-500 focus:ring-primary-500"
								/>
								<span class="text-sm text-neutral-400 group-hover:text-white"
									>Recipes only</span
								>
							</label>
						</div>
					</div>
				</div>
			</div>
		</aside>

		<!-- Main Content -->
		<div class="flex-1 min-w-0 space-y-4">
			<!-- Quick-filter preset pills with first-time guidance -->
			<div class="space-y-2">
			{#if showFirstTimeHint && !hasActiveFilters}
				<p class="text-xs text-neutral-500">Not sure where to start? Pick what you need:</p>
			{:else}
				<p class="text-xs text-neutral-600">I'm looking for...</p>
			{/if}
			<div class="flex flex-wrap items-center gap-2">
				<button
					onclick={() => toggleQuickFilter("newest")}
					class="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-full border transition-colors {quickFilter === 'newest' ? 'bg-primary-500/20 text-primary-300 border-primary-500/50' : 'bg-neutral-800/60 text-neutral-400 border-neutral-700 hover:border-neutral-500 hover:text-white'}"
				>
					<Icon name="clock" size={14} /> Recently Added
				</button>
				<button
					onclick={() => toggleQuickFilter("trusted")}
					class="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-full border transition-colors {quickFilter === 'trusted' ? 'bg-amber-500/20 text-amber-300 border-amber-500/50' : 'bg-neutral-800/60 text-neutral-400 border-neutral-700 hover:border-neutral-500 hover:text-white'}"
				>
					<Icon name="shield" size={14} /> Most Trusted
				</button>
				<span class="text-neutral-700 text-xs select-none">|</span>
				<button
					onclick={() => setPreset("gpu")}
					class="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-full border transition-colors {selectedPreset === 'gpu' ? 'bg-purple-500/20 text-purple-300 border-purple-500/50' : 'bg-neutral-800/60 text-neutral-400 border-neutral-700 hover:border-neutral-500 hover:text-white'}"
				>
					<Icon name="gpu" size={14} /> GPU Servers
				</button>
				<button
					onclick={() => setPreset("budget")}
					class="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-full border transition-colors {selectedPreset === 'budget' ? 'bg-emerald-500/20 text-emerald-300 border-emerald-500/50' : 'bg-neutral-800/60 text-neutral-400 border-neutral-700 hover:border-neutral-500 hover:text-white'}"
				>
					Budget (&lt;$20/mo)
				</button>
				<button
					onclick={() => setPreset("na")}
					class="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-full border transition-colors {selectedPreset === 'na' ? 'bg-sky-500/20 text-sky-300 border-sky-500/50' : 'bg-neutral-800/60 text-neutral-400 border-neutral-700 hover:border-neutral-500 hover:text-white'}"
				>
					North America
				</button>
				<button
					onclick={() => setPreset("europe")}
					class="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-full border transition-colors {selectedPreset === 'europe' ? 'bg-sky-500/20 text-sky-300 border-sky-500/50' : 'bg-neutral-800/60 text-neutral-400 border-neutral-700 hover:border-neutral-500 hover:text-white'}"
				>
					Europe
				</button>
			</div>
			</div>

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

			{#if providerFilter}
				<div class="bg-primary-500/10 border border-primary-500/30 p-3 flex items-center justify-between text-sm">
					<span class="text-primary-300">Showing offerings from one provider</span>
					<button onclick={() => { providerFilter = ''; syncFiltersToUrl(); }} class="text-neutral-400 hover:text-white text-xs">Clear filter ×</button>
				</div>
			{/if}

			{#if recentlyViewedOfferings.length > 0}
				<div class="mb-4">
					<div class="text-xs text-neutral-500 mb-2 uppercase tracking-wide">Recently Viewed</div>
					<div class="flex flex-wrap gap-2">
						{#each recentlyViewedOfferings as o}
							<a href="/dashboard/marketplace/{o.id}" class="flex items-center gap-1.5 px-3 py-1.5 bg-surface-elevated border border-neutral-800 hover:border-neutral-600 text-sm text-neutral-300 hover:text-white transition-colors">
								{o.offer_name}
								{#if o.monthly_price}
									<span class="text-neutral-500 text-xs">{o.monthly_price.toFixed(2)} {o.currency}</span>
								{/if}
							</a>
						{/each}
					</div>
				</div>
			{/if}

			{#if trendingOfferings.length >= 2 && !hasActiveFilters}
				<div class="mb-6">
					<div class="flex items-center justify-between mb-2">
						<div class="text-xs text-neutral-500 uppercase tracking-wide">Trending this week</div>
						<a href="/dashboard/marketplace" class="text-xs text-primary-400 hover:text-primary-300 transition-colors">See all</a>
					</div>
					<div class="flex gap-3 overflow-x-auto pb-1">
						{#each trendingOfferings as t}
							<a
								href="/dashboard/marketplace/{t.offering_id}"
								class="flex-none w-44 p-3 bg-surface-elevated border border-neutral-800 hover:border-neutral-600 transition-colors"
							>
								<div class="font-medium text-white text-sm truncate mb-1">{t.offer_name}</div>
								<div class="text-xs text-neutral-400 mb-2">{t.product_type}</div>
								<div class="text-xs text-neutral-300 mb-1">{t.monthly_price.toFixed(2)} {t.currency}/mo</div>
								{#if t.datacenter_city || t.datacenter_country}
									<div class="text-xs text-neutral-500 truncate mb-2">{[t.datacenter_city, t.datacenter_country].filter(Boolean).join(', ')}</div>
								{/if}
								<div class="text-xs text-orange-400">&#x1F525; {t.views_7d} views this week</div>
							</a>
						{/each}
					</div>
				</div>
			{/if}

			{#if newProviders.length >= 2 && !hasActiveFilters}
				<div class="mb-6">
					<div class="flex items-center justify-between mb-2">
						<div class="text-xs text-neutral-500 uppercase tracking-wide">New to the platform</div>
					</div>
					<div class="flex gap-3 overflow-x-auto pb-2 scrollbar-hide">
						{#each newProviders as provider}
							<a
								href="/dashboard/providers/{provider.pubkey}"
								class="flex-shrink-0 w-44 bg-surface-elevated border border-neutral-800 hover:border-neutral-600 rounded-lg p-3 transition-colors"
							>
								<div class="text-sm font-medium text-neutral-200 truncate mb-1">{provider.name}</div>
								<div class="text-xs text-neutral-500 mb-2">{provider.offerings_count} offering{provider.offerings_count !== 1 ? 's' : ''}</div>
								{#if provider.trust_score !== null && provider.trust_score !== undefined}
									<div class="text-xs text-neutral-400">Trust: {provider.trust_score}</div>
								{/if}
								<div class="text-xs text-green-500 mt-1">New · {provider.joined_days_ago}d ago</div>
							</a>
						{/each}
					</div>
				</div>
			{/if}

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
					{#if searchQuery}
						<p class="text-neutral-400 mb-1 font-medium">No results for "{searchQuery}"</p>
						<p class="text-neutral-600 text-sm mb-4">Try a different term, or use field syntax like <code class="text-neutral-400 bg-neutral-800 px-1">product_type:gpu</code></p>
					{:else}
						<p class="text-neutral-500 mb-2">No offerings found</p>
					{/if}
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
								<th class="pb-3 font-medium text-right text-neutral-500"></th>
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
													title="Provider is not actively monitoring — requests are still accepted when agent comes back online"
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
										<button
											onclick={(e) => toggleBookmark(e, offering.id!)}
											title={savedIds.has(offering.id) ? "Remove from saved" : "Save offering"}
											class="p-1 transition-colors {savedIds.has(offering.id) ? 'text-primary-400 hover:text-primary-300' : 'text-neutral-600 hover:text-neutral-400'}"
										>
											<Icon name="bookmark" size={16} />
										</button>
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
										<td colspan="8" class="p-4">
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
												title="Provider is not actively monitoring — requests are still accepted when agent comes back online"
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
								<div class="flex items-center gap-2 shrink-0">
									{#if offering.trust_score !== undefined}
										<TrustBadge
											score={offering.trust_score}
											hasFlags={offering.has_critical_flags ??
												false}
											compact={true}
										/>
									{/if}
									{#if offering.id !== undefined}
										<button
											onclick={(e) => toggleBookmark(e, offering.id!)}
											title={savedIds.has(offering.id) ? "Remove from saved" : "Save offering"}
											class="p-1 transition-colors {savedIds.has(offering.id) ? 'text-primary-400 hover:text-primary-300' : 'text-neutral-600 hover:text-neutral-400'}"
										>
											<Icon name="bookmark" size={16} />
										</button>
									{/if}
								</div>
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
							{#if offering.id !== undefined}
								{@const inCompare = compareIds.has(offering.id)}
								<div class="mt-2">
									<button
										onclick={(e) => toggleCompare(e, offering.id!)}
										title={inCompare ? "Remove from comparison" : "Add to comparison"}
										class="px-2 py-1 text-xs border rounded transition-colors {inCompare
											? 'bg-primary-500/20 text-primary-300 border-primary-400/50 hover:bg-primary-500/10'
											: 'bg-neutral-800 text-neutral-400 border-neutral-700 hover:bg-neutral-700 hover:text-white'}"
									>{inCompare ? "✓ In compare" : "+ Compare"}</button>
								</div>
							{/if}
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
					onclick={() => { compareIds = new Set(); }}
					class="px-3 py-1.5 text-xs border border-neutral-600 text-neutral-400 hover:text-white hover:border-neutral-500 rounded transition-colors"
				>Clear</button>
				<a
					href={compareIds.size >= 2 ? `/dashboard/marketplace/compare?ids=${[...compareIds].join(',')}` : undefined}
					aria-disabled={compareIds.size < 2}
					class="px-4 py-1.5 text-xs font-medium rounded transition-colors {compareIds.size >= 2
						? 'bg-primary-600 hover:bg-primary-500 text-white'
						: 'bg-neutral-700 text-neutral-500 cursor-not-allowed pointer-events-none'}"
				>Compare ({compareIds.size})</a>
			</div>
		</div>
	</div>
{/if}
