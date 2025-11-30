<script lang="ts">
	const features = [
		{
			icon: '🛡️',
			title: 'Trust Score System',
			description:
				'Every provider has a 0-100 trust score calculated from real data: delivery speed, completion rate, response time, and customer patterns. No self-reported metrics.'
		},
		{
			icon: '🚨',
			title: 'Red Flag Detection',
			description:
				'Automatic alerts for concerning patterns: sudden cancellation spikes, slow responses, stuck contracts, or inactive providers. You see warnings before you buy.'
		},
		{
			icon: '💸',
			title: 'Escrow & Refunds',
			description:
				'Payments held until delivery. Cancel anytime with automatic prorated refunds. Your money is protected from the moment you click "rent".'
		},
		{
			icon: '👁️',
			title: 'Full Transparency',
			description:
				'See every provider\'s contract history, repeat customer count, and time-to-delivery. All metrics derived from real transactions, not reviews that can be faked.'
		}
	];
</script>

<section id="features" class="py-20 px-4">
	<div class="max-w-7xl mx-auto">
		<h2 class="text-4xl md:text-5xl font-bold text-center mb-4">Why Decent Cloud?</h2>
		<p class="text-xl text-white/70 text-center mb-16">
			A marketplace where trust is earned, not claimed. Every metric you see is calculated from real
			transactions.
		</p>

		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-8">
			{#each features as feature}
				<div
					class="bg-white/10 backdrop-blur-lg rounded-xl p-6 hover:bg-white/20 transition-all hover:scale-105"
				>
					<div class="text-6xl mb-4">{feature.icon}</div>
					<h3 class="text-xl font-bold mb-3">{feature.title}</h3>
					<p class="text-white/70">{feature.description}</p>
				</div>
			{/each}
		</div>
	</div>
</section>
