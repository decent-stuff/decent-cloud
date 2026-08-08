<script lang="ts">
	import Icon from './Icons.svelte';
	import type { IconName } from './Icons.svelte';

	const guarantees: { fear: string; solution: string; description: string; icon: IconName }[] = [
		{
			fear: 'I pay and get nothing',
			solution: 'Escrow Protection',
			description:
				'Your payment is held in escrow and released to the provider only on delivery. Disputes are resolved through the platform.',
			icon: 'shield'
		},
		{
			fear: 'The service is garbage',
			solution: 'Trust Score System',
			description:
				'Every provider has a transparent 0-100 trust score based on delivery speed, completion rate, and customer satisfaction.',
			icon: 'star'
		},
		{
			fear: 'Hidden problems emerge later',
			solution: 'Pattern Detection',
			description:
				'We surface unusual cancellation spikes and service degradation from real delivery data, so provider performance stays visible.',
			icon: 'alert'
		},
		{
			fear: "I can't get my money back",
			solution: 'Prorated Refunds',
			description:
				"Cancel anytime and receive a prorated refund for the time you didn't use. Terms depend on the provider's offering.",
			icon: 'wallet'
		}
	];
</script>

<section class="py-28 px-6 bg-surface-elevated/30">
	<div class="max-w-6xl mx-auto">
		<!-- Section header -->
		<div class="text-center mb-16">
			<h2 class="section-title mb-4">
				Your Concerns. <span class="text-gradient">Our Solutions.</span>
			</h2>
			<p class="section-subtitle mx-auto text-center">
				We know what worries you about renting from unknown providers.
				That's why we built a trust system that keeps provider performance visible.
			</p>
		</div>

		<!-- Guarantees grid -->
		<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
			{#each guarantees as item, i}
				<div
					class="feature-card group"
					style="animation: slide-up 0.5s ease-out {i * 0.06}s both"
				>
					<div class="flex gap-3">
						<!-- Icon -->
						<div class="feature-card-icon shrink-0 w-10 h-10 mb-0">
							<Icon name={item.icon} size={20} class="text-primary-400" />
						</div>

						<!-- Content -->
						<div class="space-y-2">
							<!-- Fear (the problem) -->
							<p class="text-xs text-neutral-600">
								"{item.fear}"
							</p>
							<!-- Solution -->
							<h3 class="text-base font-semibold text-white">
								{item.solution}
							</h3>
							<!-- Description -->
							<p class="text-sm text-neutral-500 leading-relaxed">
								{item.description}
							</p>
						</div>
					</div>
				</div>
			{/each}
		</div>

		<!-- Bottom badge -->
		<div class="mt-14 text-center">
			<div class="inline-flex items-center gap-2.5 px-4 py-2.5 bg-surface border border-neutral-800">
				<Icon name="check" size={20} class="text-primary-500" />
				<span class="text-neutral-400 text-xs">
					All provider metrics are calculated from real data, not self-reported
				</span>
			</div>
		</div>
	</div>
</section>
