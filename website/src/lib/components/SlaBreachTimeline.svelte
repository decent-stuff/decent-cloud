<script lang="ts">
	import type { OfferingSlaTimelineDay } from '$lib/services/api';

	interface Props {
		timeline?: OfferingSlaTimelineDay[];
		days?: number;
	}

	let { timeline = [], days = 30 }: Props = $props();

	function formatDay(date: Date): string {
		return date.toISOString().slice(0, 10);
	}

	const renderedDays = $derived.by(() => {
		const byDate = new Map(timeline.map((entry) => [entry.date, entry]));
		const today = new Date();
		today.setUTCHours(0, 0, 0, 0);

		return Array.from({ length: days }, (_, index) => {
			const date = new Date(today);
			date.setUTCDate(today.getUTCDate() - (days - index - 1));
			const key = formatDay(date);
			const entry = byDate.get(key);
			return {
				date: key,
				entry,
				title: entry
					? `${key}: ${entry.breached ? 'SLA breach' : 'Within SLA'}${entry.uptimePercent != null ? `, uptime ${entry.uptimePercent.toFixed(2)}%` : ''}${entry.incidentCount > 0 ? `, incidents ${entry.incidentCount}` : ''}`
					: `${key}: no provider-reported SLI data`
			};
		});
	});
</script>

<div class="space-y-2">
	<div class="flex items-end gap-1.5">
		{#each renderedDays as day}
			<div class="flex flex-col items-center gap-1">
				<div
					class={`w-1.5 h-10 rounded-full ${day.entry ? (day.entry.breached ? 'bg-red-400' : 'bg-emerald-400') : 'bg-neutral-700'}`}
					title={day.title}
				></div>
			</div>
		{/each}
	</div>
	<div class="flex items-center justify-between text-[11px] text-neutral-500">
		<span>{renderedDays[0]?.date}</span>
		<span>Last {days} days</span>
		<span>{renderedDays[renderedDays.length - 1]?.date}</span>
	</div>
	<div class="flex flex-wrap items-center gap-3 text-xs text-neutral-400">
		<span class="inline-flex items-center gap-1.5"><span class="h-2 w-2 rounded-full bg-emerald-400"></span>Within SLA</span>
		<span class="inline-flex items-center gap-1.5"><span class="h-2 w-2 rounded-full bg-red-400"></span>Breach</span>
		<span class="inline-flex items-center gap-1.5"><span class="h-2 w-2 rounded-full bg-neutral-700"></span>No report</span>
	</div>
</div>
