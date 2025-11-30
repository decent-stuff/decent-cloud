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
	let selectedType = $state<"all" | string>("all");
	let selectedOffering = $state<Offering | null>(null);
	let successMessage = $state<string | null>(null);
	let isAuthenticated = $state(false);
	let showAuthModal = $state(false);

	authStore.isAuthenticated.subscribe((value) => {
		isAuthenticated = value;
	});

	onMount(async () => {
		try {
			loading = true;
			error = null;
			offerings = await searchOfferings({
				limit: 100,
				in_stock_only: true,
			});
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load offerings";
			console.error("Error loading offerings:", e);
		} finally {
			loading = false;
		}
	});

	function handleRentClick(offering: Offering) {
		if (!isAuthenticated) {
			showAuthModal = true;
			return;
		}
		selectedOffering = offering;
	}

	function handleDialogClose() {
		selectedOffering = null;
	}

	function handleRentalSuccess(contractId: string) {
		selectedOffering = null;
		successMessage = `Rental request created successfully! Contract ID: ${contractId}`;
		setTimeout(() => {
			successMessage = null;
		}, 5000);
	}

	const filteredOfferings = $derived(
		offerings.filter((offering) => {
			const matchesSearch =
				offering.offer_name
					.toLowerCase()
					.includes(searchQuery.toLowerCase()) ||
				offering.description
					?.toLowerCase()
					.includes(searchQuery.toLowerCase()) ||
				offering.product_type
					.toLowerCase()
					.includes(searchQuery.toLowerCase());
			const matchesType =
				selectedType === "all" ||
				offering.product_type === selectedType;
			return matchesSearch && matchesType;
		}),
	);

	function getTypeIcon(productType: string) {
		const type = productType.toLowerCase();
		if (type.includes("gpu")) return "🎮";
		if (type.includes("compute") || type.includes("vm")) return "💻";
		if (type.includes("storage")) return "💾";
		if (type.includes("network") || type.includes("cdn")) return "🌐";
		return "📦";
	}

	function formatPrice(offering: Offering): string {
		if (offering.monthly_price) {
			return `${offering.monthly_price.toFixed(2)} ${offering.currency}/mo`;
		}
		return "Price on request";
	}

	function formatSpecs(offering: Offering): string {
		const specs: string[] = [];
		const type = offering.product_type.toLowerCase();

		// GPU-specific specs
		if (type.includes("gpu")) {
			if (offering.gpu_name) specs.push(offering.gpu_name);
			if (offering.gpu_count) specs.push(`${offering.gpu_count}x GPU`);
			if (offering.gpu_memory_gb) specs.push(`${offering.gpu_memory_gb}GB VRAM`);
		} else {
			// Default compute specs
			if (offering.processor_cores) {
				specs.push(`${offering.processor_cores} vCPU`);
			}
			if (offering.memory_amount) {
				specs.push(`${offering.memory_amount} RAM`);
			}
			if (offering.total_ssd_capacity) {
				specs.push(`${offering.total_ssd_capacity} SSD`);
			} else if (offering.total_hdd_capacity) {
				specs.push(`${offering.total_hdd_capacity} HDD`);
			}
		}

		if (offering.datacenter_country) {
			specs.push(
				`${offering.datacenter_city}, ${offering.datacenter_country}`,
			);
		}
		return specs.length > 0
			? specs.join(", ")
			: offering.description || "No details available";
	}

	function shortPubkey(pubkey: string): string {
		if (pubkey.length <= 12) return pubkey;
		return `${pubkey.slice(0, 6)}...${pubkey.slice(-6)}`;
	}
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">Marketplace</h1>
		<p class="text-white/60">
			Discover and purchase cloud services from trusted providers
		</p>
	</div>

	{#if successMessage}
		<div
			class="bg-green-500/20 border border-green-500/30 rounded-lg p-4 text-green-400"
		>
			<p class="font-semibold">Success!</p>
			<p class="text-sm mt-1">{successMessage}</p>
		</div>
	{/if}

	{#if error}
		<div
			class="bg-red-500/20 border border-red-500/30 rounded-lg p-4 text-red-400"
		>
			<p class="font-semibold">Error loading marketplace</p>
			<p class="text-sm mt-1">{error}</p>
		</div>
	{/if}

	{#if loading}
		<div class="flex justify-center items-center p-8">
			<div
				class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-400"
			></div>
		</div>
	{:else}
		<!-- Search and Filters -->
		<div class="flex flex-col md:flex-row gap-4">
			<div class="flex-1">
				<input
					type="text"
					placeholder="Search offerings..."
					bind:value={searchQuery}
					class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:border-blue-400 transition-colors"
				/>
			</div>
			<div class="flex gap-2">
				<button
					onclick={() => (selectedType = "all")}
					class="px-4 py-3 rounded-lg font-medium transition-all {selectedType ===
					'all'
						? 'bg-blue-600 text-white'
						: 'bg-white/10 text-white/70 hover:bg-white/20'}"
				>
					All
				</button>
				<button
					onclick={() => (selectedType = "compute")}
					class="px-4 py-3 rounded-lg font-medium transition-all {selectedType ===
					'compute'
						? 'bg-blue-600 text-white'
						: 'bg-white/10 text-white/70 hover:bg-white/20'}"
				>
					💻 Compute
				</button>
				<button
					onclick={() => (selectedType = "gpu")}
					class="px-4 py-3 rounded-lg font-medium transition-all {selectedType ===
					'gpu'
						? 'bg-blue-600 text-white'
						: 'bg-white/10 text-white/70 hover:bg-white/20'}"
				>
					🎮 GPU
				</button>
				<button
					onclick={() => (selectedType = "storage")}
					class="px-4 py-3 rounded-lg font-medium transition-all {selectedType ===
					'storage'
						? 'bg-blue-600 text-white'
						: 'bg-white/10 text-white/70 hover:bg-white/20'}"
				>
					💾 Storage
				</button>
				<button
					onclick={() => (selectedType = "network")}
					class="px-4 py-3 rounded-lg font-medium transition-all {selectedType ===
					'network'
						? 'bg-blue-600 text-white'
						: 'bg-white/10 text-white/70 hover:bg-white/20'}"
				>
					🌐 Network
				</button>
			</div>
		</div>

		<!-- Results Count -->
		<div class="text-white/60">
			Showing {filteredOfferings.length} of {offerings.length} offerings
		</div>

		<!-- Marketplace Grid -->
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
			{#each filteredOfferings as offering}
				<div
					class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20 hover:border-blue-400 transition-all group"
				>
					<div class="flex items-start justify-between mb-4">
						<span class="text-4xl"
							>{getTypeIcon(offering.product_type)}</span
						>
						<div class="flex items-center gap-2">
							{#if offering.trust_score !== undefined}
								<TrustBadge
									score={offering.trust_score}
									hasFlags={offering.has_critical_flags ?? false}
									compact={true}
								/>
							{/if}
							<span
								class="inline-flex items-center gap-1 px-3 py-1 rounded-full text-xs font-medium bg-green-500/20 text-green-400 border border-green-500/30"
							>
								<span class="w-2 h-2 rounded-full bg-current"
								></span>
								Available
							</span>
						</div>
					</div>

					<h3
						class="text-xl font-bold text-white mb-1 group-hover:text-blue-400 transition-colors"
					>
						{offering.offer_name}
					</h3>
					<p class="text-white/60 text-sm mb-2">
						{offering.product_type}
					</p>

					<!-- Provider Info -->
					<a
						href="/dashboard/reputation/{offering.pubkey}"
						class="flex items-center gap-2 text-sm text-white/70 hover:text-blue-400 transition-colors mb-4"
					>
						<span class="text-xs">👤</span>
						<span class="font-mono text-xs"
							>{shortPubkey(offering.pubkey)}</span
						>
						<span class="text-xs">→</span>
					</a>

					<div class="space-y-2 text-sm mb-4">
						<div
							class="flex items-center justify-between text-white/70"
						>
							<span>Type</span>
							<span class="text-white font-medium"
								>{offering.product_type}</span
							>
						</div>
						<div
							class="flex items-center justify-between text-white/70"
						>
							<span>Price</span>
							<span class="text-white font-medium"
								>{formatPrice(offering)}</span
							>
						</div>
						{#if offering.datacenter_country}
							<div
								class="flex items-center justify-between text-white/70"
							>
								<span>Location</span>
								<span class="text-white font-medium"
									>{offering.datacenter_city}, {offering.datacenter_country}</span
								>
							</div>
						{/if}
					</div>

					<div
						class="text-white/60 text-sm mb-4 p-3 bg-white/5 rounded-lg"
					>
						{formatSpecs(offering)}
					</div>

					<button
						onclick={() => handleRentClick(offering)}
						class="w-full px-4 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold hover:brightness-110 hover:scale-105 transition-all"
					>
						🚀 Rent Resource
					</button>
				</div>
			{/each}
		</div>

		<!-- Empty State -->
		{#if filteredOfferings.length === 0}
			<div class="text-center py-16">
				<span class="text-6xl mb-4 block">🔍</span>
				<h3 class="text-2xl font-bold text-white mb-2">
					No Results Found
				</h3>
				<p class="text-white/60">
					Try adjusting your search or filters
				</p>
			</div>
		{/if}
	{/if}
</div>

<!-- Rental Request Dialog -->
<RentalRequestDialog
	offering={selectedOffering}
	onClose={handleDialogClose}
	onSuccess={handleRentalSuccess}
/>

<!-- Auth Prompt Modal -->
<AuthPromptModal
	isOpen={showAuthModal}
	onClose={() => showAuthModal = false}
	message="Create an account or login to rent cloud resources"
/>
