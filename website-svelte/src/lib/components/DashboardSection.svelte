<script lang="ts">
	export let dashboardData: {
		dctPrice: number;
		providerCount: number;
		totalBlocks: number;
		blocksUntilHalving: number;
		validatorCount: number;
		blockReward: number;
	};

	const stats = [
		{ label: 'DCT Price', getValue: () => `$${dashboardData.dctPrice.toFixed(4)}`, icon: '💰' },
		{ label: 'Providers', getValue: () => dashboardData.providerCount.toString(), icon: '🖥️' },
		{ label: 'Total Blocks', getValue: () => dashboardData.totalBlocks.toLocaleString(), icon: '⛓️' },
		{
			label: 'Blocks Until Halving',
			getValue: () => dashboardData.blocksUntilHalving.toLocaleString(),
			icon: '📉'
		},
		{ label: 'Validators', getValue: () => dashboardData.validatorCount.toString(), icon: '✅' },
		{ label: 'Block Reward', getValue: () => `${dashboardData.blockReward.toFixed(2)} DCT`, icon: '🎁' }
	];
</script>

<section class="py-20 px-4">
	<div class="max-w-7xl mx-auto">
		<h2 class="text-4xl md:text-5xl font-bold text-center mb-4">Network Statistics</h2>
		<p class="text-xl text-white/70 text-center mb-16">Real-time data from the Decent Cloud network</p>

		<div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-6">
			{#each stats as stat}
				<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 text-center hover:bg-white/20 transition-all">
					<div class="text-4xl mb-2">{stat.icon}</div>
					<div class="text-2xl font-bold mb-1">{stat.getValue()}</div>
					<div class="text-sm text-white/60">{stat.label}</div>
				</div>
			{/each}
		</div>

		<div class="mt-12 text-center">
			<a
				href="/dashboard"
				class="inline-block px-8 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold hover:from-blue-600 hover:to-purple-700 transition-all"
			>
				View Full Dashboard
			</a>
		</div>
	</div>
</section>
