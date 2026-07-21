<script lang="ts">
	/**
	 * OfferingStatusMenu — disclosure-widget replacement for the previous
	 * `cycle` buttons (#437).
	 *
	 * Built on a plain `<button>` + conditional panel (rather than `<details>`)
	 * because: (a) the parent offering card has its own `onclick` that opens
	 * the Quick Edit dialog, so the trigger must `stopPropagation` — and the
	 * `stopPropagation` + `<details>` + role-overrides interact unpredictably
	 * across browsers; (b) we need per-card mutual exclusion (only one menu
	 * open per offering) which `<details name=>` provides natively but only
	 * when the browser drives the toggle — manual toggling bypasses it.
	 *
	 * Mutual exclusion: each menu registers itself with the per-card
	 * `OfferingStatusMenuGroup` context; opening one closes the other.
	 */
	import { onDestroy } from 'svelte';

	interface Option {
		value: string;
		label: string;
		description: string;
	}

	interface Props {
		kind: 'visibility' | 'stock';
		currentValue: string;
		offeringId: number | string;
		onSelect: (value: string) => void | Promise<void>;
	}

	let { kind, currentValue, offeringId, onSelect }: Props = $props();

	const visibilityOptions: Option[] = [
		{ value: 'public', label: 'Public (marketplace)', description: 'Visible to everyone in the marketplace' },
		{ value: 'shared', label: 'Shared (allowlist only)', description: 'Visible only to customers on your allowlist' },
		{ value: 'private', label: 'Private (owner only)', description: 'Hidden from the marketplace' },
	];

	const stockOptions: Option[] = [
		{ value: 'in_stock', label: 'In Stock', description: 'Available for new orders' },
		{ value: 'out_of_stock', label: 'Out of Stock', description: 'Listed, but unavailable right now' },
		{ value: 'discontinued', label: 'Discontinued', description: 'Permanently unavailable; hidden from marketplace' },
	];

	const options = $derived(kind === 'visibility' ? visibilityOptions : stockOptions);

	const isCurrent = $derived((value: string) =>
		kind === 'visibility' ? currentValue.toLowerCase() === value.toLowerCase() : currentValue === value
	);

	function triggerLabel(value: string): string {
		if (kind === 'visibility') {
			switch (value.toLowerCase()) {
				case 'public': return 'Public';
				case 'shared': return 'Shared';
				case 'private': return 'Private';
				default: return value;
			}
		}
		switch (value) {
			case 'in_stock': return 'In Stock';
			case 'out_of_stock': return 'Out of Stock';
			case 'discontinued': return 'Discontinued';
			default: return value.replace(/_/g, ' ');
		}
	}

	function triggerStyle(): string {
		if (kind === 'visibility') {
			switch (currentValue.toLowerCase()) {
				case 'public': return 'bg-green-500/20 text-green-400 border-green-500/30 hover:bg-green-500/30';
				case 'shared': return 'bg-blue-500/20 text-blue-400 border-blue-500/30 hover:bg-blue-500/30';
				case 'private': return 'bg-red-500/20 text-red-400 border-red-500/30 hover:bg-red-500/30';
				default: return 'bg-gray-500/20 text-gray-400 border-gray-500/30 hover:bg-gray-500/30';
			}
		}
		switch (currentValue) {
			case 'in_stock': return 'bg-green-500/20 text-green-400 border-green-500/30';
			case 'out_of_stock': return 'bg-red-500/20 text-red-400 border-red-500/30';
			case 'discontinued': return 'bg-gray-500/20 text-gray-400 border-gray-500/30';
			default: return 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30';
		}
	}

	function triggerPrefix(): string {
		return kind === 'visibility' ? 'Visibility' : 'Stock';
	}

	let open = $state(false);

	// Per-card mutual-exclusion registry. Keyed by `offeringId`, each entry is
	// a Set of "close" callbacks. When a menu opens it calls every registered
	// sibling closer first. The registry lives on a module-level Map so the
	// two OfferingStatusMenu instances in the same card find each other
	// without lifting state into +page.svelte.
	type Closer = () => void;
	const groupKey = `offering-${offeringId}-status`;
	const registry = (globalThis as Record<string, unknown>).__offeringStatusMenus as
		| Map<string, Set<Closer>>
		| undefined;
	const groups = registry ?? new Map<string, Set<Closer>>();
	(globalThis as Record<string, unknown>).__offeringStatusMenus = groups;

	const myClosers = groups.get(groupKey) ?? new Set<Closer>();
	groups.set(groupKey, myClosers);
	myClosers.add(() => { open = false; });

	onDestroy(() => {
		myClosers.delete(() => { open = false; });
		if (myClosers.size === 0) groups.delete(groupKey);
	});

	function closeSiblings() {
		for (const close of myClosers) {
			// Each closer closes its own menu; calling it from this iterator
			// closes every menu in the group (including this one — then we
			// re-open below).
			close();
		}
	}

	function toggleOpen(e: MouseEvent) {
		e.stopPropagation();
		const willOpen = !open;
		closeSiblings();
		open = willOpen;
	}

	function onTriggerKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' || e.key === ' ') {
			e.stopPropagation();
			e.preventDefault();
			const willOpen = !open;
			closeSiblings();
			open = willOpen;
		} else if (e.key === 'Escape' && open) {
			e.stopPropagation();
			open = false;
		}
	}

	async function handleSelect(value: string, e: MouseEvent) {
		e.stopPropagation();
		open = false;
		await onSelect(value);
	}

	function onOptionKeydown(value: string, e: KeyboardEvent) {
		if (e.key === 'Enter' || e.key === ' ') {
			e.stopPropagation();
			e.preventDefault();
			open = false;
			void onSelect(value);
		} else if (e.key === 'Escape') {
			e.stopPropagation();
			open = false;
		}
	}

	// Click-outside-to-close. Fires on every document click while open; we
	// only react if the target is outside our container.
	let containerEl: HTMLDivElement | undefined = $state();
	function onDocumentClick(e: MouseEvent) {
		if (!open || !containerEl) return;
		if (!containerEl.contains(e.target as Node)) {
			open = false;
		}
	}

	// Panel direction: drop DOWN by default, but flip UP if there isn't
	// enough space below the trigger in the viewport. Without this, on
	// narrow cards the badges flex-wrap onto multiple rows and a downward
	// panel can visually cover the wrapped sibling trigger (or extend
	// beyond the viewport bottom), making it unclickable.
	let dropUp = $state(false);
	function recomputeDirection() {
		if (!containerEl) return;
		const trigger = containerEl.querySelector('button');
		if (!trigger) return;
		const rect = trigger.getBoundingClientRect();
		const spaceBelow = window.innerHeight - rect.bottom;
		// 200px is a conservative estimate of the open panel's height
		// (3 options × ~60px each + padding).
		dropUp = spaceBelow < 200 && rect.top >= 200;
	}
	$effect(() => {
		if (open) {
			recomputeDirection();
			document.addEventListener('click', onDocumentClick);
			window.addEventListener('scroll', recomputeDirection, { passive: true });
			window.addEventListener('resize', recomputeDirection);
		}
		return () => {
			document.removeEventListener('click', onDocumentClick);
			window.removeEventListener('scroll', recomputeDirection);
			window.removeEventListener('resize', recomputeDirection);
		};
	});
</script>

<div bind:this={containerEl} class="relative">
	<button
		type="button"
		aria-label="{triggerPrefix()}: {triggerLabel(currentValue)}"
		aria-haspopup="menu"
		aria-expanded={open}
		onclick={toggleOpen}
		onkeydown={onTriggerKeydown}
		class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium border transition-all hover:scale-105 cursor-pointer {triggerStyle()}"
	>
		{#if kind === 'stock'}
			<span class="w-2 h-2 rounded-full bg-current" aria-hidden="true"></span>
		{/if}
		{triggerLabel(currentValue)}
		<svg class="w-3 h-3 opacity-70" viewBox="0 0 12 12" fill="none" aria-hidden="true">
			<path d="M3 4.5L6 7.5L9 4.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
		</svg>
	</button>

	{#if open}
		<div
			data-status-menu={kind}
			role="menu"
			aria-label="{triggerPrefix()} options"
			class="absolute right-0 z-30 {dropUp ? 'bottom-full mb-1' : 'mt-1'} min-w-[16rem] rounded-lg border border-neutral-700 bg-surface-elevated shadow-xl overflow-hidden"
		>
			{#each options as opt (opt.value)}
				<button
					type="button"
					role="menuitemradio"
					data-value={opt.value}
					aria-checked={isCurrent(opt.value)}
					onclick={(e) => handleSelect(opt.value, e)}
					onkeydown={(e) => onOptionKeydown(opt.value, e)}
					class="w-full text-left px-3 py-2 flex items-start gap-2 transition-colors {isCurrent(opt.value) ? 'bg-neutral-800/60' : 'hover:bg-neutral-800/60'}"
				>
					<div class="flex-1">
						<div class="text-xs font-medium text-white">{opt.label}</div>
						<div class="text-[11px] text-neutral-400">{opt.description}</div>
					</div>
					<svg
						class="w-3.5 h-3.5 mt-0.5 flex-shrink-0 {isCurrent(opt.value) ? 'text-primary-400' : 'opacity-0'}"
						viewBox="0 0 12 12" fill="none" aria-hidden="true"
					>
						<path d="M2.5 6L5 8.5L9.5 3.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
					</svg>
				</button>
			{/each}
		</div>
	{/if}
</div>
