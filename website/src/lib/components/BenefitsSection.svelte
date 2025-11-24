<script lang="ts">
	const benefits = [
		{
			title: 'For Providers',
			items: [
				'Monetize idle infrastructure and offer cloud services',
				'Flexible pricing with AI-powered recommendations',
				'Build verifiable on-chain reputation',
				'Access global demand through unified marketplace',
				'Automated anomaly detection for security'
			],
			icon: '🖥️',
			tagline: 'Turn spare capacity into revenue'
		},
		{
			title: 'For Users',
			items: [
				'Access infrastructure and services from verified providers globally',
				'AI-assisted provider selection and cost optimization',
				'Transparent pricing with no hidden fees',
				'Choose providers based on verified reputation metrics',
				'No vendor lock-in with unified API'
			],
			icon: '👥',
			tagline: 'Deploy anywhere with confidence'
		},
		{
			title: 'For Developers',
			items: [
				'Unified API across all providers',
				'Seamless multi-cloud deployments',
				'Liberal open source license (Apache 2.0)',
				'AI-powered migration from existing clouds',
				'Active community and transparent development'
			],
			icon: '💻',
			tagline: 'Build without constraints'
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
				</div>
			{/each}
		</div>
	</div>
</section>
