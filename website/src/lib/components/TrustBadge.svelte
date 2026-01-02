<script lang="ts">
	interface Props {
		score: number;
		hasFlags?: boolean;
		compact?: boolean;
	}

	let { score, hasFlags = false, compact = false }: Props = $props();

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
	<div
		class="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium border {getBgColor(score)}"
		title="Trust Score: {score}/100{hasFlags ? ' (has warnings)' : ''}"
	>
		{#if hasFlags}
			<span class="text-red-400">&#x26A0;</span>
		{/if}
		<span class={getScoreColor(score)}>{score}</span>
	</div>
{:else}
	<div
		class="inline-flex items-center gap-2 px-3 py-1 rounded-full text-sm font-medium border {getBgColor(score)}"
	>
		{#if hasFlags}
			<span class="text-red-400">&#x26A0;</span>
		{/if}
		<span class={getScoreColor(score)}>{score}</span>
		<span class="text-neutral-500">{getLabel(score)}</span>
	</div>
{/if}
