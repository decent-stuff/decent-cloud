<script lang="ts">
	const benefits = [
		{
			title: 'For Providers',
			items: [
				'Clear guidance on what users expect from good service',
				'AI support agent handles common requests automatically',
				'Build a verifiable track record that attracts customers',
				'Fair system: good providers rise, bad ones get flagged',
				'Access global demand through unified marketplace'
			],
			icon: '🖥️',
			tagline: 'Excellence is rewarded, not just promised',
			warning:
				'Trust scores are earned through real performance. Cutting corners will show in your metrics.'
		},
		{
			title: 'For Users',
			items: [
				'See trust scores, response times, and red flags before buying',
				'Escrow protection: pay only when service is delivered',
				'Automatic prorated refunds if you cancel early',
				'Real-time alerts if provider behavior changes',
				'Full transparency: nothing hidden, all data verifiable'
			],
			icon: '👥',
			tagline: 'Your money is protected, your data is transparent'
		},
		{
			title: 'For Developers',
			items: [
				'Unified API across all providers',
				'Query provider trust metrics programmatically',
				'Liberal open source license (Apache 2.0)',
				'Integrate trust checks into your deployment pipelines',
				'Active community and transparent development'
			],
			icon: '💻',
			tagline: 'Build with confidence, deploy with data'
		}
	];
</script>

<section class="py-20 px-4 bg-black/30">
	<div class="max-w-7xl mx-auto">
		<h2 class="text-4xl md:text-5xl font-bold text-center mb-16">Benefits for Everyone</h2>

		<div class="grid grid-cols-1 md:grid-cols-3 gap-8">
			{#each benefits as benefit}
				<div class="bg-white/10 backdrop-blur-lg rounded-xl p-8">
					<div class="text-6xl mb-4 text-center">{benefit.icon}</div>
					<h3 class="text-2xl font-bold mb-2 text-center">{benefit.title}</h3>
					<p class="text-purple-300 text-sm text-center mb-6 italic">{benefit.tagline}</p>
					<ul class="space-y-3">
						{#each benefit.items as item}
							<li class="flex items-start">
								<span class="text-green-400 mr-2 mt-1">✓</span>
								<span class="text-white/80 text-sm">{item}</span>
							</li>
						{/each}
					</ul>
					{#if benefit.warning}
						<div class="mt-4 pt-4 border-t border-white/10">
							<p class="text-amber-400/80 text-xs flex items-start gap-2">
								<span>⚠️</span>
								<span>{benefit.warning}</span>
							</p>
						</div>
					{/if}
				</div>
			{/each}
		</div>
	</div>
</section>
