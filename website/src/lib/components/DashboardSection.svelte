<script lang="ts">
	import Icon from './Icons.svelte';
	import type { IconName } from './Icons.svelte';

	interface Props {
		dashboardData: {
			totalProviders: number;
			activeProviders: number;
			totalOfferings: number;
			totalContracts: number;
			activeValidators: number;
		};
		error?: string | null;
	}

	let { dashboardData, error = null }: Props = $props();

	const stats: { label: string; key: keyof Props['dashboardData']; icon: IconName }[] = [
		{ label: 'Total Providers', key: 'totalProviders', icon: 'server' },
		{ label: 'Active Providers', key: 'activeProviders', icon: 'activity' },
		{ label: 'Available Offerings', key: 'totalOfferings', icon: 'package' },
		{ label: 'Total Contracts', key: 'totalContracts', icon: 'file' },
		{ label: 'Active Validators', key: 'activeValidators', icon: 'shield' }
	];
</script>

<section class="py-24 px-6">
	<div class="max-w-6xl mx-auto">
		<!-- Section header -->
		<div class="text-center mb-16">
			<h2 class="text-3xl md:text-4xl font-bold text-white mb-4">
				Marketplace Statistics
			</h2>
			<p class="text-neutral-400">
				Real-time marketplace activity and growth
			</p>
		</div>

		{#if error}
			<div class="mb-8 bg-danger/10 border border-danger/30 p-4 text-danger text-center">
				<p class="font-semibold">Error loading statistics</p>
				<p class="text-sm mt-1 opacity-80">{error}</p>
			</div>
		{/if}

		<!-- Stats grid -->
		<div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-5 gap-4">
			{#each stats as stat, i}
				<div
					class="bg-surface border border-neutral-800 p-6 text-center hover:border-neutral-700 transition-colors"
					style="animation: slide-up 0.5s ease-out {i * 0.08}s both"
				>
					<div class="flex justify-center mb-3">
						<div class="w-10 h-10 bg-surface-elevated border border-neutral-700 flex items-center justify-center">
							<Icon name={stat.icon} size={18} class="text-primary-400" />
						</div>
					</div>
					<div class="text-2xl font-bold text-white font-mono tabular-nums mb-1">
						{dashboardData[stat.key].toLocaleString()}
					</div>
					<div class="text-xs uppercase tracking-wider text-neutral-500">
						{stat.label}
					</div>
				</div>
			{/each}
		</div>

		<!-- CTA -->
		<div class="mt-12 text-center">
			<a
				href="/dashboard/marketplace"
				class="inline-flex items-center gap-3 px-6 py-3 bg-primary-500 text-base font-semibold hover:bg-primary-400 transition-colors"
			>
				<span>View Full Dashboard</span>
				<Icon name="arrow-right" size={18} />
			</a>
		</div>
	</div>
</section>
