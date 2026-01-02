<script lang="ts">
	import Icon from './Icons.svelte';
	import type { IconName } from './Icons.svelte';

	const guarantees: { fear: string; solution: string; description: string; icon: IconName }[] = [
		{
			fear: 'I pay and get nothing',
			solution: 'Escrow Protection',
			description:
				'Your payment is held in escrow until the provider delivers. No delivery = automatic refund.',
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
			fear: 'They ghost me when I need help',
			solution: 'Response Time Tracking',
			description:
				'We track how fast providers respond. Slow responders get flagged. You see this BEFORE you buy.',
			icon: 'clock'
		},
		{
			fear: 'Hidden problems emerge later',
			solution: 'Pattern Detection',
			description:
				'Our system detects unusual cancellation spikes and service degradation in real-time. Red flags shown prominently.',
			icon: 'alert'
		},
		{
			fear: "I can't get my money back",
			solution: 'Easy Refunds',
			description:
				"Prorated refunds processed automatically. Cancel anytime, get back what you didn't use.",
			icon: 'wallet'
		},
		{
			fear: 'Other users got burned',
			solution: 'Full Transparency',
			description:
				"See every provider's contract history, cancellation rates, and repeat customer count. Nothing hidden.",
			icon: 'eye'
		}
	];
</script>

<section class="py-24 px-6 bg-surface/50">
	<div class="max-w-6xl mx-auto">
		<!-- Section header -->
		<div class="text-center mb-16">
			<h2 class="text-3xl md:text-4xl font-bold text-white mb-4">
				Your Concerns. <span class="text-gradient">Our Solutions.</span>
			</h2>
			<p class="text-neutral-400 max-w-2xl mx-auto">
				We know what worries you about renting from unknown providers.
				That's why we built a system that makes bad behavior impossible to hide.
			</p>
		</div>

		<!-- Guarantees grid -->
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
			{#each guarantees as item, i}
				<div
					class="bg-surface border border-neutral-800 p-6 hover:border-neutral-700 transition-colors"
					style="animation: slide-up 0.5s ease-out {i * 0.08}s both"
				>
					<div class="flex gap-4">
						<!-- Icon -->
						<div class="shrink-0 w-10 h-10 bg-surface-elevated border border-neutral-700 flex items-center justify-center">
							<Icon name={item.icon} size={18} class="text-primary-400" />
						</div>

						<!-- Content -->
						<div class="space-y-2">
							<!-- Fear (the problem) -->
							<p class="text-sm text-neutral-500 italic">
								"{item.fear}"
							</p>
							<!-- Solution -->
							<h3 class="text-lg font-semibold text-white">
								{item.solution}
							</h3>
							<!-- Description -->
							<p class="text-sm text-neutral-400 leading-relaxed">
								{item.description}
							</p>
						</div>
					</div>
				</div>
			{/each}
		</div>

		<!-- Bottom badge -->
		<div class="mt-12 text-center">
			<div class="inline-flex items-center gap-3 px-5 py-3 bg-surface border border-neutral-800">
				<Icon name="check" size={18} class="text-primary-400" />
				<span class="text-neutral-300 text-sm">
					All provider metrics are calculated from real data, not self-reported
				</span>
			</div>
		</div>
	</div>
</section>
