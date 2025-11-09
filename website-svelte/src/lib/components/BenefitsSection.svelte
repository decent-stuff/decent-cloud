<script lang="ts">
	const benefits = [
		{
			title: 'For Providers',
			items: [
				'Monetize your idle infrastructure',
				'Set your own pricing',
				'Build reputation in the network',
				'Zero upfront costs'
			],
			icon: '🖥️'
		},
		{
			title: 'For Users',
			items: [
				'Access global infrastructure',
				'Pay only for what you use',
				'Choose from verified providers',
				'No vendor lock-in'
			],
			icon: '👥'
		},
		{
			title: 'For Developers',
			items: [
				'Simple, consistent APIs',
				'Multi-cloud deployments',
				'Liberal open source license',
				'Active community support'
			],
			icon: '💻'
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
					<h3 class="text-2xl font-bold mb-6 text-center">{benefit.title}</h3>
					<ul class="space-y-3">
						{#each benefit.items as item}
							<li class="flex items-start">
								<span class="text-green-400 mr-2">✓</span>
								<span class="text-white/80">{item}</span>
							</li>
						{/each}
					</ul>
				</div>
			{/each}
		</div>
	</div>
</section>
