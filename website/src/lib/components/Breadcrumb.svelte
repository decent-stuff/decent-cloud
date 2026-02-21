<script lang="ts">
	interface BreadcrumbItem {
		label: string;
		href?: string;
	}

	let { items }: { items: BreadcrumbItem[] } = $props();

	// On mobile show only the last 2 items to avoid overflow
	const visibleItems = $derived(items.length > 2 ? items.slice(-2) : items);
</script>

<!-- Desktop: all items; Mobile: last 2 items -->
<nav aria-label="Breadcrumb" class="text-sm text-neutral-500">
	<!-- Desktop -->
	<ol class="hidden md:flex items-center flex-wrap gap-1">
		{#each items as item, i}
			{#if i > 0}
				<li aria-hidden="true" class="text-neutral-700">›</li>
			{/if}
			<li>
				{#if item.href}
					<a href={item.href} class="hover:text-white transition-colors">{item.label}</a>
				{:else}
					<span class="text-white">{item.label}</span>
				{/if}
			</li>
		{/each}
	</ol>
	<!-- Mobile: last 2 items -->
	<ol class="flex md:hidden items-center flex-wrap gap-1">
		{#if items.length > 2}
			<li class="text-neutral-700">…</li>
			<li aria-hidden="true" class="text-neutral-700">›</li>
		{/if}
		{#each visibleItems as item, i}
			{#if i > 0}
				<li aria-hidden="true" class="text-neutral-700">›</li>
			{/if}
			<li>
				{#if item.href}
					<a href={item.href} class="hover:text-white transition-colors">{item.label}</a>
				{:else}
					<span class="text-white">{item.label}</span>
				{/if}
			</li>
		{/each}
	</ol>
</nav>
