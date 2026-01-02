<script lang="ts">
	import Icon from './Icons.svelte';
	import type { IconName } from './Icons.svelte';

	const features: { icon: IconName; title: string; description: string }[] = [
		{
			icon: 'shield',
			title: 'Trust Score System',
			description:
				'Every provider has a 0-100 trust score calculated from real data: delivery speed, completion rate, response time, and customer patterns.'
		},
		{
			icon: 'alert',
			title: 'Red Flag Detection',
			description:
				'Get warned before contracting: high cancellation rates, slow responses, low satisfaction, and more.'
		},
		{
			icon: 'wallet',
			title: 'Escrow & Refunds',
			description:
				'Your payments are safe in escrow, and are released incrementally. Cancel anytime with prorated refunds.'
		},
		{
			icon: 'eye',
			title: 'Full Transparency',
			description:
				'See every provider\'s contract history, repeat customer count, and time-to-delivery. All based on objective metrics, from real transactions.'
		}
	];
</script>

<section id="features" class="py-24 px-6">
	<div class="max-w-6xl mx-auto">
		<!-- Section header -->
		<div class="text-center mb-16">
			<div class="inline-block px-3 py-1 mb-4 text-xs uppercase tracking-widest text-primary-400 border border-primary-500/30 bg-primary-500/5">
				Features
			</div>
			<h2 class="text-3xl md:text-4xl font-bold text-white mb-4">
				Why Decent Cloud?
			</h2>
			<p class="text-neutral-400 max-w-2xl mx-auto">
				A marketplace where trust is earned, not claimed. Every metric you see is calculated from real transactions.
			</p>
		</div>

		<!-- Features grid -->
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
			{#each features as feature, i}
				<div
					class="group bg-surface border border-neutral-800 p-6 hover:border-neutral-700 transition-colors"
					style="animation: slide-up 0.5s ease-out {i * 0.1}s both"
				>
					<!-- Icon -->
					<div class="w-12 h-12 bg-surface-elevated border border-neutral-700 flex items-center justify-center mb-5 group-hover:border-primary-500/50 transition-colors">
						<Icon name={feature.icon} size={22} class="text-primary-400" />
					</div>

					<!-- Content -->
					<h3 class="text-lg font-semibold text-white mb-2">
						{feature.title}
					</h3>
					<p class="text-sm text-neutral-400 leading-relaxed">
						{feature.description}
					</p>
				</div>
			{/each}
		</div>
	</div>
</section>
