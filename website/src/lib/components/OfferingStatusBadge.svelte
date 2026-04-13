<script lang="ts">
	import Icon from './Icons.svelte';

	interface Props {
		providerOnline?: boolean;
		trustScore?: number;
		hasCriticalFlags?: boolean;
		isReseller?: boolean;
		resellerName?: string;
		resellerCommission?: number;
		isDemo?: boolean;
		isSubscription?: boolean;
		subscriptionIntervalDays?: number;
		hasRecipe?: boolean;
	}

	let {
		providerOnline = true,
		trustScore,
		hasCriticalFlags = false,
		isReseller = false,
		resellerName,
		resellerCommission,
		isDemo = false,
		isSubscription = false,
		subscriptionIntervalDays,
		hasRecipe = false
	}: Props = $props();

	let showTooltip = $state(false);

	function getSubscriptionLabel(): string | null {
		if (!isSubscription) return null;
		if (!subscriptionIntervalDays) return "Recurring";
		if (subscriptionIntervalDays <= 31) return "Monthly";
		if (subscriptionIntervalDays <= 93) return "Quarterly";
		if (subscriptionIntervalDays <= 366) return "Yearly";
		return `${subscriptionIntervalDays}d`;
	}

	function getPrimaryStatus(): { label: string; color: string } | null {
		if (providerOnline === false) {
			return { label: "Offline", color: "bg-danger/20 text-danger border-danger/30" };
		}
		if (isDemo) {
			return { label: "Demo", color: "bg-warning/20 text-warning border-warning/30" };
		}
		if (isReseller && resellerName) {
			return { label: `Via ${resellerName}`, color: "bg-primary-500/20 text-primary-400 border-primary-500/30" };
		}
		return null;
	}

	const primaryStatus = $derived(getPrimaryStatus());
	const subscriptionLabel = $derived(getSubscriptionLabel());

	const additionalInfo = $derived.by(() => {
		const items: string[] = [];
		if (trustScore !== undefined) {
			items.push(`Trust: ${trustScore}/100`);
		}
		if (subscriptionLabel) {
			items.push(`Subscription: ${subscriptionLabel}`);
		}
		if (hasRecipe) {
			items.push("Has setup recipe");
		}
		if (hasCriticalFlags) {
			items.push("Has warnings");
		}
		return items;
	});

	const hasTooltip = $derived(additionalInfo.length > 0);
</script>

<div class="inline-flex items-center gap-1.5">
	{#if primaryStatus}
		<div
			class="inline-flex items-center gap-1 px-1.5 py-0.5 text-xs rounded border {primaryStatus.color}"
		>
			{#if providerOnline === false}
				<span class="h-1.5 w-1.5 rounded-full bg-danger"></span>
			{/if}
			{primaryStatus.label}
			{#if isReseller && resellerCommission}
				<span class="opacity-70">(+{resellerCommission}%)</span>
			{/if}
		</div>
	{:else if trustScore !== undefined}
		<div
			class="inline-flex items-center gap-1 px-1.5 py-0.5 text-xs rounded border {hasCriticalFlags ? 'bg-danger/20 text-danger border-danger/30' : trustScore >= 80 ? 'bg-success/20 text-success border-success/30' : trustScore >= 60 ? 'bg-warning/20 text-warning border-warning/30' : 'bg-danger/20 text-danger border-danger/30'}"
		>
			{#if hasCriticalFlags}
				<Icon name="alert" size={12} />
			{/if}
			<span>{trustScore}</span>
		</div>
	{/if}

	{#if hasTooltip}
		<div class="relative">
			<button
				type="button"
				class="text-neutral-500 hover:text-neutral-300 transition-colors"
				onmouseenter={() => showTooltip = true}
				onmouseleave={() => showTooltip = false}
				onclick={(e) => { e.stopPropagation(); showTooltip = !showTooltip; }}
				aria-label="More details"
			>
				<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
					<path d="M8 0a8 8 0 100 16A8 8 0 008 0zm0 12a1 1 0 110-2 1 1 0 010 2zm1-3.5V9a1 1 0 00-2 0v-.5a1 1 0 001-1V5a1 1 0 00-2 0 1 1 0 00-2 0 3 3 0 106 0v2.5z"/>
				</svg>
			</button>
			{#if showTooltip}
				<div
					class="absolute bottom-full left-0 mb-2 z-50 w-44 bg-neutral-900 border border-neutral-700 rounded p-2 text-xs shadow-xl pointer-events-none"
					role="tooltip"
				>
					{#each additionalInfo as info}
						<div class="text-neutral-300 py-0.5">{info}</div>
					{/each}
				</div>
			{/if}
		</div>
	{/if}
</div>
