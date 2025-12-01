<script lang="ts">
	interface Props {
		count: number;
	}

	let { count }: Props = $props();

	const displayCount = $derived(count > 99 ? '99+' : count.toString());
	const shouldShow = $derived(count > 0);
</script>

{#if shouldShow}
	<span
		class="inline-flex items-center justify-center min-w-5 h-5 px-1.5 rounded-full bg-red-500 text-white text-xs font-bold"
	>
		{displayCount}
	</span>
{/if}
