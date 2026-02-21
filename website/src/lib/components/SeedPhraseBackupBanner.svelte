<script lang="ts">
	let { onDismiss }: { onDismiss: () => void } = $props();
</script>

<div class="fixed top-14 md:top-0 left-0 md:left-60 right-0 z-40 bg-amber-500/10 border-b border-amber-500/30 px-4 py-3">
	<div class="max-w-6xl mx-auto flex items-center gap-3">
		<svg class="w-4 h-4 text-amber-400 shrink-0" fill="currentColor" viewBox="0 0 20 20">
			<path fill-rule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clip-rule="evenodd"></path>
		</svg>
		<p class="flex-1 text-sm text-amber-300">
			Back up your seed phrase to avoid losing access to your account
		</p>
		<a
			href="/dashboard/account/security"
			onclick={onDismiss}
			class="px-3 py-1.5 bg-amber-500 hover:bg-amber-400 text-neutral-900 text-xs font-semibold transition-colors whitespace-nowrap"
		>
			Back Up Now
		</a>
		<button
			type="button"
			onclick={onDismiss}
			class="text-amber-400 hover:text-amber-300 transition-colors p-1"
			aria-label="Dismiss"
		>
			<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
				<path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
			</svg>
		</button>
	</div>
</div>
