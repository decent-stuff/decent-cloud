<script lang="ts">
	import Icon from '$lib/components/Icons.svelte';

	interface Props {
		ctaHref?: string;
		ctaLabel?: string;
		highlight?: boolean;
	}

	let {
		ctaHref = '/agents#waitlist',
		ctaLabel = 'Start beta',
		highlight = true
	}: Props = $props();

	const bullets: string[] = [
		'1 dedicated agent identity (always-on container)',
		'Up to 20 active agent-hours / month',
		'Up to 3M Claude Sonnet tokens / month',
		'Overage billed at 1.5x Anthropic rates',
		'Unlimited idle and webhook monitoring',
		'Cancel any time, prorated refund on unused days'
	];
</script>

<div
	class="bg-surface border p-8 space-y-6 shadow-layered max-w-md w-full"
	class:border-primary-500={highlight}
	class:border-neutral-800={!highlight}
>
	<!-- Tier name -->
	<div class="space-y-2">
		<div class="section-label">Decent Agents - Beta</div>
		<h3 class="text-2xl font-bold text-white tracking-tight">Single tier</h3>
	</div>

	<!-- Price -->
	<div class="flex items-baseline gap-2 pt-2 border-t border-neutral-800/80">
		<span class="text-4xl font-bold text-white font-mono tabular-nums">CHF 49</span>
		<span class="text-sm text-neutral-500">/ month</span>
	</div>

	<!-- Bullets -->
	<ul class="space-y-3">
		{#each bullets as bullet}
			<li class="flex items-start gap-3 text-sm text-neutral-300">
				<span class="mt-0.5 text-primary-400 shrink-0">
					<Icon name="check" size={20} />
				</span>
				<span>{bullet}</span>
			</li>
		{/each}
	</ul>

	<!-- CTA -->
	<a
		href={ctaHref}
		class="btn-primary w-full inline-flex items-center justify-center gap-2 text-sm"
	>
		<span>{ctaLabel}</span>
		<Icon name="arrow-right" size={20} />
	</a>

	<p class="text-[10px] text-neutral-500 text-center uppercase tracking-label">
		EU-hosted on Hetzner &middot; B2B-first
	</p>
</div>
