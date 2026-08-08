<script lang="ts">
	import { authStore } from '$lib/stores/auth';
	import { resendVerificationEmail } from '$lib/services/account-api';
	import Icon from '$lib/components/Icons.svelte';

	let {
		showEmailAction,
		hasEmail,
		showSeedAction,
		onDismissEmail,
		onDismissSeed
	}: {
		showEmailAction: boolean;
		hasEmail: boolean;
		showSeedAction: boolean;
		onDismissEmail: () => void;
		onDismissSeed: () => void;
	} = $props();

	let expanded = $state(false);
	let isResending = $state(false);
	let statusMessage = $state<{ type: 'success' | 'error'; text: string } | null>(null);

	type Action = {
		key: 'email' | 'seed';
		label: string;
		href: string;
		cta: string;
		resendable: boolean;
	};

	const actions = $derived<Action[]>([
		...(showEmailAction
			? [{
					key: 'email' as const,
					label: hasEmail ? 'verify your email' : 'add your email',
					href: '/dashboard/account/profile',
					cta: hasEmail ? 'Resend email' : 'Add email',
					resendable: hasEmail
				}]
			: []),
		...(showSeedAction
			? [{
					key: 'seed' as const,
					label: 'back up your seed phrase',
					href: '/dashboard/account/security',
					cta: 'Back up now',
					resendable: false
				}]
			: [])
	]);

	const count = $derived(actions.length);
	const summary = $derived(actions.map((a) => a.label).join(', '));

	async function handleResend() {
		if (isResending) return;
		isResending = true;
		statusMessage = null;
		try {
			const identityResult = await authStore.getAuthenticatedIdentity();
			if (!identityResult) {
				statusMessage = { type: 'error', text: 'Not authenticated' };
				return;
			}
			const message = await resendVerificationEmail(identityResult.identity as any);
			statusMessage = { type: 'success', text: message };
		} catch (error) {
			statusMessage = {
				type: 'error',
				text: error instanceof Error ? error.message : 'Failed to resend email'
			};
		} finally {
			isResending = false;
		}
	}

	function dismissAction(key: 'email' | 'seed') {
		if (key === 'email') onDismissEmail();
		else onDismissSeed();
	}

	function dismissAll() {
		if (showEmailAction) onDismissEmail();
		if (showSeedAction) onDismissSeed();
		expanded = false;
	}
</script>

{#if count > 0}
	<div
		role="status"
		data-testid="action-required-banner"
		class="bg-amber-500/10 border border-amber-500/30 px-4 py-2.5"
	>
		<div class="flex items-center gap-3">
			<Icon name="alert" size={16} class="text-amber-400 shrink-0" />
			<p class="flex-1 text-sm text-amber-300 min-w-0 truncate">
				<span class="font-semibold text-amber-200">{count} {count === 1 ? 'action' : 'actions'} needed:</span>
				<span class="ml-1">{summary}</span>
			</p>
			<button
				type="button"
				onclick={() => (expanded = !expanded)}
				class="text-xs font-semibold text-amber-300 hover:text-amber-200 transition-colors flex items-center gap-1 shrink-0"
				aria-expanded={expanded}
				aria-controls="action-required-details"
			>
				{expanded ? 'Hide' : 'Review'}
				<Icon name={expanded ? 'chevron-up' : 'chevron-down'} size={14} />
			</button>
			<button
				type="button"
				onclick={dismissAll}
				class="text-amber-400 hover:text-amber-300 transition-colors p-1 shrink-0"
				aria-label="Dismiss all action reminders"
			>
				<Icon name="x" size={14} />
			</button>
		</div>

		{#if expanded}
			<div id="action-required-details" class="mt-2.5 pt-2.5 border-t border-amber-500/20 space-y-2">
				{#each actions as action (action.key)}
					<div class="flex items-center gap-3">
						<Icon
							name={action.key === 'email' ? 'mail' : 'key'}
							size={14}
							class="text-amber-400/80 shrink-0"
						/>
						<span class="flex-1 text-sm text-amber-200/90 min-w-0">{action.label}</span>
						{#if action.resendable}
							<button
								type="button"
								onclick={handleResend}
								disabled={isResending}
								class="px-2.5 py-1 bg-amber-500 hover:bg-amber-400 disabled:opacity-50 text-neutral-900 text-xs font-semibold transition-colors shrink-0"
							>
								{isResending ? 'Sending…' : action.cta}
							</button>
						{:else}
							<a
								href={action.href}
								class="px-2.5 py-1 bg-amber-500 hover:bg-amber-400 text-neutral-900 text-xs font-semibold transition-colors shrink-0"
							>
								{action.cta}
							</a>
						{/if}
						<button
							type="button"
							onclick={() => dismissAction(action.key)}
							class="text-amber-400/70 hover:text-amber-300 transition-colors p-1 shrink-0"
							aria-label="Dismiss {action.label}"
						>
							<Icon name="x" size={12} />
						</button>
					</div>
				{/each}
				{#if statusMessage}
					<p class="text-xs {statusMessage.type === 'success' ? 'text-amber-200' : 'text-red-300'} pl-6">
						{statusMessage.text}
					</p>
				{/if}
			</div>
		{/if}
	</div>
{/if}
