<script lang="ts">
	import { authStore } from '$lib/stores/auth';
	import { updateAccountEmail } from '$lib/services/account-api';

	let { email = '', username = '', onEmailUpdated = () => {} } = $props<{
		email: string;
		username: string;
		onEmailUpdated?: (newEmail: string) => void;
	}>();

	let newEmail = $state(email || '');
	let isEditing = $state(false);
	let isSubmitting = $state(false);
	let error = $state<string | null>(null);
	let success = $state<string | null>(null);

	function validateEmail(value: string): boolean {
		const emailPattern = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
		return emailPattern.test(value.trim());
	}

	const emailValid = $derived(validateEmail(newEmail));
	const hasChanged = $derived(newEmail.trim() !== (email || ''));

	async function handleSubmit() {
		if (!emailValid || !hasChanged || isSubmitting) return;

		isSubmitting = true;
		error = null;
		success = null;

		try {
			const identityResult = await authStore.getAuthenticatedIdentity();
			if (!identityResult) {
				error = 'Not authenticated';
				return;
			}

			await updateAccountEmail(identityResult.identity as any, username, newEmail.trim());
			success = 'Email updated. Check your inbox for verification link.';
			isEditing = false;
			onEmailUpdated(newEmail.trim());
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to update email';
		} finally {
			isSubmitting = false;
		}
	}

	function handleCancel() {
		newEmail = email || '';
		isEditing = false;
		error = null;
	}
</script>

<div class="space-y-3">
	<label for="account-email-input" class="block text-sm font-medium text-white/70">Account Email</label>
	{#if isEditing}
		<div class="space-y-2">
			<input
				id="account-email-input"
				type="email"
				bind:value={newEmail}
				placeholder="you@example.com"
				class="w-full px-3 py-2 bg-white/5 border border-white/20 rounded-lg text-white placeholder-white/40 focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent"
			/>
			{#if newEmail && !emailValid}
				<p class="text-xs text-red-400">Please enter a valid email address</p>
			{/if}
			<div class="flex gap-2">
				<button
					onclick={handleSubmit}
					disabled={!emailValid || !hasChanged || isSubmitting}
					class="px-3 py-1.5 bg-purple-600 hover:bg-purple-500 disabled:bg-gray-600 disabled:opacity-50 rounded text-white text-sm font-medium transition-colors"
				>
					{isSubmitting ? 'Saving...' : 'Save'}
				</button>
				<button
					onclick={handleCancel}
					disabled={isSubmitting}
					class="px-3 py-1.5 bg-white/10 hover:bg-white/20 rounded text-white text-sm transition-colors"
				>
					Cancel
				</button>
			</div>
		</div>
	{:else}
		<div class="flex items-center gap-3">
			<span class="text-white/90">{email || 'Not set'}</span>
			<button
				onclick={() => { isEditing = true; error = null; success = null; }}
				class="px-2 py-1 text-xs bg-white/10 hover:bg-white/20 rounded text-white/70 hover:text-white transition-colors"
			>
				{email ? 'Change' : 'Add Email'}
			</button>
		</div>
	{/if}

	{#if error}
		<p class="text-sm text-red-400">{error}</p>
	{/if}
	{#if success}
		<p class="text-sm text-green-400">{success}</p>
	{/if}

	<p class="text-xs text-white/50">
		Used for account verification and recovery. A verification email will be sent when changed.
	</p>
</div>
