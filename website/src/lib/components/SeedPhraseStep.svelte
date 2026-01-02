<script lang="ts">
	import { generateMnemonic, validateMnemonic } from 'bip39';

	let {
		initialMode = 'choose',
		showModeChoice = true,
		showDeviceName = false,
		onComplete,
		onBack
	} = $props<{
		initialMode?: 'choose' | 'generate' | 'import';
		showModeChoice?: boolean;
		showDeviceName?: boolean;
		onComplete: (seed: string, deviceName?: string) => void;
		onBack?: () => void;
	}>();

	type Mode = 'choose' | 'generate' | 'import';
	let mode = $state<Mode>(initialMode);
	let seedPhrase = $state('');
	let deviceName = $state('');
	let showSeedEntry = $state(false);
	let seedBackedUp = $state(false);
	let error = $state<string | null>(null);

	// Auto-generate seed when entering generate mode
	$effect(() => {
		if (mode === 'generate' && !seedPhrase) {
			seedPhrase = generateMnemonic();
		}
	});

	function chooseMode(selectedMode: 'generate' | 'import') {
		mode = selectedMode;
		error = null;
		if (selectedMode === 'generate') {
			seedPhrase = generateMnemonic();
		}
	}

	function copySeedPhrase() {
		navigator.clipboard.writeText(seedPhrase);
	}

	function confirmSeedBackup() {
		if (!seedBackedUp) {
			error = 'Please confirm you have backed up your seed phrase';
			return;
		}
		onComplete(seedPhrase, deviceName.trim() || undefined);
	}

	function validateSeedPhrase() {
		const trimmed = seedPhrase.trim();
		if (!trimmed) {
			error = 'Please enter your seed phrase';
			return false;
		}
		if (!validateMnemonic(trimmed)) {
			error = 'Invalid seed phrase. Please check and try again.';
			return false;
		}
		error = null;
		return true;
	}

	function continueWithSeed() {
		if (!validateSeedPhrase()) return;
		onComplete(seedPhrase, deviceName.trim() || undefined);
	}

	function handlePaste(e: ClipboardEvent) {
		e.preventDefault();
		const pasted = e.clipboardData?.getData('text') || '';
		seedPhrase = pasted.trim();
	}

	function handleBackClick() {
		if (mode !== 'choose' && showModeChoice) {
			mode = 'choose';
			seedPhrase = '';
			deviceName = '';
			seedBackedUp = false;
			error = null;
		} else if (onBack) {
			onBack();
		}
	}
</script>

<!-- Choose Mode -->
{#if mode === 'choose'}
	<div class="space-y-4">
		<h3 class="text-2xl font-bold text-white">Seed Phrase</h3>
		<p class="text-neutral-500">Generate a new seed phrase or import an existing one</p>

		<div class="grid gap-4">
			<button
				type="button"
				onclick={() => chooseMode('generate')}
				class="p-6 bg-gradient-to-r from-primary-500 to-primary-600 hover:from-primary-400 hover:to-primary-500  text-left transition-all group"
			>
				<div class="text-3xl mb-2">✨</div>
				<h4 class="text-xl font-bold text-white mb-1">Generate New</h4>
				<p class="text-neutral-300 text-sm">Create a new 12-word seed phrase</p>
			</button>

			<button
				type="button"
				onclick={() => chooseMode('import')}
				class="p-6 bg-surface-elevated hover:bg-surface-elevated border border-neutral-800  text-left transition-all group"
			>
				<div class="text-3xl mb-2">🔑</div>
				<h4 class="text-xl font-bold text-white mb-1">Import Existing</h4>
				<p class="text-neutral-500 text-sm">Use an existing seed phrase</p>
			</button>
		</div>

		{#if onBack}
			<button
				type="button"
				onclick={onBack}
				class="w-full px-4 py-3 bg-surface-elevated hover:bg-surface-elevated  text-white transition-colors"
			>
				Back
			</button>
		{/if}
	</div>
{/if}

<!-- Generate Mode: Backup Seed -->
{#if mode === 'generate'}
	<div class="space-y-4">
		<h3 class="text-2xl font-bold text-white">Backup Your Seed Phrase</h3>
		<p class="text-neutral-500">
			Save these 12 words in a secure location. You'll need them to recover your account.
		</p>

		<!-- Seed phrase display with 12 boxes -->
		<div class="p-4 bg-black/40 border border-neutral-800 ">
			<div class="grid grid-cols-3 gap-2 text-sm">
				{#each seedPhrase.split(' ') as word, i}
					<div class="flex items-center gap-2 p-2 bg-surface-elevated rounded">
						<span class="text-neutral-600 text-xs w-4">{i + 1}.</span>
						<span class="text-white font-mono">{word}</span>
					</div>
				{/each}
			</div>
		</div>

		<!-- Copy button -->
		<button
			type="button"
			onclick={copySeedPhrase}
			class="w-full px-4 py-3 bg-surface-elevated hover:bg-surface-elevated border border-neutral-800  text-white transition-colors flex items-center justify-center gap-2"
		>
			<span>📋</span>
			<span>Copy to Clipboard</span>
		</button>

		<!-- Warning -->
		<div class="p-4 bg-yellow-500/10 border border-yellow-500/30 ">
			<div class="flex gap-3">
				<span class="text-yellow-400 text-xl">⚠️</span>
				<div class="flex-1 space-y-2">
					<p class="text-sm text-yellow-400 font-medium">Never share your seed phrase!</p>
					<ul class="text-xs text-yellow-400/80 space-y-1 list-disc list-inside">
						<li>Anyone with these words can access your account</li>
						<li>Decent Cloud will never ask for your seed phrase</li>
						<li>Store it offline in a secure location</li>
					</ul>
				</div>
			</div>
		</div>

		<!-- Device Name (optional) -->
		{#if showDeviceName}
			<div class="space-y-2">
				<label for="device-name" class="block text-sm font-medium text-neutral-400">
					Device Name (optional)
				</label>
				<input
					id="device-name"
					type="text"
					bind:value={deviceName}
					placeholder="e.g., Laptop, Phone, Work Computer"
					class="w-full px-4 py-2 bg-surface-elevated border border-neutral-800  text-white placeholder:text-neutral-700 focus:outline-none focus:ring-2 focus:ring-primary-500/50 transition-all"
				/>
			</div>
		{/if}

		<!-- Confirmation checkbox -->
		<label class="flex items-start gap-3 cursor-pointer">
			<input
				type="checkbox"
				bind:checked={seedBackedUp}
				class="mt-1 w-5 h-5 rounded border-neutral-800 bg-surface-elevated text-primary-600 focus:ring-2 focus:ring-primary-500/50"
			/>
			<span class="text-sm text-neutral-300">
				I have saved my seed phrase in a secure location
			</span>
		</label>

		{#if error}
			<div class="p-4 bg-red-500/20 border border-red-500/30  text-red-400 text-sm">
				{error}
			</div>
		{/if}

		<div class="flex gap-3">
			<button
				type="button"
				onclick={handleBackClick}
				class="flex-1 px-4 py-3 bg-surface-elevated hover:bg-surface-elevated  text-white transition-colors"
			>
				Back
			</button>
			<button
				type="button"
				onclick={confirmSeedBackup}
				class="flex-1 px-4 py-3 bg-gradient-to-r from-primary-500 to-primary-600 hover:from-primary-400 hover:to-primary-500  text-white font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed"
				disabled={!seedBackedUp}
			>
				Continue
			</button>
		</div>
	</div>
{/if}

<!-- Import Mode: Enter Seed -->
{#if mode === 'import'}
	<div class="space-y-4">
		<h3 class="text-2xl font-bold text-white">Enter Your Seed Phrase</h3>
		<p class="text-neutral-500">Type or paste your 12-word recovery phrase</p>

		<div class="space-y-2">
			<label for="seedPhrase" class="block text-sm font-medium text-neutral-400">
				Seed Phrase
			</label>
			<div class="relative">
				<textarea
					id="seedPhrase"
					bind:value={seedPhrase}
					onpaste={handlePaste}
					placeholder="word1 word2 word3 ..."
					rows="4"
					class="w-full px-4 py-3 bg-surface-elevated border border-neutral-800  text-white placeholder:text-neutral-700 focus:outline-none focus:ring-2 focus:ring-primary-500/50 transition-all font-mono text-sm {showSeedEntry
						? ''
						: 'blur-sm'}"
				></textarea>
				<button
					type="button"
					onclick={() => (showSeedEntry = !showSeedEntry)}
					class="absolute top-3 right-3 px-3 py-1 bg-surface-elevated hover:bg-surface-elevated rounded text-xs text-white transition-colors"
				>
					{showSeedEntry ? '🙈 Hide' : '👁️ Show'}
				</button>
			</div>

			<!-- Word counter -->
			<div class="text-xs text-neutral-600">
				{seedPhrase
					.trim()
					.split(/\s+/)
					.filter((w) => w).length} / 12 words
			</div>
		</div>

		<!-- Device Name (optional) -->
		{#if showDeviceName}
			<div class="space-y-2">
				<label for="device-name-import" class="block text-sm font-medium text-neutral-400">
					Device Name (optional)
				</label>
				<input
					id="device-name-import"
					type="text"
					bind:value={deviceName}
					placeholder="e.g., Laptop, Phone, Work Computer"
					class="w-full px-4 py-2 bg-surface-elevated border border-neutral-800  text-white placeholder:text-neutral-700 focus:outline-none focus:ring-2 focus:ring-primary-500/50 transition-all"
				/>
			</div>
		{/if}

		{#if error}
			<div class="p-4 bg-red-500/20 border border-red-500/30  text-red-400 text-sm">
				{error}
			</div>
		{/if}

		<div class="flex gap-3">
			<button
				type="button"
				onclick={handleBackClick}
				class="flex-1 px-4 py-3 bg-surface-elevated hover:bg-surface-elevated  text-white transition-colors"
			>
				Back
			</button>
			<button
				type="button"
				onclick={continueWithSeed}
				class="flex-1 px-4 py-3 bg-gradient-to-r from-primary-500 to-primary-600 hover:from-primary-400 hover:to-primary-500  text-white font-medium transition-all"
			>
				Continue
			</button>
		</div>
	</div>
{/if}
