<script lang="ts">
	import { themeStore } from '$lib/stores/theme';
	import Icon from './Icons.svelte';

	let current = $state<'dark' | 'light'>('dark');

	themeStore.subscribe((t) => {
		current = t;
	});
</script>

<button
	type="button"
	onclick={() => themeStore.toggle()}
	class="text-neutral-400 p-2 hover:bg-surface-hover hover:text-neutral-100 transition-colors"
	aria-label="Toggle theme"
	title={current === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'}
>
	{#if current === 'dark'}
		<Icon name="sun" size={20} />
	{:else}
		<Icon name="moon" size={20} />
	{/if}
</button>
