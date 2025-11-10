<script lang="ts">
	import { authStore } from '$lib/stores/auth';
	import SeedPhraseDialog from './SeedPhraseDialog.svelte';

	let { open = $bindable(false) } = $props();
	let isLoading = false;
	let errorMsg = '';
	let showSeedPhraseDialog = $state(false);

	async function handleIILogin() {
		isLoading = true;
		errorMsg = '';
		try {
			await authStore.loginWithII('/dashboard');
			open = false;
		} catch (error) {
			errorMsg = 'Failed to login with Internet Identity';
			console.error(error);
		} finally {
			isLoading = false;
		}
	}

	function handleSeedPhraseClick() {
		open = false;
		showSeedPhraseDialog = true;
	}

	async function handleSeedPhraseConfirm(seedPhrase: string) {
		try {
			await authStore.loginWithSeedPhrase(seedPhrase, '/dashboard');
			showSeedPhraseDialog = false;
		} catch (error) {
			errorMsg = 'Failed to login with seed phrase';
			console.error('Failed to login with seed phrase:', error);
			showSeedPhraseDialog = false;
			open = true;
		}
	}

	function handleClose() {
		if (!isLoading) {
			open = false;
			errorMsg = '';
		}
	}
</script>

{#if open}
	<!-- Backdrop -->
	<div
		class="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center p-4"
		onclick={handleClose}
		onkeydown={(e) => e.key === 'Escape' && handleClose()}
		role="button"
		tabindex="0"
	>
		<!-- Dialog -->
		<div
			class="bg-gray-900 rounded-2xl p-8 max-w-md w-full border border-white/20 shadow-2xl"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			role="dialog"
			tabindex="-1"
		>
			<h2 class="text-3xl font-bold text-white mb-2">Connect Wallet</h2>
			<p class="text-white/60 mb-8">Choose your authentication method</p>

			{#if errorMsg}
				<div class="mb-4 p-4 bg-red-500/20 border border-red-500/30 rounded-lg text-red-400 text-sm">
					{errorMsg}
				</div>
			{/if}

			<div class="space-y-4">
				<!-- Internet Identity -->
				<button
					type="button"
					onclick={handleIILogin}
					disabled={isLoading}
					class="w-full p-4 bg-gradient-to-r from-blue-500/20 to-purple-600/20 border border-blue-500/30 rounded-xl hover:border-blue-400 transition-all group disabled:opacity-50 disabled:cursor-not-allowed"
				>
					<div class="flex items-center gap-4">
						<span class="text-4xl">🆔</span>
						<div class="text-left flex-1">
							<h3 class="text-white font-semibold group-hover:text-blue-400">Internet Identity</h3>
							<p class="text-white/60 text-sm">Secure ICP authentication</p>
						</div>
						<span class="text-white/40">→</span>
					</div>
				</button>

				<!-- Seed Phrase -->
				<button
					type="button"
					onclick={handleSeedPhraseClick}
					disabled={isLoading}
					class="w-full p-4 bg-gradient-to-r from-purple-500/20 to-pink-600/20 border border-purple-500/30 rounded-xl hover:border-purple-400 transition-all group disabled:opacity-50 disabled:cursor-not-allowed"
				>
					<div class="flex items-center gap-4">
						<span class="text-4xl">🔑</span>
						<div class="text-left flex-1">
							<h3 class="text-white font-semibold group-hover:text-purple-400">Seed Phrase</h3>
							<p class="text-white/60 text-sm">Generate new or import existing</p>
						</div>
						<span class="text-white/40">→</span>
					</div>
				</button>
			</div>

			<button
				type="button"
				onclick={handleClose}
				disabled={isLoading}
				class="w-full mt-6 px-4 py-3 text-white/60 hover:text-white transition-colors disabled:opacity-50"
			>
				Cancel
			</button>
		</div>
	</div>
{/if}

<SeedPhraseDialog bind:open={showSeedPhraseDialog} onConfirm={handleSeedPhraseConfirm} />
