<script lang="ts">
	import { onMount } from 'svelte';
	import { getProviderOfferings, type Offering } from '$lib/services/api';
	import { authStore } from '$lib/stores/auth';
	import { hexEncode } from '$lib/services/api';

	let offerings = $state<Offering[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let currentIdentity = $state<any>(null);

	onMount(async () => {
		const unsubscribe = authStore.currentIdentity.subscribe((identity) => {
			currentIdentity = identity;
		});

		try {
			loading = true;
			error = null;

			if (!currentIdentity || !currentIdentity.publicKeyBytes) {
				error = 'Please authenticate to view your offerings';
				return;
			}

			// Convert public key bytes to hex for API
			const pubkeyHex = hexEncode(currentIdentity.publicKeyBytes);
			offerings = await getProviderOfferings(pubkeyHex);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load offerings';
			console.error('Error loading offerings:', e);
		} finally {
			loading = false;
		}

		return unsubscribe;
	});

	function getStatusColor(stockStatus: string) {
		switch (stockStatus) {
			case 'in_stock':
				return 'bg-green-500/20 text-green-400 border-green-500/30';
			case 'out_of_stock':
				return 'bg-red-500/20 text-red-400 border-red-500/30';
			case 'discontinued':
				return 'bg-gray-500/20 text-gray-400 border-gray-500/30';
			default:
				return 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30';
		}
	}

	function getTypeIcon(productType: string) {
		const type = productType.toLowerCase();
		if (type.includes('compute') || type.includes('vm')) return '💻';
		if (type.includes('storage')) return '💾';
		if (type.includes('network') || type.includes('cdn')) return '🌐';
		return '📦';
	}

	function formatPrice(offering: Offering): string {
		if (offering.price_per_hour_e9s) {
			const price = offering.price_per_hour_e9s / 1_000_000_000;
			return `${price.toFixed(4)} DCT/hr`;
		}
		if (offering.monthly_price) {
			return `${offering.monthly_price.toFixed(2)} ${offering.currency}/mo`;
		}
		return 'Price on request';
	}
</script>

<div class="space-y-8">
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-4xl font-bold text-white mb-2">My Offerings</h1>
			<p class="text-white/60">Manage your cloud service offerings</p>
		</div>
		<button
			class="px-6 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold hover:brightness-110 hover:scale-105 transition-all"
		>
			+ Create Offering
		</button>
	</div>

	{#if error}
		<div class="bg-red-500/20 border border-red-500/30 rounded-lg p-4 text-red-400">
			<p class="font-semibold">Error loading offerings</p>
			<p class="text-sm mt-1">{error}</p>
		</div>
	{/if}

	{#if loading}
		<div class="flex justify-center items-center p-8">
			<div class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-400"></div>
		</div>
	{:else}
		<!-- Stats Summary -->
		<div class="grid grid-cols-1 md:grid-cols-3 gap-6">
			<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
				<div class="flex items-center justify-between mb-2">
					<h3 class="text-white/70 text-sm font-medium">Active Offerings</h3>
					<span class="text-2xl">📦</span>
				</div>
				<p class="text-3xl font-bold text-white">
					{offerings.filter((o) => o.stock_status === 'in_stock').length}
				</p>
			</div>

			<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
				<div class="flex items-center justify-between mb-2">
					<h3 class="text-white/70 text-sm font-medium">Total Offerings</h3>
					<span class="text-2xl">📊</span>
				</div>
				<p class="text-3xl font-bold text-white">{offerings.length}</p>
			</div>

			<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
				<div class="flex items-center justify-between mb-2">
					<h3 class="text-white/70 text-sm font-medium">Product Types</h3>
					<span class="text-2xl">🏷️</span>
				</div>
				<p class="text-3xl font-bold text-white">
					{new Set(offerings.map((o) => o.product_type)).size}
				</p>
			</div>
		</div>

		<!-- Offerings Grid -->
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
			{#each offerings as offering}
				<div
					class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20 hover:border-white/40 transition-all group"
				>
					<div class="flex items-start justify-between mb-4">
						<span class="text-4xl">{getTypeIcon(offering.product_type)}</span>
						<span
							class="inline-flex items-center gap-1 px-3 py-1 rounded-full text-xs font-medium border {getStatusColor(
								offering.stock_status
							)}"
						>
							<span class="w-2 h-2 rounded-full bg-current"></span>
							{offering.stock_status.replace('_', ' ')}
						</span>
					</div>

					<h3 class="text-xl font-bold text-white mb-2 group-hover:text-blue-400 transition-colors">
						{offering.offer_name}
					</h3>

					<div class="space-y-2 text-sm">
						<div class="flex items-center justify-between text-white/70">
							<span>Type</span>
							<span class="text-white font-medium">{offering.product_type}</span>
						</div>
						<div class="flex items-center justify-between text-white/70">
							<span>Price</span>
							<span class="text-white font-medium">{formatPrice(offering)}</span>
						</div>
						{#if offering.datacenter_country}
							<div class="flex items-center justify-between text-white/70">
								<span>Location</span>
								<span class="text-white font-medium">{offering.datacenter_city}, {offering.datacenter_country}</span>
							</div>
						{/if}
						{#if offering.description}
							<div class="text-white/60 text-xs mt-2 line-clamp-2">{offering.description}</div>
						{/if}
					</div>

					<div class="mt-4 pt-4 border-t border-white/10 flex gap-2">
						<button
							class="flex-1 px-4 py-2 bg-white/10 rounded-lg text-sm font-medium hover:bg-white/20 transition-all"
						>
							Edit
						</button>
						<button
							class="flex-1 px-4 py-2 bg-white/10 rounded-lg text-sm font-medium hover:bg-white/20 transition-all"
						>
							{offering.stock_status === 'in_stock' ? 'Disable' : 'Enable'}
						</button>
					</div>
				</div>
			{/each}
		</div>

		<!-- Empty State (if no offerings) -->
		{#if offerings.length === 0}
			<div class="text-center py-16">
				<span class="text-6xl mb-4 block">📦</span>
				<h3 class="text-2xl font-bold text-white mb-2">No Offerings Yet</h3>
				<p class="text-white/60 mb-6">Create your first cloud service offering to get started</p>
				<button
					class="px-6 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold hover:brightness-110 hover:scale-105 transition-all"
				>
					Create Your First Offering
				</button>
			</div>
		{/if}
	{/if}
</div>
