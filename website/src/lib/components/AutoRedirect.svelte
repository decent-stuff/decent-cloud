<script lang="ts">
	/**
	 * Auto-redirect countdown with an inline manual link.
	 *
	 * Mounts a 1s ticker that decrements `remaining`; when it hits 0, calls
	 * `goto(href)` for client-side navigation. A visible "Redirecting … in Ns"
	 * copy tells the user the redirect is coming, and an inline `<a>` provides
	 * a manual "Go now" affordance so the auto-redirect is never the only
	 * path (accessibility + test determinism). The interval is cleared on
	 * destroy and on the first redirect (manual or automatic) to avoid double
	 * navigation if the user clicks the link as the timer fires.
	 */
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';

	let {
		href = '/dashboard/marketplace',
		delaySeconds = 4,
		label = 'dashboard',
	}: {
		href?: string;
		delaySeconds?: number;
		label?: string;
	} = $props();

	let remaining = $state(delaySeconds);
	let timer: ReturnType<typeof setInterval> | null = null;

	function redirect() {
		if (timer) {
			clearInterval(timer);
			timer = null;
		}
		goto(href);
	}

	onMount(() => {
		timer = setInterval(() => {
			remaining -= 1;
			if (remaining <= 0) redirect();
		}, 1000);
	});

	onDestroy(() => {
		if (timer) clearInterval(timer);
	});
</script>

<p class="text-neutral-500 text-sm">
	Redirecting to {label} in {remaining}s…
	<a href={href} class="text-primary-400 hover:text-primary-300 underline ml-1">Go now</a>
</p>
