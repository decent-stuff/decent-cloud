<script lang="ts">
	const userFears = [
		{
			fear: '"I pay and get nothing"',
			solution: 'Escrow Protection',
			description:
				'Your payment is held in escrow until the provider delivers. No delivery = automatic refund.',
			icon: '🛡️'
		},
		{
			fear: '"The service is garbage"',
			solution: 'Trust Score System',
			description:
				'Every provider has a transparent 0-100 trust score based on delivery speed, completion rate, and customer satisfaction.',
			icon: '⭐'
		},
		{
			fear: '"They ghost me when I need help"',
			solution: 'Response Time Tracking',
			description:
				'We track how fast providers respond. Slow responders get flagged. You see this BEFORE you buy.',
			icon: '⏱️'
		},
		{
			fear: '"Hidden problems emerge later"',
			solution: 'Pattern Detection',
			description:
				'Our system detects unusual cancellation spikes and service degradation in real-time. Red flags shown prominently.',
			icon: '🚨'
		},
		{
			fear: '"I can\'t get my money back"',
			solution: 'Easy Refunds',
			description:
				'Prorated refunds processed automatically. Cancel anytime, get back what you didn\'t use.',
			icon: '💸'
		},
		{
			fear: '"Other users got burned"',
			solution: 'Full Transparency',
			description:
				'See every provider\'s contract history, cancellation rates, and repeat customer count. Nothing hidden.',
			icon: '👁️'
		}
	];
</script>

<section class="py-20 px-4">
	<div class="max-w-7xl mx-auto">
		<h2 class="text-4xl md:text-5xl font-bold text-center mb-4">
			Your Fears.
			<span class="text-transparent bg-clip-text bg-gradient-to-r from-green-400 to-blue-500">
				Our Guarantees.
			</span>
		</h2>
		<p class="text-xl text-white/70 text-center mb-16 max-w-3xl mx-auto">
			We know what worries you about renting from unknown providers. That's why we built a system
			that makes bad behavior impossible to hide.
		</p>

		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
			{#each userFears as item}
				<div
					class="bg-gradient-to-br from-white/10 to-white/5 backdrop-blur-lg rounded-xl p-6 border border-white/10 hover:border-green-400/50 transition-all"
				>
					<div class="flex items-start gap-4">
						<div class="text-4xl">{item.icon}</div>
						<div>
							<p class="text-red-400/80 text-sm font-medium mb-1">{item.fear}</p>
							<h3 class="text-xl font-bold text-green-400 mb-2">{item.solution}</h3>
							<p class="text-white/70 text-sm">{item.description}</p>
						</div>
					</div>
				</div>
			{/each}
		</div>

		<div class="mt-12 text-center">
			<div
				class="inline-flex items-center gap-2 px-6 py-3 bg-green-500/20 border border-green-500/50 rounded-full"
			>
				<span class="text-green-400 text-2xl">✓</span>
				<span class="text-green-300 font-medium"
					>All provider metrics are calculated from real data, not self-reported</span
				>
			</div>
		</div>
	</div>
</section>
