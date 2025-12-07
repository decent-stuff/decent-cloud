<script lang="ts">
	import { onMount } from "svelte";
	import { searchOfferings, type Offering } from "$lib/services/api";
	import RentalRequestDialog from "$lib/components/RentalRequestDialog.svelte";
	import AuthPromptModal from "$lib/components/AuthPromptModal.svelte";
	import TrustBadge from "$lib/components/TrustBadge.svelte";
	import { authStore } from "$lib/stores/auth";

	let offerings = $state<Offering[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let searchQuery = $state("");
	let selectedOffering = $state<Offering | null>(null);
	let successMessage = $state<string | null>(null);
	let isAuthenticated = $state(false);
	let showAuthModal = $state(false);
	let expandedRow = $state<string | null>(null);
	let sortDir = $state<"asc" | "desc">("asc");
	let showFilters = $state(false);
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;

	// Filters
	let selectedTypes = $state<Set<string>>(new Set());
	let minPrice = $state<number | undefined>(undefined);
	let maxPrice = $state<number | undefined>(undefined);
	let selectedCountry = $state<string>("");
	let selectedCity = $state<string>("");
	let minCores = $state<number | undefined>(undefined);
	let unmeteredOnly = $state(false);
	let minTrust = $state<number | undefined>(undefined);

	// Derived: unique countries and cities from offerings
	let countries = $derived([...new Set(offerings.map(o => o.datacenter_country).filter(Boolean))].sort());
	let cities = $derived(() => {
		const filtered = selectedCountry
			? offerings.filter(o => o.datacenter_country === selectedCountry)
			: offerings;
		return [...new Set(filtered.map(o => o.datacenter_city).filter(Boolean))].sort();
	});

	// Derived: filtered and sorted offerings
	let filteredOfferings = $derived.by(() => {
		let result = [...offerings];

		// Client-side type filter (multi-select)
		if (selectedTypes.size > 0) {
			result = result.filter(o => {
				const type = o.product_type.toLowerCase();
				for (const t of selectedTypes) {
					if (type.includes(t)) return true;
				}
				return false;
			});
		}

		// Client-side city filter
		if (selectedCity) {
			result = result.filter(o => o.datacenter_city === selectedCity);
		}

		// Client-side cores filter
		if (minCores !== undefined) {
			const threshold = minCores;
			result = result.filter(o => (o.processor_cores ?? 0) >= threshold);
		}

		// Client-side unmetered filter
		if (unmeteredOnly) {
			result = result.filter(o => o.unmetered_bandwidth);
		}

		// Client-side trust filter
		if (minTrust !== undefined) {
			const threshold = minTrust;
			result = result.filter(o => (o.trust_score ?? 0) >= threshold);
		}

		// Sort by price
		result.sort((a, b) => {
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
				q: searchQuery.trim() || undefined,
				country: selectedCountry || undefined,
				min_price_monthly: minPrice,
				max_price_monthly: maxPrice,
			});
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load offerings";
		} finally {
			loading = false;
		}
	}

	onMount(() => fetchOfferings());

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
		minPrice = undefined;
		maxPrice = undefined;
		selectedCountry = "";
		selectedCity = "";
		minCores = undefined;
		unmeteredOnly = false;
		minTrust = undefined;
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

	function toggleRow(id: string) {
		expandedRow = expandedRow === id ? null : id;
	}

	function toggleSort() {
		sortDir = sortDir === "asc" ? "desc" : "asc";
	}

	function handleRentalSuccess(contractId: string) {
		selectedOffering = null;
		successMessage = `Rental request created! Contract ID: ${contractId}`;
		setTimeout(() => successMessage = null, 5000);
	}

	function getTypeIcon(productType: string) {
		const type = productType.toLowerCase();
		if (type.includes("gpu")) return "🎮";
		if (type.includes("compute") || type.includes("vm")) return "💻";
		if (type.includes("storage")) return "💾";
		if (type.includes("network") || type.includes("cdn")) return "🌐";
		return "📦";
	}

	function formatPrice(offering: Offering): string {
		// If offering has a reseller, calculate price with commission
		if (offering.reseller_commission_percent && offering.monthly_price) {
			const basePrice = offering.monthly_price;
			const commission = basePrice * (offering.reseller_commission_percent / 100);
			const totalPrice = basePrice + commission;
			return `${totalPrice.toFixed(2)} ${offering.currency}`;
		}
		if (offering.monthly_price) return `${offering.monthly_price.toFixed(2)} ${offering.currency}`;
		return "On request";
	}

	function hasReseller(offering: Offering): boolean {
		return !!(offering.reseller_name && offering.reseller_commission_percent);
	}

	function getResellerBadgeText(offering: Offering): string {
		if (!offering.reseller_name) return "";
		const commission = offering.reseller_commission_percent || 0;
		return `Via ${offering.reseller_name} (+${commission}%)`;
	}

	function formatSpecs(offering: Offering): string {
		const type = offering.product_type.toLowerCase();
		if (type.includes("gpu")) {
			const parts = [offering.gpu_name, offering.gpu_count ? `${offering.gpu_count}x` : null, offering.gpu_memory_gb ? `${offering.gpu_memory_gb}GB` : null].filter(Boolean);
			return parts.join(" ") || "—";
		}
		const parts = [
			offering.processor_cores ? `${offering.processor_cores} vCPU` : null,
			offering.memory_amount,
			offering.total_ssd_capacity ? `${offering.total_ssd_capacity} SSD` : offering.total_hdd_capacity ? `${offering.total_hdd_capacity} HDD` : null
		].filter(Boolean);
		return parts.join(" · ") || "—";
	}

	function formatLocation(offering: Offering): string {
		if (offering.datacenter_city && offering.datacenter_country) {
			return `${offering.datacenter_city}, ${offering.datacenter_country}`;
		}
		return offering.datacenter_country || "—";
	}

	function shortPubkey(pubkey: string): string {
		return pubkey.length <= 12 ? pubkey : `${pubkey.slice(0, 6)}…${pubkey.slice(-4)}`;
	}

	const typeOptions = [
		{ key: "compute", label: "Compute", icon: "💻" },
		{ key: "gpu", label: "GPU", icon: "🎮" },
		{ key: "storage", label: "Storage", icon: "💾" },
		{ key: "network", label: "Network", icon: "🌐" },
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
</script>

<div class="space-y-4">
	<div>
		<h1 class="text-3xl font-bold text-white">Marketplace</h1>
		<p class="text-white/60 text-sm">Find and rent cloud resources</p>
	</div>

	{#if successMessage}
		<div class="bg-green-500/20 border border-green-500/30 rounded-lg p-3 text-green-400 text-sm">{successMessage}</div>
	{/if}

	{#if error}
		<div class="bg-red-500/20 border border-red-500/30 rounded-lg p-3 text-red-400 text-sm">{error}</div>
	{/if}

	<div class="flex flex-col md:flex-row gap-6">
		<!-- Filters: collapsible on mobile, sidebar on desktop -->
		<aside class="w-full md:w-56 shrink-0">
			<div class="bg-white/5 rounded-lg p-4">
				<div class="flex items-center justify-between">
					<button
						onclick={() => showFilters = !showFilters}
						class="flex items-center gap-2 md:cursor-default"
					>
						<span class="text-white font-medium text-sm">Filters</span>
						<span class="md:hidden text-white/60 text-sm">{showFilters ? '▲' : '▼'}</span>
					</button>
					<button onclick={clearFilters} class="text-xs text-blue-400 hover:text-blue-300">Clear</button>
				</div>

				<div class="space-y-4 mt-4 {showFilters ? '' : 'hidden'} md:block">

				<!-- Type Filter -->
				<div>
					<div class="text-white/60 text-xs uppercase tracking-wide mb-2">Type</div>
					<div class="space-y-1">
						{#each typeOptions as opt}
							<label class="flex items-center gap-2 cursor-pointer group">
								<input
									type="checkbox"
									checked={selectedTypes.has(opt.key)}
									onchange={() => toggleType(opt.key)}
									class="rounded border-white/30 bg-white/10 text-blue-500 focus:ring-blue-500"
								/>
								<span class="text-sm text-white/80 group-hover:text-white">{opt.icon} {opt.label}</span>
							</label>
						{/each}
					</div>
				</div>

				<!-- Price Filter -->
				<div>
					<div class="text-white/60 text-xs uppercase tracking-wide mb-2">Price ($/mo)</div>
					<div class="flex gap-2">
						<input
							type="number"
							placeholder="Min"
							bind:value={minPrice}
							onchange={handleFilterChange}
							class="w-full px-2 py-1.5 text-sm bg-white/10 border border-white/20 rounded text-white placeholder-white/40 focus:outline-none focus:border-blue-400"
						/>
						<input
							type="number"
							placeholder="Max"
							bind:value={maxPrice}
							onchange={handleFilterChange}
							class="w-full px-2 py-1.5 text-sm bg-white/10 border border-white/20 rounded text-white placeholder-white/40 focus:outline-none focus:border-blue-400"
						/>
					</div>
				</div>

				<!-- Country Filter -->
				<div>
					<div class="text-white/60 text-xs uppercase tracking-wide mb-2">Country</div>
					<select
						bind:value={selectedCountry}
						onchange={() => { selectedCity = ""; handleFilterChange(); }}
						class="w-full px-2 py-1.5 text-sm bg-slate-800 border border-white/20 rounded text-white focus:outline-none focus:border-blue-400"
					>
						<option value="" class="bg-slate-800">All countries</option>
						{#each countries as country}
							<option value={country} class="bg-slate-800">{country}</option>
						{/each}
					</select>
				</div>

				<!-- City Filter -->
				<div>
					<div class="text-white/60 text-xs uppercase tracking-wide mb-2">City</div>
					<select
						bind:value={selectedCity}
						class="w-full px-2 py-1.5 text-sm bg-slate-800 border border-white/20 rounded text-white focus:outline-none focus:border-blue-400"
					>
						<option value="" class="bg-slate-800">All cities</option>
						{#each cities() as city}
							<option value={city} class="bg-slate-800">{city}</option>
						{/each}
					</select>
				</div>

				<!-- CPU Cores Filter -->
				<div>
					<div class="text-white/60 text-xs uppercase tracking-wide mb-2">Min CPU Cores</div>
					<input
						type="number"
						placeholder="e.g., 4"
						bind:value={minCores}
						min="1"
						class="w-full px-2 py-1.5 text-sm bg-white/10 border border-white/20 rounded text-white placeholder-white/40 focus:outline-none focus:border-blue-400"
					/>
				</div>

				<!-- Min Trust Filter -->
				<div>
					<div class="text-white/60 text-xs uppercase tracking-wide mb-2">Min Trust Score</div>
					<input
						type="number"
						placeholder="0-100"
						bind:value={minTrust}
						min="0"
						max="100"
						class="w-full px-2 py-1.5 text-sm bg-white/10 border border-white/20 rounded text-white placeholder-white/40 focus:outline-none focus:border-blue-400"
					/>
				</div>

				<!-- Unmetered Bandwidth Filter -->
				<div>
					<label class="flex items-center gap-2 cursor-pointer group">
						<input
							type="checkbox"
							bind:checked={unmeteredOnly}
							class="rounded border-white/30 bg-white/10 text-blue-500 focus:ring-blue-500"
						/>
						<span class="text-sm text-white/80 group-hover:text-white">Unmetered bandwidth only</span>
					</label>
				</div>
				</div>
			</div>
		</aside>

		<!-- Main Content -->
		<div class="flex-1 min-w-0 space-y-4">
			<!-- Search -->
			<input
				type="text"
				placeholder="Search (e.g., type:gpu price:<=100)..."
				bind:value={searchQuery}
				oninput={handleSearchInput}
				class="w-full px-4 py-2.5 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:border-blue-400"
			/>

			<!-- Results count -->
			<div class="text-white/60 text-sm">
				{filteredOfferings.length} offerings
			</div>

			{#if loading}
				<div class="flex justify-center py-12">
					<div class="animate-spin rounded-full h-10 w-10 border-t-2 border-b-2 border-blue-400"></div>
				</div>
			{:else if filteredOfferings.length === 0}
				<div class="text-center py-12">
					<span class="text-5xl block mb-3">🔍</span>
					<p class="text-white/60">No offerings found</p>
				</div>
			{:else}
				<!-- Desktop Table -->
				<div class="hidden md:block overflow-x-auto">
					<table class="w-full text-sm">
						<thead>
							<tr class="text-left text-white/60 border-b border-white/10">
								<th class="pb-3 font-medium">Offering</th>
								<th class="pb-3 font-medium">Type</th>
								<th class="pb-3 font-medium">Specs</th>
								<th class="pb-3 font-medium">Location</th>
								<th class="pb-3 font-medium cursor-pointer hover:text-white" onclick={toggleSort}>
									Price {sortDir === "asc" ? "↑" : "↓"}
								</th>
								<th class="pb-3 font-medium"></th>
							</tr>
						</thead>
						<tbody>
							{#each filteredOfferings as offering (offering.offering_id)}
								{@const isExpanded = expandedRow === offering.offering_id}
								<tr
									class="border-b border-white/5 hover:bg-white/5 cursor-pointer transition-colors"
									onclick={() => toggleRow(offering.offering_id)}
								>
									<td class="py-3 pr-4">
										<div class="flex items-center gap-2">
											<span class="font-medium text-white">{offering.offer_name}</span>
											{#if offering.trust_score !== undefined}
												<TrustBadge score={offering.trust_score} hasFlags={offering.has_critical_flags ?? false} compact={true} />
											{/if}
											{#if hasReseller(offering)}
												<span class="px-1.5 py-0.5 text-xs bg-blue-500/20 text-blue-400 rounded">{getResellerBadgeText(offering)}</span>
											{:else if offering.offering_source === 'seeded'}
												<span class="px-1.5 py-0.5 text-xs bg-purple-500/20 text-purple-400 rounded">External</span>
											{:else if offering.is_example}
												<span class="px-1.5 py-0.5 text-xs bg-amber-500/20 text-amber-400 rounded">Demo</span>
											{/if}
										</div>
										<a
											href="/dashboard/reputation/{offering.pubkey}"
											onclick={(e) => e.stopPropagation()}
											class="text-xs text-white/50 hover:text-blue-400 font-mono"
										>{shortPubkey(offering.pubkey)}</a>
									</td>
									<td class="py-3 pr-4">
										<span class="whitespace-nowrap">{getTypeIcon(offering.product_type)} {offering.product_type}</span>
									</td>
									<td class="py-3 pr-4 text-white/80">{formatSpecs(offering)}</td>
									<td class="py-3 pr-4 text-white/80">{formatLocation(offering)}</td>
									<td class="py-3 pr-4 font-medium text-white">{formatPrice(offering)}</td>
									<td class="py-3">
										{#if hasReseller(offering)}
											<button
												onclick={(e) => handleRentClick(e, offering)}
												class="px-3 py-1.5 bg-blue-600 hover:bg-blue-500 rounded text-xs font-medium whitespace-nowrap"
											>Rent</button>
										{:else if offering.offering_source === 'seeded' && offering.external_checkout_url}
											<a
												href={offering.external_checkout_url}
												target="_blank"
												rel="noopener noreferrer"
												onclick={(e) => e.stopPropagation()}
												class="px-3 py-1.5 bg-purple-600 hover:bg-purple-500 rounded text-xs font-medium whitespace-nowrap inline-block"
											>Visit Provider ↗</a>
										{:else}
											<button
												onclick={(e) => handleRentClick(e, offering)}
												disabled={offering.is_example}
												class="px-3 py-1.5 bg-blue-600 hover:bg-blue-500 rounded text-xs font-medium disabled:opacity-50 disabled:cursor-not-allowed whitespace-nowrap"
											>Rent</button>
										{/if}
									</td>
								</tr>
								{#if isExpanded}
									<tr class="bg-white/5">
										<td colspan="6" class="p-4">
											<div class="grid grid-cols-3 gap-4 text-sm">
												<div>
													<div class="text-white/60 text-xs uppercase mb-1">Description</div>
													<div class="text-white/80">{offering.description || "No description"}</div>
												</div>
												<div class="space-y-2">
													{#if offering.processor_name || offering.processor_brand}
														<div><span class="text-white/60">CPU:</span> <span class="text-white/80">{offering.processor_name || offering.processor_brand}{offering.processor_speed ? ` @ ${offering.processor_speed}` : ""}</span></div>
													{/if}
													{#if offering.memory_amount}
														<div><span class="text-white/60">Memory:</span> <span class="text-white/80">{offering.memory_amount}{offering.memory_type ? ` ${offering.memory_type}` : ""}{offering.memory_error_correction ? ` (${offering.memory_error_correction})` : ""}</span></div>
													{/if}
													{#if offering.total_ssd_capacity || offering.total_hdd_capacity}
														<div><span class="text-white/60">Storage:</span> <span class="text-white/80">{[offering.total_ssd_capacity ? `${offering.total_ssd_capacity} SSD` : null, offering.total_hdd_capacity ? `${offering.total_hdd_capacity} HDD` : null].filter(Boolean).join(" + ")}</span></div>
													{/if}
													{#if offering.uplink_speed || offering.unmetered_bandwidth}
														<div><span class="text-white/60">Network:</span> <span class="text-white/80">{offering.uplink_speed || ""}{offering.unmetered_bandwidth ? " (Unmetered)" : offering.traffic ? ` (${offering.traffic} TB)` : ""}</span></div>
													{/if}
													{#if offering.virtualization_type}
														<div><span class="text-white/60">Platform:</span> <span class="text-white/80">{offering.virtualization_type}</span></div>
													{/if}
												</div>
												<div class="space-y-2">
													<div><span class="text-white/60">Billing:</span> <span class="text-white/80">{formatBilling(offering)}</span></div>
													{#if offering.setup_fee > 0}
														<div><span class="text-white/60">Setup Fee:</span> <span class="text-white/80">{offering.setup_fee.toFixed(2)} {offering.currency}</span></div>
													{/if}
													{#if offering.min_contract_hours || offering.max_contract_hours}
														<div><span class="text-white/60">Contract:</span> <span class="text-white/80">{formatContractTerms(offering)}</span></div>
													{/if}
													{#if offering.operating_systems}
														<div><span class="text-white/60">OS:</span> <span class="text-white/80">{offering.operating_systems}</span></div>
													{/if}
													{#if offering.features}
														<div><span class="text-white/60">Features:</span> <span class="text-white/80">{offering.features}</span></div>
													{/if}
													{#if offering.control_panel}
														<div><span class="text-white/60">Control Panel:</span> <span class="text-white/80">{offering.control_panel}</span></div>
													{/if}
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
					{#each filteredOfferings as offering (offering.offering_id)}
						<div
							role="button"
							tabindex="0"
							class="bg-white/5 rounded-lg p-4 border border-white/10"
							onclick={() => toggleRow(offering.offering_id)}
							onkeydown={(e) => e.key === 'Enter' && toggleRow(offering.offering_id)}
						>
							<div class="flex items-start justify-between mb-2">
								<div>
									<div class="flex items-center gap-2">
										<span class="font-medium text-white">{offering.offer_name}</span>
										{#if hasReseller(offering)}
											<span class="px-1.5 py-0.5 text-xs bg-blue-500/20 text-blue-400 rounded">{getResellerBadgeText(offering)}</span>
										{:else if offering.offering_source === 'seeded'}
											<span class="px-1.5 py-0.5 text-xs bg-purple-500/20 text-purple-400 rounded">External</span>
										{:else if offering.is_example}
											<span class="px-1.5 py-0.5 text-xs bg-amber-500/20 text-amber-400 rounded">Demo</span>
										{/if}
									</div>
									<div class="text-xs text-white/50">{getTypeIcon(offering.product_type)} {offering.product_type}</div>
								</div>
								{#if offering.trust_score !== undefined}
									<TrustBadge score={offering.trust_score} hasFlags={offering.has_critical_flags ?? false} compact={true} />
								{/if}
							</div>
							<div class="text-sm text-white/70 mb-2">{formatSpecs(offering)}</div>
							<div class="flex items-center justify-between">
								<div>
									<div class="text-white font-medium">{formatPrice(offering)}</div>
									<div class="text-xs text-white/50">{formatLocation(offering)}</div>
								</div>
								{#if hasReseller(offering)}
									<button
										onclick={(e) => handleRentClick(e, offering)}
										class="px-3 py-1.5 bg-blue-600 hover:bg-blue-500 rounded text-xs font-medium"
									>Rent</button>
								{:else if offering.offering_source === 'seeded' && offering.external_checkout_url}
									<a
										href={offering.external_checkout_url}
										target="_blank"
										rel="noopener noreferrer"
										onclick={(e) => e.stopPropagation()}
										class="px-3 py-1.5 bg-purple-600 hover:bg-purple-500 rounded text-xs font-medium"
									>Visit Provider ↗</a>
								{:else}
									<button
										onclick={(e) => handleRentClick(e, offering)}
										disabled={offering.is_example}
										class="px-3 py-1.5 bg-blue-600 hover:bg-blue-500 rounded text-xs font-medium disabled:opacity-50"
									>Rent</button>
								{/if}
							</div>
							{#if expandedRow === offering.offering_id}
								<div class="mt-3 pt-3 border-t border-white/10 text-sm space-y-2">
									<div class="text-white/70">{offering.description || "No description"}</div>
									<div class="grid grid-cols-2 gap-2 text-xs">
										{#if offering.processor_name || offering.processor_brand}
											<div><span class="text-white/50">CPU:</span> <span class="text-white/70">{offering.processor_name || offering.processor_brand}</span></div>
										{/if}
										{#if offering.memory_amount}
											<div><span class="text-white/50">Memory:</span> <span class="text-white/70">{offering.memory_amount}</span></div>
										{/if}
										{#if offering.total_ssd_capacity || offering.total_hdd_capacity}
											<div><span class="text-white/50">Storage:</span> <span class="text-white/70">{offering.total_ssd_capacity || offering.total_hdd_capacity}</span></div>
										{/if}
										{#if offering.virtualization_type}
											<div><span class="text-white/50">Platform:</span> <span class="text-white/70">{offering.virtualization_type}</span></div>
										{/if}
										<div><span class="text-white/50">Billing:</span> <span class="text-white/70">{formatBilling(offering)}</span></div>
										{#if offering.setup_fee > 0}
											<div><span class="text-white/50">Setup:</span> <span class="text-white/70">{offering.setup_fee.toFixed(2)} {offering.currency}</span></div>
										{/if}
										{#if offering.min_contract_hours || offering.max_contract_hours}
											<div><span class="text-white/50">Contract:</span> <span class="text-white/70">{formatContractTerms(offering)}</span></div>
										{/if}
										{#if offering.unmetered_bandwidth}
											<div><span class="text-white/50">Bandwidth:</span> <span class="text-white/70">Unmetered</span></div>
										{/if}
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
	onClose={() => selectedOffering = null}
	onSuccess={handleRentalSuccess}
/>

<AuthPromptModal
	isOpen={showAuthModal}
	onClose={() => showAuthModal = false}
	message="Create an account or login to rent cloud resources"
/>
