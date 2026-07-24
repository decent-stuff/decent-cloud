<script lang="ts">
	import Icon from '$lib/components/Icons.svelte';

	// `open` is bindable so the parent can toggle it, and the overlay can close
	// itself (backdrop click, X button, Escape) by flipping the same flag.
	let { open = $bindable(false) } = $props<{
		open?: boolean;
	}>();

	let dialogEl = $state<HTMLDivElement | null>(null);

	// Single source of truth for the documented keyboard shortcuts. Keep in
	// sync with the real handlers: marketplace '/' (handleGlobalKeydown in
	// +page.svelte), CommandPalette 'Cmd/Ctrl+K', this overlay '?', and the
	// Escape-to-close pattern used by dialogs across the app.
	interface Shortcut {
		keys: string[];
		description: string;
	}
	const SHORTCUTS: Shortcut[] = [
		{ keys: ['/'], description: 'Focus marketplace search' },
		{ keys: ['⌘K', 'Ctrl+K'], description: 'Open command palette' },
		{ keys: ['?'], description: 'Show this help' },
		{ keys: ['Esc'], description: 'Close dialogs/overlays' },
	];

	function close() {
		open = false;
	}

	// Move focus into the dialog on open so the overlay is keyboard/screen-reader
	// accessible. Escape is handled by the dashboard layout's <svelte:window>
	// handler, but the dialog mirrors it so the component is self-contained.
	$effect(() => {
		if (open && dialogEl) {
			dialogEl.focus();
		}
	});
</script>

{#if open}
	<!-- Backdrop: click to dismiss. Escape handled at the window level by the
	parent layout (idempotent with the dialog's own Escape handler below). -->
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div
		role="presentation"
		class="fixed inset-0 bg-base/80 backdrop-blur-sm z-50 flex items-center justify-center p-4"
		onclick={close}
		onkeydown={(e) => e.key === 'Escape' && close()}
	>
		<div
			bind:this={dialogEl}
			data-testid="keyboard-help"
			role="dialog"
			aria-modal="true"
			aria-label="Keyboard shortcuts help"
			tabindex="-1"
			class="bg-surface border border-neutral-800 rounded-lg w-full max-w-md shadow-2xl overflow-hidden"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => {
				if (e.key === 'Escape') close();
			}}
		>
			<div class="flex items-center justify-between px-5 py-4 border-b border-neutral-800">
				<h2 class="text-base font-semibold text-white">Keyboard shortcuts</h2>
				<button
					type="button"
					onclick={close}
					class="text-neutral-500 hover:text-white transition-colors"
					aria-label="Close keyboard shortcuts help"
				>
					<Icon name="x" size={18} />
				</button>
			</div>

			<ul class="divide-y divide-neutral-800">
				{#each SHORTCUTS as s}
					<li class="flex items-center justify-between gap-4 px-5 py-3">
						<span class="text-sm text-neutral-300">{s.description}</span>
						<span class="flex items-center gap-1.5 flex-shrink-0">
							{#each s.keys as key, i}
								{#if i > 0}<span class="text-xs text-neutral-600">or</span>{/if}
								<kbd
									class="font-mono text-xs bg-neutral-800 text-neutral-200 border border-neutral-700 rounded px-2 py-0.5"
								>{key}</kbd>
							{/each}
						</span>
					</li>
				{/each}
			</ul>

			<div class="px-5 py-2.5 border-t border-neutral-800 text-xs text-neutral-600">
				Press <kbd class="font-mono">Esc</kbd> to close
			</div>
		</div>
	</div>
{/if}
