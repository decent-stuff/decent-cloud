<script lang="ts">
	import Icon, { type IconName } from '$lib/components/Icons.svelte';
	import type { Offering } from '$lib/services/api';

	interface Props {
		offering: Partial<Offering>;
	}

	let { offering }: Props = $props();

	function getTypeIcon(productType: string | undefined): IconName {
		const type = (productType ?? '').toLowerCase();
		if (type.includes('gpu')) return 'gpu';
		if (type.includes('compute') || type.includes('vm')) return 'cpu';
		if (type.includes('storage')) return 'hard-drive';
		if (type.includes('network') || type.includes('cdn')) return 'globe';
		return 'package';
	}

	const specs = $derived.by(() => {
		const type = (offering.product_type ?? '').toLowerCase();
		if (type.includes('gpu')) {
			const parts = [
				offering.gpu_name,
				offering.gpu_count ? `${offering.gpu_count}x` : null,
				offering.gpu_memory_gb ? `${offering.gpu_memory_gb}GB` : null
			].filter(Boolean);
			return parts.join(' ') || null;
		}
		const parts = [
			offering.processor_cores ? `${offering.processor_cores} vCPU` : null,
			offering.memory_amount ?? null,
			offering.total_ssd_capacity ? `${offering.total_ssd_capacity} SSD` : null
		].filter(Boolean);
		return parts.length > 0 ? parts.join(' · ') : null;
	});

	const location = $derived.by(() => {
		if (offering.datacenter_city && offering.datacenter_country) {
			return `${offering.datacenter_city}, ${offering.datacenter_country}`;
		}
		return offering.datacenter_country ?? null;
	});

	const priceDisplay = $derived.by(() => {
		if (offering.monthly_price && offering.monthly_price > 0) {
			return `${offering.monthly_price.toFixed(2)} ${offering.currency ?? 'USD'}/mo`;
		}
		return null;
	});

	const descriptionTruncated = $derived.by(() => {
		const d = offering.description;
		if (!d) return null;
		return d.length > 120 ? d.slice(0, 120) + '…' : d;
	});
</script>

<div class="space-y-3">
	<div class="flex items-center gap-2 text-xs text-neutral-500 uppercase tracking-wide">
		<Icon name="eye" size={14} class="text-neutral-600" />
		Marketplace preview
	</div>

	<div class="card border border-neutral-700/60 p-4 opacity-90 relative overflow-hidden">
		<!-- Preview watermark -->
		<div class="absolute top-2 right-2">
			<span class="px-1.5 py-0.5 text-xs bg-primary-500/20 text-primary-400 border border-primary-500/30 rounded font-medium">
				Preview
			</span>
		</div>

		<!-- Offering name row -->
		<div class="flex items-start gap-2 pr-16 mb-2">
			<div>
				{#if offering.offer_name?.trim()}
					<div class="font-medium text-white text-sm leading-snug">{offering.offer_name.trim()}</div>
				{:else}
					<div class="font-medium text-neutral-600 italic text-sm leading-snug">Untitled offering</div>
				{/if}
				<div class="text-xs text-neutral-500 mt-0.5 font-mono">
					{offering.offering_id?.trim() || '—'}
				</div>
			</div>
		</div>

		<!-- Type + badges row -->
		<div class="flex flex-wrap items-center gap-2 mb-3">
			{#if offering.product_type}
				<span class="inline-flex items-center gap-1 text-xs text-neutral-300">
					<Icon name={getTypeIcon(offering.product_type)} size={14} />
					{offering.product_type}
				</span>
			{/if}
			{#if offering.post_provision_script?.trim()}
				<span class="px-1.5 py-0.5 text-xs bg-blue-500/20 text-blue-400 rounded">Recipe</span>
			{/if}
			{#if offering.is_draft}
				<span class="px-1.5 py-0.5 text-xs bg-amber-500/20 text-amber-400 rounded">Draft</span>
			{/if}
		</div>

		<!-- Specs + location row -->
		{#if specs || location}
			<div class="flex flex-wrap gap-x-4 gap-y-1 mb-3 text-xs text-neutral-400">
				{#if specs}
					<span class="flex items-center gap-1">
						<Icon name="cpu" size={12} class="text-neutral-600" />
						{specs}
					</span>
				{/if}
				{#if location}
					<span class="flex items-center gap-1">
						<Icon name="globe" size={12} class="text-neutral-600" />
						{location}
					</span>
				{/if}
			</div>
		{/if}

		<!-- Description -->
		{#if descriptionTruncated}
			<p class="text-xs text-neutral-500 mb-3 leading-relaxed">{descriptionTruncated}</p>
		{:else}
			<p class="text-xs text-neutral-700 italic mb-3">No description</p>
		{/if}

		<!-- Price + action row -->
		<div class="flex items-center justify-between pt-2 border-t border-neutral-800">
			<div>
				{#if priceDisplay}
					<span class="text-sm font-semibold text-white">{priceDisplay}</span>
				{:else}
					<span class="text-sm text-neutral-600 italic">Price not set</span>
				{/if}
				{#if offering.setup_fee && offering.setup_fee > 0}
					<span class="text-xs text-neutral-500 ml-1">+ {offering.setup_fee.toFixed(2)} setup</span>
				{/if}
			</div>
			<span class="px-3 py-1 bg-neutral-800 text-neutral-500 rounded text-xs cursor-not-allowed border border-neutral-700">
				Rent
			</span>
		</div>
	</div>
</div>
