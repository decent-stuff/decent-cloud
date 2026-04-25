<script lang="ts">
	import Header from '$lib/components/Header.svelte';
	import Footer from '$lib/components/Footer.svelte';
	import Icon from '$lib/components/Icons.svelte';
	import PricingCard from '$lib/components/agents/PricingCard.svelte';

	const exampleRows: Array<{ label: string; value: string; note: string }> = [
		{
			label: 'Active hours',
			value: '~10 / month',
			note: 'Well under the 20-hour cap.'
		},
		{
			label: 'Tokens used',
			value: '~300k / month',
			note: 'About 10% of the 3M cap.'
		},
		{
			label: 'Issues handled',
			value: '~15-25 / month',
			note: 'Mix of small fixes and medium features.'
		},
		{
			label: 'Pull requests opened',
			value: '~12-20 / month',
			note: 'Excludes follow-up commits.'
		}
	];
</script>

<svelte:head>
	<title>Decent Agents pricing - CHF 49/month</title>
	<meta
		name="description"
		content="Decent Agents pricing details, usage assumptions, active-hour definition, token cap and refund policy."
	/>
</svelte:head>

<div class="min-h-screen bg-base text-white">
	<Header />

	<!-- Title -->
	<section class="pt-28 pb-12 px-6 border-b border-neutral-800/80">
		<div class="max-w-4xl mx-auto text-center space-y-4">
			<div class="section-label">Pricing</div>
			<h1 class="text-4xl sm:text-5xl font-bold tracking-display">
				<span class="text-white">CHF 49 / month.</span>
				<span class="text-gradient">That is it.</span>
			</h1>
			<p class="text-lg text-neutral-400 max-w-2xl mx-auto leading-relaxed">
				No tiers, no per-seat add-ons, no surprise overage. Below is exactly what you get and how
				we count it.
			</p>
		</div>
	</section>

	<!-- Pricing card -->
	<section class="py-16 px-6">
		<div class="max-w-md mx-auto">
			<PricingCard ctaHref="/agents#waitlist" />
		</div>
	</section>

	<!-- Typical user example -->
	<section class="py-16 px-6 border-t border-neutral-800/80 bg-surface/30">
		<div class="max-w-3xl mx-auto space-y-8">
			<div class="space-y-2">
				<div class="section-label">Typical user</div>
				<h2 class="section-title">A 5-person team, ~20 issues / month</h2>
				<p class="section-subtitle">
					This is what our beta customers actually look like. Most teams stay well below the caps.
				</p>
			</div>
			<div class="bg-surface border border-neutral-800">
				<table class="w-full text-sm">
					<tbody>
						{#each exampleRows as row, idx}
							<tr
								class="border-neutral-800/80"
								class:border-t={idx > 0}
							>
								<td class="px-5 py-4 text-neutral-400 font-medium w-48">{row.label}</td>
								<td class="px-5 py-4 text-white font-mono tabular-nums">{row.value}</td>
								<td class="px-5 py-4 text-neutral-500 text-xs">{row.note}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</div>
	</section>

	<!-- Definitions -->
	<section class="py-16 px-6 border-t border-neutral-800/80">
		<div class="max-w-3xl mx-auto space-y-10">
			<div class="space-y-4">
				<div class="section-label">Definitions</div>
				<h2 class="section-title">What counts, and how</h2>
			</div>

			<div class="space-y-6">
				<!-- Active hour -->
				<div class="bg-surface border border-neutral-800 p-6 space-y-3">
					<div class="flex items-center gap-3">
						<div class="icon-box">
							<Icon name="clock" size={20} />
						</div>
						<h3 class="text-lg font-semibold text-white">Active agent-hour</h3>
					</div>
					<p class="text-sm text-neutral-400 leading-relaxed">
						An active hour is wall-clock time the agent spends running on a task: planning,
						editing, running tests, opening or updating a PR, responding to review comments.
					</p>
					<p class="text-sm text-neutral-400 leading-relaxed">
						Idle time, webhook polling, waiting for CI, and waiting for human review do
						<span class="text-white">not</span> count. Most issues complete in 5-15 active minutes.
					</p>
				</div>

				<!-- Token cap -->
				<div class="bg-surface border border-neutral-800 p-6 space-y-3">
					<div class="flex items-center gap-3">
						<div class="icon-box">
							<Icon name="cpu" size={20} />
						</div>
						<h3 class="text-lg font-semibold text-white">Token cap and overage</h3>
					</div>
					<p class="text-sm text-neutral-400 leading-relaxed">
						You get up to 3M Claude Sonnet tokens per calendar month, combined input and output.
						You will see a notification at 80%. At 100% you choose, in the dashboard:
					</p>
					<ul class="space-y-2 text-sm text-neutral-400 leading-relaxed pl-4">
						<li class="flex items-start gap-2">
							<span class="text-primary-400 mt-0.5 shrink-0">
								<Icon name="check" size={20} />
							</span>
							<span>
								<span class="text-white font-medium">Pause</span> - the agent stops accepting new
								work until the next billing cycle. Default.
							</span>
						</li>
						<li class="flex items-start gap-2">
							<span class="text-primary-400 mt-0.5 shrink-0">
								<Icon name="check" size={20} />
							</span>
							<span>
								<span class="text-white font-medium">Continue with overage</span> - charged at
								1.5x Anthropic published per-token rates, billed at end of cycle.
							</span>
						</li>
					</ul>
					<p class="text-xs text-neutral-500 leading-relaxed">
						Active-hour cap (20h / month) follows the same pause-or-overage rule. Overage there is
						charged at CHF 4 per active hour.
					</p>
				</div>

				<!-- Refunds -->
				<div class="bg-surface border border-neutral-800 p-6 space-y-3">
					<div class="flex items-center gap-3">
						<div class="icon-box">
							<Icon name="refresh" size={20} />
						</div>
						<h3 class="text-lg font-semibold text-white">Cancellation and refunds</h3>
					</div>
					<p class="text-sm text-neutral-400 leading-relaxed">
						Cancel any time from your dashboard. We refund unused days in the current billing
						period prorated to the day, on the original payment method, within 5 business days.
					</p>
					<p class="text-sm text-neutral-400 leading-relaxed">
						Example: cancel on day 12 of a 30-day cycle and you receive
						<span class="text-white font-mono tabular-nums">CHF 49 x 18 / 30 = CHF 29.40</span>
						back. No questions asked, no exit interview.
					</p>
				</div>
			</div>
		</div>
	</section>

	<!-- Back link -->
	<section class="py-10 px-6 border-t border-neutral-800/80 text-center">
		<a
			href="/agents"
			class="text-sm text-neutral-400 hover:text-white inline-flex items-center gap-1.5 transition-colors"
		>
			<Icon name="arrow-left" size={20} />
			<span>Back to Decent Agents overview</span>
		</a>
	</section>

	<Footer />
</div>
