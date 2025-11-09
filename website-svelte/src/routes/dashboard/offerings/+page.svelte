<script lang="ts">
	// TODO: Integrate with actual offerings API when available
	interface Offering {
		id: string;
		name: string;
		type: 'VM' | 'Storage' | 'Network';
		status: 'active' | 'paused' | 'pending';
		price: number;
		revenue: number;
		activeUsers: number;
	}

	const mockOfferings: Offering[] = [
		{
			id: '1',
			name: 'Cloud VM Standard',
			type: 'VM',
			status: 'active',
			price: 0.05,
			revenue: 125.5,
			activeUsers: 3
		},
		{
			id: '2',
			name: 'Object Storage',
			type: 'Storage',
			status: 'active',
			price: 0.02,
			revenue: 45.25,
			activeUsers: 5
		},
		{
			id: '3',
			name: 'CDN Edge Node',
			type: 'Network',
			status: 'paused',
			price: 0.08,
			revenue: 0,
			activeUsers: 0
		}
	];

	function getStatusColor(status: string) {
		switch (status) {
			case 'active':
				return 'bg-green-500/20 text-green-400 border-green-500/30';
			case 'paused':
				return 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30';
			case 'pending':
				return 'bg-blue-500/20 text-blue-400 border-blue-500/30';
			default:
				return 'bg-gray-500/20 text-gray-400 border-gray-500/30';
		}
	}

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

	<!-- Stats Summary -->
	<div class="grid grid-cols-1 md:grid-cols-3 gap-6">
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
			<div class="flex items-center justify-between mb-2">
				<h3 class="text-white/70 text-sm font-medium">Active Offerings</h3>
				<span class="text-2xl">📦</span>
			</div>
			<p class="text-3xl font-bold text-white">
				{mockOfferings.filter((o) => o.status === 'active').length}
			</p>
		</div>

		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
			<div class="flex items-center justify-between mb-2">
				<h3 class="text-white/70 text-sm font-medium">Total Revenue</h3>
				<span class="text-2xl">💰</span>
			</div>
			<p class="text-3xl font-bold text-white">
				{mockOfferings.reduce((sum, o) => sum + o.revenue, 0).toFixed(2)} DCT
			</p>
		</div>

		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
			<div class="flex items-center justify-between mb-2">
				<h3 class="text-white/70 text-sm font-medium">Active Users</h3>
				<span class="text-2xl">👥</span>
			</div>
			<p class="text-3xl font-bold text-white">
				{mockOfferings.reduce((sum, o) => sum + o.activeUsers, 0)}
			</p>
		</div>
	</div>

	<!-- Offerings Grid -->
	<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
		{#each mockOfferings as offering}
			<div
				class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20 hover:border-white/40 transition-all group"
			>
				<div class="flex items-start justify-between mb-4">
					<span class="text-4xl">{getTypeIcon(offering.type)}</span>
					<span
						class="inline-flex items-center gap-1 px-3 py-1 rounded-full text-xs font-medium border {getStatusColor(
							offering.status
						)}"
					>
						<span class="w-2 h-2 rounded-full bg-current"></span>
						{offering.status}
					</span>
				</div>

				<h3 class="text-xl font-bold text-white mb-2 group-hover:text-blue-400 transition-colors">
					{offering.name}
				</h3>

				<div class="space-y-2 text-sm">
					<div class="flex items-center justify-between text-white/70">
						<span>Type</span>
						<span class="text-white font-medium">{offering.type}</span>
					</div>
					<div class="flex items-center justify-between text-white/70">
						<span>Price</span>
						<span class="text-white font-medium">{offering.price} DCT/hr</span>
					</div>
					<div class="flex items-center justify-between text-white/70">
						<span>Revenue</span>
						<span class="text-white font-medium">{offering.revenue.toFixed(2)} DCT</span>
					</div>
					<div class="flex items-center justify-between text-white/70">
						<span>Active Users</span>
						<span class="text-white font-medium">{offering.activeUsers}</span>
					</div>
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
						{offering.status === 'active' ? 'Pause' : 'Activate'}
					</button>
				</div>
			</div>
		{/each}
	</div>

	<!-- Empty State (if no offerings) -->
	{#if mockOfferings.length === 0}
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
</div>
