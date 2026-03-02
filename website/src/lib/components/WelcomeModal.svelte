<script lang="ts">
	import { browser } from '$app/environment';
	import Icon from '$lib/components/Icons.svelte';
	import { getExternalKeys } from '$lib/services/user-api';
	import type { IdentityInfo } from '$lib/stores/auth';
	import {
		completeOnboarding,
		getActivationActionHref,
		hasSshExternalKey,
		isOnboardingCompleted,
		nextStep,
		type ActivationAction,
		type OnboardingStep,
	} from './welcome-onboarding';

	let { identity = null }: { identity?: IdentityInfo | null } = $props();

	let open = $state(browser ? !isOnboardingCompleted(sessionStorage) : false);
	let step = $state<OnboardingStep>(1);
	let hasSshKey = $state(false);
	let sshKeysLoading = $state(false);
	let sshKeysError = $state<string | null>(null);
	let checkedUsername = $state<string | null>(null);

	let username = $derived(identity?.account?.username ?? '');
	let email = $derived(identity?.account?.email ?? '');
	let hasUsername = $derived(username.length > 0);
	let hasEmail = $derived(email.length > 0);

	function finishOnboarding() {
		if (browser) {
			completeOnboarding(sessionStorage);
		}
		open = false;
	}

	function goToNextStep() {
		step = nextStep(step);
	}

	async function loadSshStatus(targetUsername: string) {
		sshKeysLoading = true;
		sshKeysError = null;
		try {
			const keys = await getExternalKeys(targetUsername);
			hasSshKey = hasSshExternalKey(keys);
		} catch (err) {
			hasSshKey = false;
			sshKeysError = err instanceof Error ? err.message : 'Failed to fetch SSH keys';
		} finally {
			sshKeysLoading = false;
		}
	}

	$effect(() => {
		if (!open || !username || checkedUsername === username) {
			return;
		}
		checkedUsername = username;
		void loadSshStatus(username);
	});

	function activationHref(action: ActivationAction): string {
		return getActivationActionHref(action);
	}
</script>

{#if open}
	<div class="fixed inset-0 z-50 flex items-center justify-center p-4">
		<!-- Backdrop -->
		<div class="absolute inset-0 bg-black/70 backdrop-blur-sm" onclick={finishOnboarding} role="presentation"></div>

		<!-- Modal -->
		<div class="relative z-10 w-full max-w-lg bg-surface border border-neutral-800 shadow-2xl">
			<!-- Progress indicator -->
			<div class="flex gap-1 p-5 pb-0">
				{#each [1, 2, 3] as s (s)}
					<div class="h-1 flex-1 {step >= s ? 'bg-primary-500' : 'bg-neutral-800'} transition-colors"></div>
				{/each}
			</div>

			<!-- Step 1: Complete profile -->
			{#if step === 1}
				<div class="p-6 space-y-4">
					<div class="icon-box-accent w-12 h-12">
						<Icon name="user" size={24} />
					</div>
					<h2 class="text-xl font-bold text-white">Complete your profile</h2>
					<p class="text-neutral-400 text-sm leading-relaxed">
						Confirm your account details so providers can trust your requests and platform notifications can reach you.
					</p>
					<div class="space-y-2 rounded border border-neutral-800 bg-surface-elevated p-3 text-sm">
						<div class="flex items-center justify-between">
							<span class="text-neutral-400">Username</span>
							<span class={hasUsername ? 'text-emerald-400' : 'text-amber-400'}>{hasUsername ? `@${username}` : 'Missing'}</span>
						</div>
						<div class="flex items-center justify-between">
							<span class="text-neutral-400">Email</span>
							<span class={hasEmail ? 'text-emerald-400' : 'text-amber-400'}>{hasEmail ? email : 'Missing'}</span>
						</div>
					</div>
					<div class="flex items-center justify-between pt-2">
						<a href="/dashboard/account/profile" class="text-xs text-primary-400 hover:text-primary-300 transition-colors">
							Open profile settings
						</a>
						<button
							type="button"
							onclick={goToNextStep}
							class="inline-flex items-center gap-2 px-5 py-2.5 bg-primary-500 hover:bg-primary-400 text-neutral-900 text-sm font-semibold transition-colors"
						>
							Continue
							<Icon name="arrow-right" size={16} />
						</button>
					</div>
				</div>
			{/if}

			<!-- Step 2: Add SSH key -->
			{#if step === 2}
				<div class="p-6 space-y-4">
					<h2 class="text-xl font-bold text-white">Add your SSH key</h2>
					<p class="text-neutral-400 text-sm leading-relaxed">
						You need at least one SSH key to access rented machines securely.
					</p>

					<div class="rounded border border-neutral-800 bg-surface-elevated p-3 text-sm">
						{#if sshKeysLoading}
							<p class="text-neutral-400">Checking your external keys...</p>
						{:else if sshKeysError}
							<p class="text-amber-400">Could not verify your SSH keys: {sshKeysError}</p>
						{:else if hasSshKey}
							<p class="text-emerald-400">SSH key detected. You are ready to rent.</p>
						{:else}
							<p class="text-amber-400">No SSH key found yet. Add one in Security settings.</p>
						{/if}
					</div>

					<div class="flex items-center justify-between gap-2 pt-2">
						<div class="flex items-center gap-3">
							<a href="/dashboard/account/security" class="text-xs text-primary-400 hover:text-primary-300 transition-colors">
								Manage SSH keys
							</a>
							<button type="button" onclick={() => loadSshStatus(username)} class="text-xs text-neutral-500 hover:text-neutral-300 transition-colors" disabled={sshKeysLoading || !username}>
								Refresh
							</button>
						</div>
						<button
							type="button"
							onclick={goToNextStep}
							class="inline-flex items-center gap-2 px-5 py-2.5 bg-primary-500 hover:bg-primary-400 text-neutral-900 text-sm font-semibold transition-colors"
						>
							Continue
							<Icon name="arrow-right" size={16} />
						</button>
					</div>
				</div>
			{/if}

			<!-- Step 3: Choose next action -->
			{#if step === 3}
				<div class="p-6 space-y-4">
					<h2 class="text-xl font-bold text-white">Choose your next action</h2>
					<p class="text-neutral-400 text-sm">Pick one path to start activating your account now.</p>

					<div class="pt-2 flex flex-col gap-3">
						<a
							href={activationHref('marketplace')}
							onclick={finishOnboarding}
							class="inline-flex items-center justify-center gap-2 px-5 py-2.5 bg-primary-500 hover:bg-primary-400 text-neutral-900 text-sm font-semibold transition-colors"
						>
							<Icon name="cart" size={16} />
							Browse marketplace
						</a>
						<a
							href={activationHref('provider')}
							onclick={finishOnboarding}
							class="inline-flex items-center justify-center gap-2 px-5 py-2.5 border border-neutral-700 bg-surface-elevated hover:border-neutral-500 text-white text-sm font-semibold transition-colors"
						>
							<Icon name="server" size={16} />
							Become a provider
						</a>
						<button
							type="button"
							onclick={finishOnboarding}
							class="text-xs text-neutral-500 hover:text-neutral-400 transition-colors text-center"
						>
							Stay on dashboard
						</button>
					</div>
				</div>
			{/if}
		</div>
	</div>
{/if}
