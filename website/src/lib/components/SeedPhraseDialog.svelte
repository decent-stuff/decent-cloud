<script lang="ts">
	import { generateMnemonic, validateMnemonic } from 'bip39';

	let { open = $bindable(false), onConfirm } = $props<{
		open: boolean;
		onConfirm: (seedPhrase: string) => void;
	}>();

	let seedPhrase = $state('');
	let error = $state('');
	let mode = $state<'choice' | 'generate' | 'import'>('choice');
	let copySuccess = $state(false);

	const buttonBaseClass = 'flex-1 px-6 py-3 rounded-lg text-white transition-colors';
	const buttonSecondaryClass = `${buttonBaseClass} bg-white/10 border border-white/20 hover:bg-white/20`;
	const buttonPrimaryClass = (gradient: string) =>
		`${buttonBaseClass} bg-gradient-to-r ${gradient} font-semibold hover:brightness-110 transition-all`;

	function resetState() {
		mode = 'choice';
		seedPhrase = '';
		error = '';
		copySuccess = false;
	}

	async function copyToClipboard() {
		try {
			await navigator.clipboard.writeText(seedPhrase);
			copySuccess = true;
			setTimeout(() => {
				copySuccess = false;
			}, 2000);
		} catch (err) {
			console.error('Failed to copy:', err);
		}
	}

	function handleGenerateNew() {
		seedPhrase = generateMnemonic();
		mode = 'generate';
		error = '';
	}

	function handleImportExisting() {
		seedPhrase = '';
		mode = 'import';
		error = '';
	}

	function handleBack() {
		resetState();
	}

	function handleConfirm() {
		const normalized = seedPhrase
			.trim()
			.replace(/\s+/g, ' ');

		if (!normalized) {
			error = 'Please enter your seed phrase';
			return;
		}

		if (!validateMnemonic(normalized)) {
			error = 'Invalid seed phrase. Must be a valid 12 or 24-word mnemonic';
			return;
		}

		try {
			onConfirm(normalized);
			handleClose();
		} catch (err) {
			error = `Invalid seed phrase: ${err instanceof Error ? err.message : String(err)}`;
		}
	}

	function handleClose() {
		if (open) {
			open = false;
			resetState();
		}
	}
</script>

{#if open}
	<div
		class="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center p-4"
		onclick={handleClose}
		onkeydown={(e) => e.key === 'Escape' && handleClose()}
		role="button"
		tabindex="0"
	>
		<div
			class="bg-gray-900 rounded-2xl p-8 max-w-2xl w-full border border-white/20 shadow-2xl"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			role="dialog"
			tabindex="-1"
		>
			{#if mode === 'choice'}
				<h2 class="text-3xl font-bold text-white mb-2">Seed Phrase Authentication</h2>
				<p class="text-white/60 mb-8">Choose how you want to proceed</p>
				<div class="space-y-4">
					<button
						type="button"
						onclick={handleGenerateNew}
						class="w-full p-6 bg-gradient-to-r from-blue-500/20 to-purple-600/20 border border-blue-500/30 rounded-xl hover:border-blue-400 transition-all group text-left"
					>
						<div class="flex items-center gap-4">
							<span class="text-4xl">✨</span>
							<div class="flex-1">
								<h3 class="text-white font-semibold text-lg group-hover:text-blue-400">
									Generate New Seed Phrase
								</h3>
								<p class="text-white/60 text-sm mt-1">Create a new identity with a fresh 12-word seed phrase</p>
							</div>
							<span class="text-white/40 text-2xl">→</span>
						</div>
					</button>
					<button
						type="button"
						onclick={handleImportExisting}
						class="w-full p-6 bg-gradient-to-r from-purple-500/20 to-pink-600/20 border border-purple-500/30 rounded-xl hover:border-purple-400 transition-all group text-left"
					>
						<div class="flex items-center gap-4">
							<span class="text-4xl">🔑</span>
							<div class="flex-1">
								<h3 class="text-white font-semibold text-lg group-hover:text-purple-400">
									Import Existing Seed Phrase
								</h3>
								<p class="text-white/60 text-sm mt-1">Use your existing 12 or 24-word seed phrase</p>
							</div>
							<span class="text-white/40 text-2xl">→</span>
						</div>
					</button>
				</div>
			{:else if mode === 'generate'}
				<h2 class="text-3xl font-bold text-white mb-2">Your New Seed Phrase</h2>
				<p class="text-white/60 mb-6">
					<strong class="text-yellow-400">⚠️ IMPORTANT:</strong> Write down these words and store them safely. This is the ONLY way to recover your account.
				</p>
				<div class="bg-gray-800 border border-white/20 rounded-xl p-6 mb-4">
					<div class="grid grid-cols-3 gap-4 mb-4">
						{#each seedPhrase.split(' ') as word, i}
							<div class="flex items-center gap-2 text-white">
								<span class="text-white/40 text-sm w-6">{i + 1}.</span>
								<span class="font-mono font-semibold">{word}</span>
							</div>
						{/each}
					</div>
					<div class="pt-4 border-t border-white/10">
						<p class="text-white/50 text-xs mb-2">Select and copy:</p>
						<p class="font-mono text-sm text-white bg-gray-900 p-3 rounded select-all break-all">
							{seedPhrase}
						</p>
					</div>
				</div>
				<button
					type="button"
					onclick={copyToClipboard}
					class="w-full mb-6 px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white hover:bg-white/20 transition-colors flex items-center justify-center gap-2"
				>
					{#if copySuccess}
						<span class="text-green-400">✓ Copied to clipboard!</span>
					{:else}
						<span>📋 Copy seed phrase to clipboard</span>
					{/if}
				</button>
				<div class="bg-red-900/20 border border-red-500/30 rounded-lg p-4 mb-6">
					<p class="text-red-400 text-sm">
						<strong>Never share your seed phrase with anyone!</strong> Anyone with access to these words can control your account and funds. There is NO way to recover a lost seed phrase.
					</p>
				</div>
				<div class="flex gap-4">
					<button type="button" onclick={handleBack} class={buttonSecondaryClass}>Back</button>
					<button type="button" onclick={handleConfirm} class={buttonPrimaryClass('from-blue-500 to-purple-600')}>
						I've Saved It, Continue
					</button>
				</div>
			{:else if mode === 'import'}
				<h2 class="text-3xl font-bold text-white mb-2">Import Seed Phrase</h2>
				<p class="text-white/60 mb-6">Enter your 12 or 24-word seed phrase</p>
				{#if error}
					<div class="mb-4 p-4 bg-red-500/20 border border-red-500/30 rounded-lg text-red-400 text-sm">{error}</div>
				{/if}
				<textarea
					bind:value={seedPhrase}
					placeholder="word1 word2 word3 ..."
					class="w-full h-32 px-4 py-3 bg-gray-800 border border-white/20 rounded-lg text-white font-mono text-sm focus:outline-none focus:border-purple-500 resize-none mb-6"
					oninput={() => (error = '')}
				></textarea>
				<div class="bg-blue-900/20 border border-blue-500/30 rounded-lg p-4 mb-6">
					<p class="text-blue-400 text-sm">💡 Tip: Paste your seed phrase directly. Words can be separated by spaces or newlines.</p>
				</div>
				<div class="flex gap-4">
					<button type="button" onclick={handleBack} class={buttonSecondaryClass}>Back</button>
					<button type="button" onclick={handleConfirm} class={buttonPrimaryClass('from-purple-500 to-pink-600')}>
						Import & Continue
					</button>
				</div>
			{/if}

			<button
				type="button"
				onclick={handleClose}
				class="w-full mt-6 px-4 py-3 text-white/60 hover:text-white transition-colors"
			>
				Cancel
			</button>
		</div>
	</div>
{/if}
