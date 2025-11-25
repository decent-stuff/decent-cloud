<script lang="ts">
	interface Props {
		dashboardData: {
			totalProviders: number;
			activeProviders: number;
			totalOfferings: number;
			totalContracts: number;
			activeValidators: number;
		};
		error?: string | null;
	}

	let { dashboardData, error = null }: Props = $props();

	const stats = $derived([
		{
			label: "Total Providers",
			value: dashboardData.totalProviders.toLocaleString(),
			icon: "🖥️",
		},
		{
			label: "Active Providers",
			value: dashboardData.activeProviders.toLocaleString(),
			icon: "✅",
		},
		{
			label: "Available Offerings",
			value: dashboardData.totalOfferings.toLocaleString(),
			icon: "📦",
		},
		{
			label: "Total Contracts",
			value: dashboardData.totalContracts.toLocaleString(),
			icon: "📝",
		},
		{
			label: "Active Validators",
			value: dashboardData.activeValidators.toLocaleString(),
			icon: "🛡️",
		},
	]);
</script>

<section class="py-20 px-4">
	<div class="max-w-7xl mx-auto">
		<h2 class="text-4xl md:text-5xl font-bold text-center mb-4">
			Marketplace Statistics
		</h2>
		<p class="text-xl text-white/70 text-center mb-16">
			Real-time marketplace activity and growth
		</p>

		{#if error}
			<div class="mb-8 bg-red-500/20 border border-red-500/30 rounded-lg p-4 text-red-400 text-center">
				<p class="font-semibold">Error loading statistics</p>
				<p class="text-sm mt-1">{error}</p>
			</div>
		{/if}

		<div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-5 gap-6">
			{#each stats as stat}
				<div
					class="bg-white/10 backdrop-blur-lg rounded-xl p-6 text-center hover:bg-white/20 transition-all"
				>
					<div class="text-4xl mb-2">{stat.icon}</div>
					<div class="text-2xl font-bold mb-1">{stat.value}</div>
					<div class="text-sm text-white/60">{stat.label}</div>
				</div>
			{/each}
		</div>

		<div class="mt-12 text-center">
			<a
				href="/dashboard/marketplace"
				class="inline-block px-8 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold hover:from-blue-600 hover:to-purple-700 transition-all"
			>
				View Full Dashboard
			</a>
		</div>
	</div>
</section>
