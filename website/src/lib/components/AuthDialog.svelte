<script lang="ts">
	import RegistrationFlow from './RegistrationFlow.svelte';
	import SignInFlow from './SignInFlow.svelte';
	import type { AccountInfo } from '$lib/stores/auth';

	let { open = $bindable(false), initialMode = 'welcome' } = $props<{
		open: boolean;
		initialMode?: 'welcome' | 'register' | 'signin';
	}>();

	type Mode = 'welcome' | 'register' | 'signin';

	let currentMode = $state<Mode>(initialMode);
	let errorMsg = $state('');

	function handleClose() {
		open = false;
		errorMsg = '';
		// Reset to welcome after a delay so transition is smooth
		setTimeout(() => {
			currentMode = 'welcome';
		}, 300);
	}

	function selectRegister() {
		currentMode = 'register';
	}

	function selectSignIn() {
		currentMode = 'signin';
	}

	function handleSuccess(account: AccountInfo) {
		console.log('Auth success:', account);
		open = false;
		// Navigate to dashboard (already handled by auth store)
	}

	function handleCancel() {
		if (currentMode === 'register' || currentMode === 'signin') {
			currentMode = 'welcome';
		} else {
			handleClose();
		}
	}

	// Reset mode when dialog opens
	$effect(() => {
		if (open && initialMode) {
			currentMode = initialMode;
		}
	});
</script>

{#if open}
	<!-- Backdrop -->
	<div
		class="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center p-4"
		onclick={(e) => {
			if (e.target === e.currentTarget) handleClose();
		}}
		onkeydown={(e) => e.key === 'Escape' && handleClose()}
		role="button"
		tabindex="0"
	>
		<!-- Dialog -->
		<div
			class="bg-gray-900/95 rounded-2xl p-6 md:p-8 max-w-lg w-full border border-white/20 shadow-2xl max-h-[90vh] overflow-y-auto"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			role="dialog"
			tabindex="-1"
		>
			<!-- Welcome Screen -->
			{#if currentMode === 'welcome'}
				<div class="space-y-6">
					<div class="text-center">
						<h2 class="text-3xl font-bold text-white mb-2">Welcome to Decent Cloud</h2>
						<p class="text-white/60">Get started in seconds</p>
					</div>

					{#if errorMsg}
						<div class="p-4 bg-red-500/20 border border-red-500/30 rounded-lg text-red-400 text-sm">
							{errorMsg}
						</div>
					{/if}

					<div class="space-y-3">
						<!-- Create Account -->
						<button
							type="button"
							onclick={selectRegister}
							class="w-full p-5 bg-gradient-to-r from-blue-600/20 to-purple-600/20 border border-blue-500/30 rounded-xl hover:border-blue-400 transition-all group"
						>
							<div class="flex items-center gap-4">
								<span class="text-5xl">✨</span>
								<div class="text-left flex-1">
									<h3 class="text-white font-semibold text-lg group-hover:text-blue-400">
										Create Account
									</h3>
									<p class="text-white/60 text-sm">New to Decent Cloud? Start here</p>
								</div>
								<span class="text-white/40 text-2xl">→</span>
							</div>
						</button>

						<!-- Sign In -->
						<button
							type="button"
							onclick={selectSignIn}
							class="w-full p-5 bg-gradient-to-r from-purple-600/20 to-pink-600/20 border border-purple-500/30 rounded-xl hover:border-purple-400 transition-all group"
						>
							<div class="flex items-center gap-4">
								<span class="text-5xl">🔐</span>
								<div class="text-left flex-1">
									<h3 class="text-white font-semibold text-lg group-hover:text-purple-400">
										Sign In
									</h3>
									<p class="text-white/60 text-sm">Already have an account</p>
								</div>
								<span class="text-white/40 text-2xl">→</span>
							</div>
						</button>
					</div>

					<button
						type="button"
						onclick={handleClose}
						class="w-full mt-4 px-4 py-3 text-white/60 hover:text-white transition-colors"
					>
						Cancel
					</button>
				</div>
			{/if}

			<!-- Registration Flow -->
			{#if currentMode === 'register'}
				<RegistrationFlow onSuccess={handleSuccess} onCancel={handleCancel} />
			{/if}

			<!-- Sign In Flow -->
			{#if currentMode === 'signin'}
				<SignInFlow onSuccess={handleSuccess} onCancel={handleCancel} />
			{/if}
		</div>
	</div>
{/if}
