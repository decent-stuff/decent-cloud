<script lang="ts">
	import { authStore, type AccountInfo } from '$lib/stores/auth';
	import { resendVerificationEmail } from '$lib/services/account-api';

	let isResending = $state(false);
	let statusMessage = $state<{ type: 'success' | 'error'; text: string } | null>(null);
	let account = $state<AccountInfo | null>(null);

	authStore.activeIdentity.subscribe((identity) => {
		account = identity?.account || null;
	});

	const hasNoEmail = $derived(!account?.email);

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
			const errorMessage = error instanceof Error ? error.message : 'Failed to resend email';
			statusMessage = { type: 'error', text: errorMessage };
		} finally {
			isResending = false;
		}
	}
</script>

<div class="fixed top-16 md:top-0 left-0 md:left-64 right-0 z-40 bg-amber-500 border-b-2 border-amber-600 p-4 shadow-lg">
	<div class="max-w-7xl mx-auto">
		<div class="flex flex-col md:flex-row items-start md:items-center gap-3">
			<div class="flex items-start gap-3 flex-1">
				<svg class="w-6 h-6 text-amber-900 flex-shrink-0 mt-0.5" fill="currentColor" viewBox="0 0 20 20">
					<path fill-rule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clip-rule="evenodd"></path>
				</svg>
				<div class="flex-1">
					{#if hasNoEmail}
						<h3 class="text-amber-900 font-bold text-lg">No Email Address on Account</h3>
						<p class="text-amber-800 text-sm mt-1">
							Your account needs an email address for verification and recovery.
							<a href="/dashboard/account/profile" class="underline font-medium hover:text-amber-700">Add your email in profile settings</a>
						</p>
					{:else}
						<h3 class="text-amber-900 font-bold text-lg">Verify Your Email Address</h3>
						<p class="text-amber-800 text-sm mt-1">Email verification improves your reputation and unlocks full platform features</p>
					{/if}
				</div>
			</div>
			{#if !hasNoEmail}
				<button
					onclick={handleResend}
					disabled={isResending}
					class="px-4 py-2 bg-amber-900 hover:bg-amber-800 disabled:bg-amber-700 disabled:opacity-50  text-white text-sm font-medium transition-colors whitespace-nowrap"
				>
					{isResending ? 'Sending...' : 'Resend Verification Email'}
				</button>
			{/if}
		</div>

		{#if statusMessage}
			<div class="mt-3 text-sm {statusMessage.type === 'success' ? 'text-amber-900' : 'text-red-900'}">
				{statusMessage.text}
			</div>
		{/if}
	</div>
</div>
