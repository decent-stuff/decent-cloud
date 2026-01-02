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

<section id="features" class="py-28 px-6">
	<div class="max-w-6xl mx-auto">
		<!-- Section header -->
		<div class="text-center mb-16">
			<div class="inline-flex items-center gap-2 px-3 py-1.5 mb-5 bg-surface border border-neutral-800">
				<span class="text-[10px] uppercase tracking-label text-primary-400">Features</span>
			</div>
			<h2 class="section-title mb-4">
				Why Decent Cloud?
			</h2>
			<p class="section-subtitle mx-auto text-center">
				A marketplace where trust is earned, not claimed. Every metric you see is calculated from real transactions.
			</p>
		</div>

		<!-- Features grid -->
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
			{#each features as feature, i}
				<div
					class="group feature-card"
					style="animation: slide-up 0.5s ease-out {i * 0.08}s both"
				>
					<!-- Icon -->
					<div class="feature-card-icon">
						<Icon name={feature.icon} size={20} class="text-primary-400" />
					</div>

					<!-- Content -->
					<h3 class="text-base font-semibold text-white mb-2">
						{feature.title}
					</h3>
					<p class="text-sm text-neutral-500 leading-relaxed">
						{feature.description}
					</p>
				</div>
			{/each}
		</div>
	</div>
</section>
