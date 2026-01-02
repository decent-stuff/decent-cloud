<script lang="ts">
	import Icon from './Icons.svelte';
	import type { IconName } from './Icons.svelte';

	interface Benefit {
		title: string;
		items: string[];
		icon: IconName;
		tagline: string;
		note?: string;
	}

	const benefits: Benefit[] = [
		{
			title: 'For Providers',
			items: [
				'Clear guidance on what users expect from good service',
				'AI support agent handles common requests automatically',
				'Build a verifiable track record that attracts customers',
				'Fair system: good providers rise, bad ones get flagged',
				'Access global demand through unified marketplace',
				'Free support stack included'
			],
			icon: 'server',
			tagline: 'Excellence is rewarded, not just promised',
			note: 'Trust scores are earned through real performance. Cutting corners will show in your metrics.'
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
			icon: 'users',
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
			icon: 'code',
			tagline: 'Build with confidence, deploy with data'
		}
	];
</script>

<section class="py-28 px-6">
	<div class="max-w-6xl mx-auto">
		<!-- Section header -->
		<div class="text-center mb-16">
			<h2 class="section-title">
				Benefits for Everyone
			</h2>
		</div>

		<!-- Benefits grid -->
		<div class="grid grid-cols-1 lg:grid-cols-3 gap-4">
			{#each benefits as benefit, i}
				<div
					class="card p-6"
					style="animation: slide-up 0.5s ease-out {i * 0.08}s both"
				>
					<!-- Header -->
					<div class="flex items-center gap-3 mb-5 pb-5 border-b border-neutral-800/80">
						<div class="icon-box">
							<Icon name={benefit.icon} size={18} />
						</div>
						<div>
							<h3 class="text-base font-semibold text-white">
								{benefit.title}
							</h3>
							<p class="text-xs text-neutral-500 mt-0.5">
								{benefit.tagline}
							</p>
						</div>
					</div>

					<!-- Items -->
					<ul class="space-y-2.5">
						{#each benefit.items as item}
							<li class="flex items-start gap-2.5">
								<Icon name="check" size={14} class="text-primary-500 shrink-0 mt-0.5" />
								<span class="text-sm text-neutral-400">{item}</span>
							</li>
						{/each}
					</ul>

					<!-- Note if present -->
					{#if benefit.note}
						<div class="mt-5 pt-4 border-t border-neutral-800/80">
							<div class="flex items-start gap-2.5">
								<Icon name="alert" size={12} class="text-warning shrink-0 mt-0.5" />
								<p class="text-xs text-neutral-500">{benefit.note}</p>
							</div>
						</div>
					{/if}
				</div>
			{/each}
		</div>
	</div>
</section>
