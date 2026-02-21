<script lang="ts">
	import { browser } from '$app/environment';
	import Icon from '$lib/components/Icons.svelte';

	export const ONBOARDING_KEY = 'onboarding_completed';
	export const ROLE_PREF_KEY = 'user_role_preference';

	export type UserRolePreference = 'tenant' | 'provider';

	function isOnboardingCompleted(): boolean {
		if (!browser) return true;
		return localStorage.getItem(ONBOARDING_KEY) === 'true';
	}

	function completeOnboarding() {
		if (browser) localStorage.setItem(ONBOARDING_KEY, 'true');
		open = false;
	}

	function saveRolePreference(role: UserRolePreference) {
		if (browser) localStorage.setItem(ROLE_PREF_KEY, role);
		selectedRole = role;
	}

	let open = $state(!isOnboardingCompleted());
	let step = $state(1);
	let selectedRole = $state<UserRolePreference | null>(null);

	function next() {
		step += 1;
	}

	function handleRoleSelect(role: UserRolePreference) {
		saveRolePreference(role);
		next();
	}
</script>

{#if open}
	<div class="fixed inset-0 z-50 flex items-center justify-center p-4">
		<!-- Backdrop -->
		<div class="absolute inset-0 bg-black/70 backdrop-blur-sm" onclick={completeOnboarding} role="presentation"></div>

		<!-- Modal -->
		<div class="relative z-10 w-full max-w-lg bg-surface border border-neutral-800 shadow-2xl">
			<!-- Progress indicator -->
			<div class="flex gap-1 p-5 pb-0">
				{#each [1, 2, 3] as s (s)}
					<div class="h-1 flex-1 {step >= s ? 'bg-primary-500' : 'bg-neutral-800'} transition-colors"></div>
				{/each}
			</div>

			<!-- Step 1: Welcome -->
			{#if step === 1}
				<div class="p-6 space-y-4">
					<div class="icon-box-accent w-12 h-12">
						<Icon name="star" size={24} />
					</div>
					<h2 class="text-xl font-bold text-white">Welcome to Decent Cloud</h2>
					<p class="text-neutral-400 text-sm leading-relaxed">
						A decentralized marketplace for cloud computing. Rent VMs from trusted providers, or list your own hardware and earn ICP.
					</p>
					<p class="text-neutral-500 text-xs">
						No lock-in. No corporate intermediary. Fully verifiable on-chain.
					</p>
					<div class="flex items-center justify-between pt-2">
						<button
							type="button"
							onclick={completeOnboarding}
							class="text-xs text-neutral-500 hover:text-neutral-400 transition-colors"
						>
							Skip
						</button>
						<button
							type="button"
							onclick={next}
							class="inline-flex items-center gap-2 px-5 py-2.5 bg-primary-500 hover:bg-primary-400 text-neutral-900 text-sm font-semibold transition-colors"
						>
							Get Started
							<Icon name="arrow-right" size={16} />
						</button>
					</div>
				</div>
			{/if}

			<!-- Step 2: Choose Role -->
			{#if step === 2}
				<div class="p-6 space-y-4">
					<h2 class="text-xl font-bold text-white">How will you use Decent Cloud?</h2>
					<p class="text-neutral-500 text-xs">Choose your primary role. You can do both.</p>

					<div class="grid grid-cols-2 gap-3 mt-2">
						<!-- Tenant card -->
						<button
							type="button"
							onclick={() => handleRoleSelect('tenant')}
							class="group text-left p-4 border {selectedRole === 'tenant' ? 'border-primary-500 bg-primary-500/10' : 'border-neutral-700 bg-surface-elevated hover:border-neutral-600'} transition-all"
						>
							<div class="icon-box mb-3 group-hover:border-primary-500/30 transition-colors">
								<Icon name="cart" size={18} />
							</div>
							<h3 class="text-sm font-semibold text-white mb-2">Rent cloud resources</h3>
							<ul class="space-y-1">
								<li class="text-xs text-neutral-500">Deploy VMs in minutes</li>
								<li class="text-xs text-neutral-500">Pay with ICP tokens</li>
								<li class="text-xs text-neutral-500">Verified provider trust scores</li>
							</ul>
						</button>

						<!-- Provider card -->
						<button
							type="button"
							onclick={() => handleRoleSelect('provider')}
							class="group text-left p-4 border {selectedRole === 'provider' ? 'border-primary-500 bg-primary-500/10' : 'border-neutral-700 bg-surface-elevated hover:border-neutral-600'} transition-all"
						>
							<div class="icon-box mb-3 group-hover:border-primary-500/30 transition-colors">
								<Icon name="server" size={18} />
							</div>
							<h3 class="text-sm font-semibold text-white mb-2">Offer my resources</h3>
							<ul class="space-y-1">
								<li class="text-xs text-neutral-500">Monetize idle hardware</li>
								<li class="text-xs text-neutral-500">Set your own pricing</li>
								<li class="text-xs text-neutral-500">Build reputation on-chain</li>
							</ul>
						</button>
					</div>

					<div class="flex items-center justify-between pt-2">
						<button
							type="button"
							onclick={completeOnboarding}
							class="text-xs text-neutral-500 hover:text-neutral-400 transition-colors"
						>
							Skip
						</button>
					</div>
				</div>
			{/if}

			<!-- Step 3: Next Steps -->
			{#if step === 3}
				<div class="p-6 space-y-4">
					<h2 class="text-xl font-bold text-white">You're all set!</h2>
					<p class="text-neutral-400 text-sm">
						{#if selectedRole === 'provider'}
							Set up your provider profile to start listing your resources on the marketplace.
						{:else}
							Browse the marketplace to find a VM that fits your needs and deploy in minutes.
						{/if}
					</p>

					<div class="pt-2 flex flex-col gap-3">
						{#if selectedRole === 'provider'}
							<a
								href="/dashboard/provider/support"
								onclick={completeOnboarding}
								class="inline-flex items-center justify-center gap-2 px-5 py-2.5 bg-primary-500 hover:bg-primary-400 text-neutral-900 text-sm font-semibold transition-colors"
							>
								<Icon name="server" size={16} />
								Set up provider profile
							</a>
						{:else}
							<a
								href="/dashboard/marketplace"
								onclick={completeOnboarding}
								class="inline-flex items-center justify-center gap-2 px-5 py-2.5 bg-primary-500 hover:bg-primary-400 text-neutral-900 text-sm font-semibold transition-colors"
							>
								<Icon name="cart" size={16} />
								Browse marketplace
							</a>
						{/if}
						<button
							type="button"
							onclick={completeOnboarding}
							class="text-xs text-neutral-500 hover:text-neutral-400 transition-colors text-center"
						>
							Skip for now
						</button>
					</div>
				</div>
			{/if}
		</div>
	</div>
{/if}
