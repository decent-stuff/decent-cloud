<script lang="ts">
	import Icon from './Icons.svelte';

	interface Props {
		score: number;
		hasFlags?: boolean;
		compact?: boolean;
		showTooltip?: boolean;
	}

	let { score, hasFlags = false, compact = false, showTooltip = true }: Props = $props();

	let tooltipVisible = $state(false);

	function getScoreColor(s: number): string {
		if (s >= 80) return 'text-green-400';
		if (s >= 60) return 'text-yellow-400';
		return 'text-red-400';
	}

	function getBgColor(s: number): string {
		if (s >= 80) return 'bg-green-500/20 border-green-500/30';
		if (s >= 60) return 'bg-yellow-500/20 border-yellow-500/30';
		return 'bg-red-500/20 border-red-500/30';
	}

	function getLabel(s: number): string {
		if (s >= 80) return 'Reliable';
		if (s >= 60) return 'Caution';
		return 'Risk';
	}
</script>

{#if compact}
	<div class="relative inline-flex items-center gap-1">
		<div
			class="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium border {getBgColor(score)}"
			title={showTooltip ? undefined : "Trust Score: {score}/100{hasFlags ? ' (has warnings)' : ''}"}
		>
			{#if hasFlags}
				<Icon name="alert" size={20} class="text-red-400" />
			{/if}
			<span class={getScoreColor(score)}>{score}</span>
		</div>
		{#if showTooltip}
			<button
				type="button"
				class="text-neutral-500 hover:text-neutral-300 transition-colors"
				onmouseenter={() => tooltipVisible = true}
				onmouseleave={() => tooltipVisible = false}
				onclick={(e) => { e.stopPropagation(); tooltipVisible = !tooltipVisible; }}
				aria-label="Trust score explanation"
			>
				<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
					<path d="M8 0a8 8 0 100 16A8 8 0 008 0zm0 12a1 1 0 110-2 1 1 0 010 2zm1-3.5V9a1 1 0 00-2 0v-.5a1 1 0 001-1V5a1 1 0 00-2 0 1 1 0 00-2 0 3 3 0 106 0v2.5z"/>
				</svg>
			</button>
			{#if tooltipVisible}
				<div
					class="absolute bottom-full left-0 mb-2 z-50 w-48 bg-neutral-900 border border-neutral-700 rounded p-3 text-xs shadow-xl pointer-events-none"
					role="tooltip"
				>
					<div class="font-semibold text-white mb-1">Trust Score: {score}%</div>
					<div class="text-neutral-400 mb-2 text-xs leading-relaxed">Based on completed contracts.</div>
					<div class="space-y-0.5">
						<div class="flex items-center gap-1.5"><span class="text-green-400">●</span><span class="text-neutral-300">90%+ Excellent</span></div>
						<div class="flex items-center gap-1.5"><span class="text-green-400">●</span><span class="text-neutral-300">80–89% Good</span></div>
						<div class="flex items-center gap-1.5"><span class="text-yellow-400">●</span><span class="text-neutral-300">60–79% Caution</span></div>
						<div class="flex items-center gap-1.5"><span class="text-red-400">●</span><span class="text-neutral-300">&lt;60% High Risk</span></div>
					</div>
				</div>
			{/if}
		{/if}
	</div>
{:else}
	<div class="relative inline-flex items-center gap-2">
		<div
			class="inline-flex items-center gap-2 px-3 py-1 rounded-full text-sm font-medium border {getBgColor(score)}"
		>
			{#if hasFlags}
				<Icon name="alert" size={20} class="text-red-400" />
			{/if}
			<span class={getScoreColor(score)}>{score}</span>
			<span class="text-neutral-500">{getLabel(score)}</span>
		</div>
		{#if showTooltip}
			<button
				type="button"
				class="text-neutral-500 hover:text-neutral-300 transition-colors"
				onmouseenter={() => tooltipVisible = true}
				onmouseleave={() => tooltipVisible = false}
				onclick={(e) => { e.stopPropagation(); tooltipVisible = !tooltipVisible; }}
				aria-label="Trust score explanation"
			>
				<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
					<path d="M8 0a8 8 0 100 16A8 8 0 008 0zm0 12a1 1 0 110-2 1 1 0 010 2zm1-3.5V9a1 1 0 00-2 0v-.5a1 1 0 001-1V5a1 1 0 00-2 0 1 1 0 00-2 0 3 3 0 106 0v2.5z"/>
				</svg>
			</button>
			{#if tooltipVisible}
				<div
					class="absolute bottom-full left-0 mb-2 z-50 w-48 bg-neutral-900 border border-neutral-700 rounded p-3 text-xs shadow-xl pointer-events-none"
					role="tooltip"
				>
					<div class="font-semibold text-white mb-1">Trust Score: {score}%</div>
					<div class="text-neutral-400 mb-2 text-xs leading-relaxed">Based on completed contracts.</div>
					<div class="space-y-0.5">
						<div class="flex items-center gap-1.5"><span class="text-green-400">●</span><span class="text-neutral-300">90%+ Excellent</span></div>
						<div class="flex items-center gap-1.5"><span class="text-green-400">●</span><span class="text-neutral-300">80–89% Good</span></div>
						<div class="flex items-center gap-1.5"><span class="text-yellow-400">●</span><span class="text-neutral-300">60–79% Caution</span></div>
						<div class="flex items-center gap-1.5"><span class="text-red-400">●</span><span class="text-neutral-300">&lt;60% High Risk</span></div>
					</div>
				</div>
			{/if}
		{/if}
	</div>
{/if}
