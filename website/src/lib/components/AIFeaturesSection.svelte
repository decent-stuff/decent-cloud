<script lang="ts">
	import Icon from './Icons.svelte';
	import type { IconName } from './Icons.svelte';

	interface Feature {
		title: string;
		description: string;
	}

	interface Category {
		category: string;
		icon: IconName;
		features: Feature[];
	}

	const aiFeatures: Category[] = [
		{
			category: 'For Users',
			icon: 'cpu',
			features: [
				{
					title: 'AI Architecture Advisor',
					description:
						'Describe your workload in plain language. Our AI recommends the right infrastructure, services, configuration, and providers based on your needs and budget.'
				},
				{
					title: 'Intelligent Provider Matching',
					description:
						'Compare providers intelligently with AI-powered ranking that considers performance, reputation, pricing, and your specific requirements.'
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
			icon: 'target',
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

<section class="py-24 px-6 bg-surface/50">
	<div class="max-w-6xl mx-auto">
		<!-- Section header -->
		<div class="text-center mb-16">
			<div class="inline-block px-3 py-1 mb-4 text-xs uppercase tracking-widest text-primary-400 border border-primary-500/30 bg-primary-500/5">
				Coming Soon
			</div>
			<h2 class="text-3xl md:text-4xl font-bold text-white mb-4">
				AI-Powered Cloud Intelligence
			</h2>
			<p class="text-neutral-400 max-w-2xl mx-auto">
				Transform how you interact with cloud infrastructure and services.
				Our AI assistant eliminates complexity, uncertainty, and manual decision-making.
			</p>
		</div>

		<!-- Two-column layout -->
		<div class="grid grid-cols-1 lg:grid-cols-2 gap-8">
			{#each aiFeatures as category, catIndex}
				<div
					class="bg-surface border border-neutral-800 p-8"
					style="animation: slide-up 0.5s ease-out {catIndex * 0.15}s both"
				>
					<!-- Category header -->
					<div class="flex items-center gap-4 mb-8 pb-6 border-b border-neutral-800">
						<div class="w-12 h-12 bg-surface-elevated border border-neutral-700 flex items-center justify-center">
							<Icon name={category.icon} size={22} class="text-primary-400" />
						</div>
						<h3 class="text-2xl font-semibold text-white">
							{category.category}
						</h3>
					</div>

					<!-- Features list -->
					<div class="space-y-6">
						{#each category.features as feature}
							<div class="border-l-2 border-primary-500/30 pl-4 hover:border-primary-500 transition-colors">
								<h4 class="text-base font-semibold text-white mb-1">
									{feature.title}
								</h4>
								<p class="text-sm text-neutral-400 leading-relaxed">
									{feature.description}
								</p>
							</div>
						{/each}
					</div>
				</div>
			{/each}
		</div>

		<!-- GPU callout -->
		<div class="mt-8 bg-surface border border-neutral-800 p-8">
			<div class="flex flex-col md:flex-row items-center gap-6">
				<div class="w-16 h-16 bg-surface-elevated border border-neutral-700 flex items-center justify-center shrink-0">
					<Icon name="zap" size={28} class="text-primary-400" />
				</div>
				<div class="text-center md:text-left">
					<h3 class="text-xl font-semibold text-white mb-2">
						Coming Soon: GPU & AI Workloads
					</h3>
					<p class="text-neutral-400">
						Access high-performance GPUs for machine learning, AI inference, and compute-intensive workloads.
						AI-powered workload optimization ensures you get the right GPU at the right price.
					</p>
				</div>
			</div>
		</div>
	</div>
</section>
