<script lang="ts">
	import { onMount } from 'svelte';
	import { fetchMetadata, fetchDctPrice } from '$lib/services/icp';
	import { getMetadataValue } from '$lib/utils/metadata.ts';

	interface DashboardData {
		dctPrice: number;
		providerCount: number;
		totalBlocks: number;
		blocksUntilHalving: number;
		validatorCount: number;
		blockReward: number;
	}

	let dashboardData: DashboardData = {
		dctPrice: 0,
		providerCount: 0,
		totalBlocks: 0,
		blocksUntilHalving: 0,
		validatorCount: 0,
		blockReward: 0
	};

	async function loadDashboardData() {
		try {
			const [metadata, dctPrice] = await Promise.all([fetchMetadata(), fetchDctPrice()]);

			dashboardData = {
				dctPrice,
				providerCount: getMetadataValue(metadata, 'ledger:total_providers'),
				totalBlocks: getMetadataValue(metadata, 'ledger:num_blocks'),
				blocksUntilHalving: getMetadataValue(metadata, 'ledger:blocks_until_next_halving'),
				validatorCount: getMetadataValue(metadata, 'ledger:current_block_validators'),
				blockReward: getMetadataValue(metadata, 'ledger:current_block_rewards_e9s')
			};
		} catch (err) {
			console.error('Error fetching dashboard data:', err);
		}
	}

	onMount(() => {
		loadDashboardData();
		const interval = setInterval(loadDashboardData, 10000);
		return () => clearInterval(interval);
	});
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">Dashboard Overview</h1>
		<p class="text-white/60">Welcome to your Decent Cloud dashboard</p>
	</div>

	<!-- Stats Grid -->
	<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
		<!-- DCT Price -->
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
			<div class="flex items-center justify-between mb-2">
				<h3 class="text-white/70 text-sm font-medium">DCT Price</h3>
				<span class="text-2xl">💰</span>
			</div>
			<p class="text-3xl font-bold text-white">${dashboardData.dctPrice.toFixed(4)}</p>
			<p class="text-white/50 text-sm mt-1">USD per token</p>
		</div>

		<!-- Total Providers -->
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
			<div class="flex items-center justify-between mb-2">
				<h3 class="text-white/70 text-sm font-medium">Providers</h3>
				<span class="text-2xl">🖥️</span>
			</div>
			<p class="text-3xl font-bold text-white">{dashboardData.providerCount}</p>
			<p class="text-white/50 text-sm mt-1">Active providers</p>
		</div>

		<!-- Total Blocks -->
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
			<div class="flex items-center justify-between mb-2">
				<h3 class="text-white/70 text-sm font-medium">Blocks</h3>
				<span class="text-2xl">🔗</span>
			</div>
			<p class="text-3xl font-bold text-white">{dashboardData.totalBlocks.toLocaleString()}</p>
			<p class="text-white/50 text-sm mt-1">Total blocks</p>
		</div>

		<!-- Validators -->
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
			<div class="flex items-center justify-between mb-2">
				<h3 class="text-white/70 text-sm font-medium">Validators</h3>
				<span class="text-2xl">✓</span>
			</div>
			<p class="text-3xl font-bold text-white">{dashboardData.validatorCount}</p>
			<p class="text-white/50 text-sm mt-1">Active validators</p>
		</div>

		<!-- Block Reward -->
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
			<div class="flex items-center justify-between mb-2">
				<h3 class="text-white/70 text-sm font-medium">Block Reward</h3>
				<span class="text-2xl">🎁</span>
			</div>
			<p class="text-3xl font-bold text-white">{dashboardData.blockReward.toFixed(2)}</p>
			<p class="text-white/50 text-sm mt-1">DCT per block</p>
		</div>

		<!-- Halving -->
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
			<div class="flex items-center justify-between mb-2">
				<h3 class="text-white/70 text-sm font-medium">Until Halving</h3>
				<span class="text-2xl">⏰</span>
			</div>
			<p class="text-3xl font-bold text-white">
				{dashboardData.blocksUntilHalving.toLocaleString()}
			</p>
			<p class="text-white/50 text-sm mt-1">Blocks remaining</p>
		</div>
	</div>

	<!-- Quick Actions -->
	<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
		<h2 class="text-2xl font-bold text-white mb-4">Quick Actions</h2>
		<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
			<a
				href="/dashboard/marketplace"
				class="p-4 bg-gradient-to-r from-blue-500/20 to-purple-600/20 rounded-lg border border-blue-500/30 hover:border-blue-400 transition-all group"
			>
				<span class="text-3xl mb-2 block">🛒</span>
				<h3 class="text-white font-semibold mb-1 group-hover:text-blue-400">Browse Marketplace</h3>
				<p class="text-white/60 text-sm">Find cloud services</p>
			</a>

			<a
				href="/dashboard/offerings"
				class="p-4 bg-gradient-to-r from-purple-500/20 to-pink-600/20 rounded-lg border border-purple-500/30 hover:border-purple-400 transition-all group"
			>
				<span class="text-3xl mb-2 block">📦</span>
				<h3 class="text-white font-semibold mb-1 group-hover:text-purple-400">Manage Offerings</h3>
				<p class="text-white/60 text-sm">Your cloud services</p>
			</a>

			<a
				href="/dashboard/validators"
				class="p-4 bg-gradient-to-r from-green-500/20 to-teal-600/20 rounded-lg border border-green-500/30 hover:border-green-400 transition-all group"
			>
				<span class="text-3xl mb-2 block">✓</span>
				<h3 class="text-white font-semibold mb-1 group-hover:text-green-400">View Validators</h3>
				<p class="text-white/60 text-sm">Network participants</p>
			</a>
		</div>
	</div>
</div>
