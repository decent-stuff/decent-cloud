<script lang="ts">
	import Icon from './Icons.svelte';
	import type { IconName } from './Icons.svelte';
	import { marketplaceIsEmpty } from '$lib/utils/marketplace-empty';

	interface Props {
		dashboardData: {
			totalProviders: number;
			activeProviders: number;
			totalOfferings: number;
			totalContracts: number;
			totalVolumeE9s: number;
		};
		error?: string | null;
	}

	let { dashboardData, error = null }: Props = $props();

	// Genuine emptiness = no fetch error AND no providers/contracts. On a fetch
	// error we don't know whether the marketplace is empty, so we keep showing
	// the stats grid (prior behavior) rather than claiming emptiness.
	const empty = $derived(!error && marketplaceIsEmpty(dashboardData));

	// Honest marketing stats only. "Active Providers" is deliberately omitted:
	// it is a heartbeat ("online now") metric that reads as a dead marketplace
	// (0 next to a non-zero total) on a marketing page. The authenticated
	// dashboard keeps it with an "Online now" annotation. Volume is aggregated
	// from paid contracts and shows $0 until payments settle — that's honest,
	// not a bug.
	const stats: {
		label: string;
		key: keyof Props['dashboardData'];
		icon: IconName;
		format?: (v: number) => string;
	}[] = [
		{ label: 'Total Providers', key: 'totalProviders', icon: 'server' },
		{ label: 'Available Offerings', key: 'totalOfferings', icon: 'package' },
		{ label: 'Total Contracts', key: 'totalContracts', icon: 'file' },
		{
			// Volume is aggregated from contract payments, all now in USD via Stripe.
			label: 'Total Volume (USD)',
			key: 'totalVolumeE9s',
			icon: 'star',
			format: (v) => Math.floor(v / 1_000_000_000).toLocaleString(),
		}
	];
</script>

<section class="py-28 px-6">
	<div class="max-w-6xl mx-auto">
		<!-- Section header -->
		<div class="text-center mb-14">
			<h2 class="section-title mb-3">
				Marketplace Statistics
			</h2>
			<p class="text-neutral-500 text-base">
				{empty
					? 'Metrics will populate here as soon as the first providers list offerings'
					: 'Real-time marketplace activity and growth'}
			</p>
		</div>

		{#if error}
			<div class="mb-8 bg-danger/10 border border-danger/20 p-4 text-center">
				<p class="font-medium text-danger text-sm">Error loading statistics</p>
				<p class="text-xs text-neutral-400 mt-1">{error}</p>
			</div>
		{/if}

		{#if empty}
			<!-- Empty marketplace: honest early-access reframe instead of a misleading all-zero grid. -->
			<div class="max-w-2xl mx-auto border border-neutral-800 bg-surface p-10 text-center">
				<div class="flex justify-center mb-4">
					<div class="icon-box">
						<Icon name="sparkles" size={24} />
					</div>
				</div>
				<h3 class="text-xl font-semibold text-white mb-2">Be Among the First Providers</h3>
				<p class="text-neutral-400 text-sm leading-relaxed max-w-md mx-auto">
					The Decent Cloud marketplace is open and accepting early providers. These
					statistics are computed from real rental activity, so they will appear here as
					soon as offerings are listed and the first contracts complete.
				</p>
			</div>
		{:else}
			<!-- Stats grid -->
			<div class="grid grid-cols-2 md:grid-cols-4 gap-3">
				{#each stats as stat, i}
					<div
						class="metric-card text-center"
						style="animation: slide-up 0.5s ease-out {i * 0.06}s both"
					>
						<div class="flex justify-center mb-3">
							<div class="icon-box">
								<Icon name={stat.icon} size={20} />
							</div>
						</div>
						<div class="metric-value mb-1">
							{stat.format ? stat.format(dashboardData[stat.key]) : dashboardData[stat.key].toLocaleString()}
						</div>
						<div class="text-[10px] uppercase tracking-label text-neutral-500">
							{stat.label}
						</div>
					</div>
				{/each}
			</div>
		{/if}

		<!-- CTA -->
		<div class="mt-12 text-center">
			<a
				href="/dashboard/marketplace"
				class="inline-flex items-center gap-2 px-5 py-2.5 bg-primary-500 text-neutral-900 text-sm font-semibold hover:bg-primary-400 transition-colors"
			>
				<span>View Full Dashboard</span>
				<Icon name="arrow-right" size={20} />
			</a>
		</div>
	</div>
</section>
