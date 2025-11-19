<script lang="ts">
	import {
		validateUsernameFormat,
		checkUsernameAvailable,
		generateUsernameSuggestions
	} from '$lib/services/account-api';

	let { value = $bindable(''), onValidChange } = $props<{
		value: string;
		onValidChange?: (valid: boolean, normalized: string) => void;
	}>();

	type ValidationState = 'idle' | 'typing' | 'validating' | 'valid' | 'invalid' | 'taken';

	let validationState = $state<ValidationState>('idle');
	let errorMessage = $state<string | null>(null);
	let suggestions = $state<string[]>([]);
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;
	let normalized = $state('');

	// Character counter
	let charCount = $derived(normalized.length);
	let charCountColor = $derived(
		charCount < 3
			? 'text-red-400'
			: charCount > 64
				? 'text-red-400'
				: charCount > 50
					? 'text-yellow-400'
					: 'text-white/40'
	);

	async function validateUsername(username: string) {
		// Normalize
		normalized = username.trim().toLowerCase();

		// Clear previous state
		errorMessage = null;
		suggestions = [];

		if (!normalized) {
			validationState = 'idle';
			onValidChange?.(false, '');
			return;
		}

		// Format validation
		const formatError = validateUsernameFormat(normalized);
		if (formatError) {
			validationState = 'invalid';
			errorMessage = formatError;
			onValidChange?.(false, normalized);
			return;
		}

		// Availability check
		validationState = 'validating';

		try {
			const available = await checkUsernameAvailable(normalized);

			if (available) {
				validationState = 'valid';
				onValidChange?.(true, normalized);
			} else {
				validationState = 'taken';
				errorMessage = 'Username already taken';
				suggestions = generateUsernameSuggestions(normalized);
				onValidChange?.(false, normalized);
			}
		} catch (error) {
			validationState = 'invalid';
			errorMessage = 'Error checking availability';
			onValidChange?.(false, normalized);
		}
	}

	function handleInput(e: Event) {
		const input = e.target as HTMLInputElement;
		value = input.value;

		// Clear previous timer
		if (debounceTimer) {
			clearTimeout(debounceTimer);
		}

		// Set state to typing
		if (value.trim()) {
			validationState = 'typing';
		} else {
			validationState = 'idle';
		}

		// Debounce validation (300ms)
		debounceTimer = setTimeout(() => {
			validateUsername(value);
		}, 300);
	}

	function selectSuggestion(suggestion: string) {
		value = suggestion;
		validateUsername(suggestion);
	}

	// Validation state icons
	const stateIcon = $derived(
		validationState === 'valid'
			? '✓'
			: validationState === 'invalid' || validationState === 'taken'
				? '✗'
				: validationState === 'validating'
					? '⊚'
					: ''
	);

	const stateColor = $derived(
		validationState === 'valid'
			? 'text-green-400 border-green-500/30'
			: validationState === 'invalid' || validationState === 'taken'
				? 'text-red-400 border-red-500/30'
				: validationState === 'validating'
					? 'text-blue-400 border-blue-500/30'
					: 'border-white/20'
	);
</script>

<div class="space-y-2">
	<label for="username" class="block text-sm font-medium text-white/70">
		Choose your username
	</label>

	<div class="relative">
		<input
			id="username"
			type="text"
			bind:value
			oninput={handleInput}
			placeholder="alice"
			autocomplete="off"
			autocapitalize="off"
			spellcheck="false"
			class="w-full px-4 py-3 bg-white/5 border {stateColor} rounded-lg text-white placeholder:text-white/30 focus:outline-none focus:ring-2 focus:ring-blue-500/50 transition-all"
		/>

		{#if stateIcon}
			<div
				class="absolute right-4 top-1/2 -translate-y-1/2 {stateColor} text-lg pointer-events-none"
			>
				{#if validationState === 'validating'}
					<span class="inline-block animate-spin">⊚</span>
				{:else}
					{stateIcon}
				{/if}
			</div>
		{/if}
	</div>

	<!-- Character counter -->
	<div class="flex justify-between items-center text-xs">
		<div class="text-white/40">
			3-64 characters, lowercase letters, numbers, ._@-
		</div>
		<div class="{charCountColor}">
			{charCount}/64
		</div>
	</div>

	<!-- Error message -->
	{#if errorMessage}
		<div class="text-sm text-red-400 flex items-start gap-2">
			<span>⚠️</span>
			<span>{errorMessage}</span>
		</div>
	{/if}

	<!-- Suggestions -->
	{#if suggestions.length > 0}
		<div class="space-y-2">
			<div class="text-sm text-white/60">Try these instead:</div>
			<div class="flex flex-wrap gap-2">
				{#each suggestions as suggestion}
					<button
						type="button"
						onclick={() => selectSuggestion(suggestion)}
						class="px-3 py-1 bg-white/10 hover:bg-white/20 border border-white/20 rounded-lg text-sm text-white transition-colors"
					>
						{suggestion}
					</button>
				{/each}
			</div>
		</div>
	{/if}

	<!-- Validation feedback -->
	{#if validationState === 'valid'}
		<div class="text-sm text-green-400 flex items-center gap-2">
			<span>✓</span>
			<span>Username available!</span>
		</div>
	{/if}
</div>
