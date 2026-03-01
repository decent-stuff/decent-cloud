<script lang="ts">
	import type { HTMLButtonAttributes, HTMLAnchorAttributes } from 'svelte/elements';
	import type { Snippet } from 'svelte';

	type Variant = 'primary' | 'secondary' | 'tertiary' | 'sm';

	// When href is provided, anchor attributes apply; otherwise button attributes apply.
	// We keep separate prop types to preserve event handler type inference.
	let {
		variant,
		href,
		class: extraClass = '',
		children,
		...rest
	}: {
		variant: Variant;
		href?: string;
		class?: string;
		children?: Snippet;
	} & (HTMLButtonAttributes & HTMLAnchorAttributes) = $props();

	const variantClass: Record<Variant, string> = {
		primary: 'btn-primary',
		secondary: 'btn-secondary',
		tertiary: 'btn-tertiary',
		sm: 'btn-sm',
	};
</script>

{#if href}
	<a {href} class="{variantClass[variant]} {extraClass}" {...(rest as HTMLAnchorAttributes)}>
		{@render children?.()}
	</a>
{:else}
	<button class="{variantClass[variant]} {extraClass}" {...(rest as HTMLButtonAttributes)}>
		{@render children?.()}
	</button>
{/if}
