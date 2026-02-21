<script lang="ts">
	interface Props {
		/** null = still loading, true = onboarding done, false = onboarding needed */
		completed: boolean | null;
	}
	let { completed }: Props = $props();
</script>

{#if completed === false}
	<div class="bg-amber-500/20 border border-amber-500/30 p-4 flex items-start gap-3">
		<svg class="w-5 h-5 text-amber-400 shrink-0 mt-0.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
		</svg>
		<div>
			<p class="text-amber-400 font-semibold">Provider Setup Required</p>
			<p class="text-amber-400/80 text-sm mt-1">
				Complete your <a href="/dashboard/provider/support" class="underline hover:text-amber-300">Provider Setup</a> to unlock this feature and make your offerings visible in the marketplace.
			</p>
		</div>
	</div>
{/if}
