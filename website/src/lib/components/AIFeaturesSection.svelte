<script lang="ts">
	const aiFeatures = [
		{
			category: 'For Users',
			icon: '🤖',
			features: [
				{
					title: 'AI Architecture Advisor',
					description:
						'Describe your workload in plain language. Our AI recommends the right infrastructure, services, configuration, and providers based on your needs and budget.'
				},
				{
					title: 'Intelligent Provider Matching',
					description:
						"Compare providers intelligently with AI-powered ranking that considers performance, reputation, pricing, and your specific requirements."
				},
				{
					title: 'Cost Optimization',
					description:
						'Get real-time cost projections and optimization suggestions. Understand exactly why one option costs more and receive alerts about over-provisioning.'
				},
				{
					title: 'Workload Migration Assistant',
					description:
						'Migrate from AWS, Azure, or other clouds seamlessly. AI generates equivalent configurations and helps you transition smoothly.'
				}
			]
		},
		{
			category: 'For Providers',
			icon: '🎯',
			features: [
				{
					title: 'Dynamic Pricing Intelligence',
					description:
						'AI-powered pricing recommendations based on real-time demand, competition, and market conditions to maximize your revenue.'
				},
				{
					title: 'Demand Forecasting',
					description:
						'Understand what infrastructure and services are in demand. Get suggestions for profitable offerings based on marketplace trends and usage patterns.'
				},
				{
					title: 'Automated Anomaly Detection',
					description:
						'Protect your infrastructure and services with AI-powered detection of suspicious usage patterns, abuse, and potential security threats.'
				},
				{
					title: 'Smart Support Automation',
					description:
						'Reduce support burden with an AI assistant that handles common questions and troubleshoots issues for your customers.'
				}
			]
		}
	];
</script>

<section class="py-20 px-4 bg-gradient-to-b from-surface/30 to-surface/30">
	<div class="max-w-7xl mx-auto">
		<div class="text-center mb-16">
			<h2 class="text-4xl md:text-5xl font-bold mb-4">AI-Powered Cloud Intelligence</h2>
			<p class="text-xl text-white/70 max-w-3xl mx-auto">
				Soon: Transform how you interact with cloud infrastructure and services. Our AI assistant eliminates
				complexity, uncertainty, and manual decision-making.
			</p>
		</div>

		<div class="grid grid-cols-1 lg:grid-cols-2 gap-12">
			{#each aiFeatures as category}
				<div class="bg-glass/10 backdrop-blur-lg rounded-xl p-8">
					<div class="flex items-center gap-4 mb-6">
						<div class="text-5xl">{category.icon}</div>
						<h3 class="text-3xl font-bold">{category.category}</h3>
					</div>

					<div class="space-y-6">
						{#each category.features as feature}
							<div class="border-l-4 border-purple-400 pl-4">
								<h4 class="text-lg font-semibold mb-2 text-purple-200">{feature.title}</h4>
								<p class="text-white/70 text-sm">{feature.description}</p>
							</div>
						{/each}
					</div>
				</div>
			{/each}
		</div>

		<div class="mt-16 bg-glass/5 backdrop-blur-lg rounded-xl p-8 border border-purple-400/30">
			<div class="flex flex-col md:flex-row items-center gap-6">
				<div class="text-6xl">⚡</div>
				<div class="flex-1 text-center md:text-left">
					<h3 class="text-2xl font-bold mb-2">Coming Soon: GPU & AI Workloads</h3>
					<p class="text-white/70">
						Access high-performance GPUs for machine learning, AI inference, and compute-intensive
						workloads. AI-powered workload optimization ensures you get the right GPU at the right
						price.
					</p>
				</div>
			</div>
		</div>
	</div>
</section>
