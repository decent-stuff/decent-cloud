<script lang="ts">
	// TODO: Integrate with actual marketplace API when available
	interface MarketplaceOffering {
		id: string;
		provider: string;
		name: string;
		type: 'VM' | 'Storage' | 'Network';
		price: number;
		rating: number;
		available: boolean;
		specs: string;
	}

	const mockMarketplace: MarketplaceOffering[] = [
		{
			id: '1',
			provider: 'Provider Alpha',
			name: 'High Performance VM',
			type: 'VM',
			price: 0.08,
			rating: 4.8,
			available: true,
			specs: '8 vCPU, 16GB RAM, 200GB SSD'
		},
		{
			id: '2',
			provider: 'Provider Beta',
			name: 'Object Storage Pro',
			type: 'Storage',
			price: 0.03,
			rating: 4.9,
			available: true,
			specs: '1TB storage, 99.9% uptime'
		},
		{
			id: '3',
			provider: 'Provider Gamma',
			name: 'CDN Edge Network',
			type: 'Network',
			price: 0.12,
			rating: 4.7,
			available: true,
			specs: 'Global CDN, DDoS protection'
		},
		{
			id: '4',
			provider: 'Provider Delta',
			name: 'Budget VM',
			type: 'VM',
			price: 0.02,
			rating: 4.5,
			available: true,
			specs: '2 vCPU, 4GB RAM, 50GB SSD'
		},
		{
			id: '5',
			provider: 'Provider Epsilon',
			name: 'Archive Storage',
			type: 'Storage',
			price: 0.01,
			rating: 4.6,
			available: false,
			specs: 'Cold storage, unlimited'
		}
	];

	let searchQuery = '';
	let selectedType: 'all' | 'VM' | 'Storage' | 'Network' = 'all';

	$: filteredOfferings = mockMarketplace.filter((offering) => {
		const matchesSearch =
			offering.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
			offering.provider.toLowerCase().includes(searchQuery.toLowerCase());
		const matchesType = selectedType === 'all' || offering.type === selectedType;
		return matchesSearch && matchesType;
	});

	function getTypeIcon(type: string) {
		switch (type) {
			case 'VM':
				return '💻';
			case 'Storage':
				return '💾';
			case 'Network':
				return '🌐';
			default:
				return '📦';
		}
	}
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">Marketplace</h1>
		<p class="text-white/60">Discover and purchase cloud services from trusted providers</p>
	</div>

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
				on:click={() => (selectedType = 'all')}
				class="px-4 py-3 rounded-lg font-medium transition-all {selectedType === 'all'
					? 'bg-blue-600 text-white'
					: 'bg-white/10 text-white/70 hover:bg-white/20'}"
			>
				All
			</button>
			<button
				on:click={() => (selectedType = 'VM')}
				class="px-4 py-3 rounded-lg font-medium transition-all {selectedType === 'VM'
					? 'bg-blue-600 text-white'
					: 'bg-white/10 text-white/70 hover:bg-white/20'}"
			>
				💻 VM
			</button>
			<button
				on:click={() => (selectedType = 'Storage')}
				class="px-4 py-3 rounded-lg font-medium transition-all {selectedType === 'Storage'
					? 'bg-blue-600 text-white'
					: 'bg-white/10 text-white/70 hover:bg-white/20'}"
			>
				💾 Storage
			</button>
			<button
				on:click={() => (selectedType = 'Network')}
				class="px-4 py-3 rounded-lg font-medium transition-all {selectedType === 'Network'
					? 'bg-blue-600 text-white'
					: 'bg-white/10 text-white/70 hover:bg-white/20'}"
			>
				🌐 Network
			</button>
		</div>
	</div>

	<!-- Results Count -->
	<div class="text-white/60">
		Showing {filteredOfferings.length} of {mockMarketplace.length} offerings
	</div>

	<!-- Marketplace Grid -->
	<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
		{#each filteredOfferings as offering}
			<div
				class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20 hover:border-blue-400 transition-all group"
			>
				<div class="flex items-start justify-between mb-4">
					<span class="text-4xl">{getTypeIcon(offering.type)}</span>
					{#if offering.available}
						<span
							class="inline-flex items-center gap-1 px-3 py-1 rounded-full text-xs font-medium bg-green-500/20 text-green-400 border border-green-500/30"
						>
							<span class="w-2 h-2 rounded-full bg-current"></span>
							Available
						</span>
					{:else}
						<span
							class="inline-flex items-center gap-1 px-3 py-1 rounded-full text-xs font-medium bg-gray-500/20 text-gray-400 border border-gray-500/30"
						>
							Unavailable
						</span>
					{/if}
				</div>

				<h3 class="text-xl font-bold text-white mb-1 group-hover:text-blue-400 transition-colors">
					{offering.name}
				</h3>
				<p class="text-white/60 text-sm mb-4">{offering.provider}</p>

				<div class="space-y-2 text-sm mb-4">
					<div class="flex items-center justify-between text-white/70">
						<span>Type</span>
						<span class="text-white font-medium">{offering.type}</span>
					</div>
					<div class="flex items-center justify-between text-white/70">
						<span>Price</span>
						<span class="text-white font-medium">{offering.price} DCT/hr</span>
					</div>
					<div class="flex items-center justify-between text-white/70">
						<span>Rating</span>
						<span class="text-white font-medium flex items-center gap-1">
							⭐ {offering.rating}
						</span>
					</div>
				</div>

				<div class="text-white/60 text-sm mb-4 p-3 bg-white/5 rounded-lg">
					{offering.specs}
				</div>

				<button
					disabled={!offering.available}
					class="w-full px-4 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold hover:brightness-110 hover:scale-105 transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100"
				>
					{offering.available ? 'Deploy Now' : 'Not Available'}
				</button>
			</div>
		{/each}
	</div>

	<!-- Empty State -->
	{#if filteredOfferings.length === 0}
		<div class="text-center py-16">
			<span class="text-6xl mb-4 block">🔍</span>
			<h3 class="text-2xl font-bold text-white mb-2">No Results Found</h3>
			<p class="text-white/60">Try adjusting your search or filters</p>
		</div>
	{/if}
</div>
