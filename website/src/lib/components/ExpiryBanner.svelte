<script lang="ts">
	import { getExpiringContracts, isUrgent, getExpiryBannerText } from '$lib/utils/expiry-utils';
	import type { UserActivity } from '$lib/services/api-user-activity';
	import { browser } from '$app/environment';

	const DISMISSED_KEY = 'expiry-banner-dismissed';

	let { activity }: { activity: UserActivity | null } = $props();

	let dismissed = $state(browser ? sessionStorage.getItem(DISMISSED_KEY) === '1' : true);

	function dismiss() {
		dismissed = true;
		if (browser) sessionStorage.setItem(DISMISSED_KEY, '1');
	}

	const expiringContracts = $derived(
		activity ? getExpiringContracts(activity.rentals_as_requester, Date.now()) : []
	);
	const hasUrgent = $derived(expiringContracts.some((c) => isUrgent(c, Date.now())));
	const bannerText = $derived(getExpiryBannerText(expiringContracts.length, hasUrgent));
	const show = $derived(!dismissed && expiringContracts.length > 0);
</script>

{#if show}
	<div class="flex items-center gap-3 p-4 border {hasUrgent ? 'bg-red-500/10 border-red-500/30' : 'bg-amber-500/10 border-amber-500/30'}">
		<svg class="w-4 h-4 shrink-0 {hasUrgent ? 'text-red-400' : 'text-amber-400'}" fill="currentColor" viewBox="0 0 20 20">
			<path fill-rule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clip-rule="evenodd"></path>
		</svg>
		<p class="flex-1 text-sm {hasUrgent ? 'text-red-300' : 'text-amber-300'}">
			{bannerText}
		</p>
		<a
			href="/dashboard/rentals"
			class="px-3 py-1.5 text-xs font-semibold transition-colors whitespace-nowrap {hasUrgent ? 'bg-red-500 hover:bg-red-400 text-white' : 'bg-amber-500 hover:bg-amber-400 text-neutral-900'}"
		>
			View Rentals
		</a>
		<button
			type="button"
			onclick={dismiss}
			class="p-1 transition-colors {hasUrgent ? 'text-red-400 hover:text-red-300' : 'text-amber-400 hover:text-amber-300'}"
			aria-label="Dismiss"
		>
			<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
				<path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
			</svg>
		</button>
	</div>
{/if}
