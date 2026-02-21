<script lang="ts">
	import { onMount } from 'svelte';
	import Icon from './Icons.svelte';
	import type { IconName } from './Icons.svelte';
	import { fetchIcpPrice } from '$lib/services/api';

	interface Props {
		dashboardData: {
			totalProviders: number;
			activeProviders: number;
			totalOfferings: number;
			totalContracts: number;
			activeValidators: number;
			totalTransfers: number;
			totalVolumeE9s: number;
		};
		error?: string | null;
	}

	let { dashboardData, error = null }: Props = $props();
	let icpPriceUsd = $state<number | null>(null);

	onMount(async () => {
		icpPriceUsd = await fetchIcpPrice();
	});

	const stats: {
		label: string;
		key: keyof Props['dashboardData'];
		icon: IconName;
		format?: (v: number) => string;
		usdHint?: (v: number, price: number | null) => string | null;
	}[] = [
		{ label: 'Total Providers', key: 'totalProviders', icon: 'server' },
		{ label: 'Active Providers', key: 'activeProviders', icon: 'activity' },
		{ label: 'Available Offerings', key: 'totalOfferings', icon: 'package' },
		{ label: 'Total Contracts', key: 'totalContracts', icon: 'file' },
		{ label: 'Active Validators', key: 'activeValidators', icon: 'shield' },
		{ label: 'Total Transfers', key: 'totalTransfers', icon: 'arrow-right' },
		{
			label: 'Total Volume (ICP)',
			key: 'totalVolumeE9s',
			icon: 'star',
			format: (v) => Math.floor(v / 1_000_000_000).toLocaleString(),
			usdHint: (v, price) =>
				price ? `≈ $${Math.floor((v / 1_000_000_000) * price).toLocaleString()}` : null,
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
				Real-time marketplace activity and growth
			</p>
		</div>

		{#if error}
			<div class="mb-8 bg-danger/10 border border-danger/20 p-4 text-center">
				<p class="font-medium text-danger text-sm">Error loading statistics</p>
				<p class="text-xs text-neutral-400 mt-1">{error}</p>
			</div>
		{/if}

		<!-- Stats grid -->
		<div class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-7 gap-3">
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
					{#if stat.usdHint}
						{@const hint = stat.usdHint(dashboardData[stat.key], icpPriceUsd)}
						{#if hint}
							<div class="text-[10px] text-neutral-500 mb-1">{hint}</div>
						{/if}
					{/if}
					<div class="text-[10px] uppercase tracking-label text-neutral-500">
						{stat.label}
					</div>
				</div>
			{/each}
		</div>

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
