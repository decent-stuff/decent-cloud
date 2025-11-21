<script lang="ts">
	import { authStore, type AccountInfo } from "$lib/stores/auth";
	import { Ed25519KeyIdentity } from "@dfinity/identity";
	import { validateMnemonic, mnemonicToSeedSync } from "bip39";
	import { hmac } from "@noble/hashes/hmac";
	import { sha512 } from "@noble/hashes/sha2";
	import {
		getAccountByPublicKey,
		type AccountWithKeys,
	} from "$lib/services/account-api";

	let { onSuccess, onCancel, onNeedRegistration } = $props<{
		onSuccess: (account: AccountInfo) => void;
		onCancel: () => void;
		onNeedRegistration?: (
			identity: Ed25519KeyIdentity,
			seedPhrase: string,
		) => void;
	}>();

	type Step =
		| "seed-entry"
		| "account-detection"
		| "username-entry"
		| "processing"
		| "success";

	let currentStep = $state<Step>("seed-entry");
	let seedPhrase = $state("");
	let username = $state("");
	let error = $state<string | null>(null);
	let showWords = $state(false);
	let loadedAccount = $state<AccountInfo | null>(null);

	// Generate Ed25519 identity from seed phrase
	function identityFromSeed(seedPhrase: string): Ed25519KeyIdentity {
		const seedBuffer = mnemonicToSeedSync(seedPhrase, "");
		const seedBytes = new Uint8Array(seedBuffer);
		const keyMaterial = hmac(sha512, "ed25519 seed", seedBytes);
		const derivedSeed = keyMaterial.slice(0, 32);
		return Ed25519KeyIdentity.fromSecretKey(derivedSeed);
	}

	function validateSeedPhrase() {
		const trimmed = seedPhrase.trim();

		if (!trimmed) {
			error = "Please enter your seed phrase";
			return false;
		}

		if (!validateMnemonic(trimmed)) {
			error = "Invalid seed phrase. Please check and try again.";
			return false;
		}

		error = null;
		return true;
	}

	function handlePaste(e: ClipboardEvent) {
		e.preventDefault();
		const pasted = e.clipboardData?.getData("text") || "";
		seedPhrase = pasted.trim();
	}

	async function continueSeedPhrase() {
		if (!validateSeedPhrase()) return;

		currentStep = "account-detection";
		error = null;

		try {
			// Create identity from seed phrase
			const identity = identityFromSeed(seedPhrase);
			const publicKeyBytes = new Uint8Array(
				identity.getPublicKey().rawKey,
			);
			const publicKeyHex = Array.from(publicKeyBytes)
				.map((b) => b.toString(16).padStart(2, "0"))
				.join("");

			// Try to find account by public key
			const account = await getAccountByPublicKey(publicKeyHex);

			if (account) {
				// Found account - auto-fill username and sign in directly
				username = account.username;
				await signInWithDetectedAccount(identity, account);
			} else {
				// Not found - ask for username manually
				currentStep = "username-entry";
			}
		} catch (err) {
			console.error("Account detection error:", err);
			// Fall back to username entry on error
			currentStep = "username-entry";
		}
	}

	async function signInWithDetectedAccount(
		_identity: Ed25519KeyIdentity,
		account: AccountWithKeys,
	) {
		currentStep = "processing";
		try {
			await authStore.loginWithSeedPhrase(seedPhrase, "/dashboard");
			loadedAccount = account;
			currentStep = "success";
		} catch (err) {
			error = err instanceof Error ? err.message : "Sign in failed";
			currentStep = "username-entry";
		}
	}

	async function signInWithUsernameAndSeed() {
		if (!username.trim()) {
			error = "Please enter your username";
			return;
		}

		if (!validateSeedPhrase()) return;

		currentStep = "processing";
		error = null;

		try {
			// Create identity from seed phrase
			const identity = identityFromSeed(seedPhrase);

			// Load account by username
			const account = await authStore.loadAccountByUsername(
				username.trim().toLowerCase(),
			);

			if (!account) {
				error = "Account not found";
				currentStep = "username-entry";
				return;
			}

			// Check if this public key is in the account
			const publicKeyBytes = new Uint8Array(
				identity.getPublicKey().rawKey,
			);
			const publicKeyHex = Array.from(publicKeyBytes)
				.map((b) => b.toString(16).padStart(2, "0"))
				.join("");

			const keyInAccount = account.publicKeys.some(
				(k) => k.publicKey === publicKeyHex && k.isActive,
			);

			if (!keyInAccount) {
				// Key not in account - offer to add it or register new account
				error = "This key is not associated with this account";
				currentStep = "username-entry";

				// Could offer to add key here
				if (
					onNeedRegistration &&
					confirm(
						"This key is not associated with this account. Would you like to add it or create a new account?",
					)
				) {
					onNeedRegistration(identity, seedPhrase);
				}
				return;
			}

			// Sign in with seed phrase
			await authStore.loginWithSeedPhrase(seedPhrase, "/dashboard");

			loadedAccount = account;
			currentStep = "success";
		} catch (err) {
			error = err instanceof Error ? err.message : "Sign in failed";
			currentStep = "username-entry";
		}
	}

	function handleSuccess() {
		if (loadedAccount) {
			onSuccess(loadedAccount);
		}
	}

	// Auto-format seed phrase (normalize spaces)
	$effect(() => {
		if (seedPhrase) {
			const words = seedPhrase.trim().split(/\s+/);
			if (words.length <= 12) {
				// Auto-format is fine
			}
		}
	});
</script>

<div class="space-y-6">
	<!-- Step 1: Enter Seed Phrase -->
	{#if currentStep === "seed-entry"}
		<div class="space-y-4">
			<h3 class="text-2xl font-bold text-white">
				Enter Your Seed Phrase
			</h3>
			<p class="text-white/60">
				Type or paste your 12-word recovery phrase
			</p>

			<div class="space-y-2">
				<label
					for="seedPhrase"
					class="block text-sm font-medium text-white/70"
				>
					Seed Phrase
				</label>
				<div class="relative">
					<textarea
						id="seedPhrase"
						bind:value={seedPhrase}
						onpaste={handlePaste}
						placeholder="word1 word2 word3 ..."
						rows="4"
						class="w-full px-4 py-3 bg-white/5 border border-white/20 rounded-lg text-white placeholder:text-white/30 focus:outline-none focus:ring-2 focus:ring-purple-500/50 transition-all font-mono text-sm {showWords
							? ''
							: 'blur-sm'}"
					></textarea>
					<button
						type="button"
						onclick={() => (showWords = !showWords)}
						class="absolute top-3 right-3 px-3 py-1 bg-white/10 hover:bg-white/20 rounded text-xs text-white transition-colors"
					>
						{showWords ? "🙈 Hide" : "👁️ Show"}
					</button>
				</div>

				<!-- Word counter -->
				<div class="text-xs text-white/40">
					{seedPhrase
						.trim()
						.split(/\s+/)
						.filter((w) => w).length} / 12 words
				</div>
			</div>

			{#if error}
				<div
					class="p-4 bg-red-500/20 border border-red-500/30 rounded-lg text-red-400 text-sm"
				>
					{error}
				</div>
			{/if}

			<div class="flex gap-3">
				<button
					type="button"
					onclick={onCancel}
					class="flex-1 px-4 py-3 bg-white/10 hover:bg-white/20 rounded-lg text-white transition-colors"
				>
					Cancel
				</button>
				<button
					type="button"
					onclick={continueSeedPhrase}
					class="flex-1 px-4 py-3 bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-500 hover:to-pink-500 rounded-lg text-white font-medium transition-all"
				>
					Continue
				</button>
			</div>

			<!-- Help text -->
			<div class="pt-4 border-t border-white/10">
				<button
					type="button"
					class="text-sm text-white/60 hover:text-white transition-colors"
				>
					Lost your seed phrase?
				</button>
			</div>
		</div>
	{/if}

	<!-- Step 2: Account Detection -->
	{#if currentStep === "account-detection"}
		<div class="space-y-4 text-center py-8">
			<div class="text-6xl animate-pulse">🔍</div>
			<h3 class="text-2xl font-bold text-white">Detecting Account</h3>
			<p class="text-white/60">Looking up your account...</p>
			<div class="flex justify-center">
				<div
					class="w-8 h-8 border-4 border-purple-500/30 border-t-purple-500 rounded-full animate-spin"
				></div>
			</div>
		</div>
	{/if}

	<!-- Step 3: Enter Username -->
	{#if currentStep === "username-entry"}
		<div class="space-y-4">
			<h3 class="text-2xl font-bold text-white">Enter Your Username</h3>
			<p class="text-white/60">What's your Decent Cloud username?</p>

			<div class="space-y-2">
				<label
					for="username"
					class="block text-sm font-medium text-white/70"
				>
					Username
				</label>
				<div class="relative">
					<input
						id="username"
						type="text"
						bind:value={username}
						placeholder="alice"
						autocomplete="off"
						autocapitalize="off"
						spellcheck="false"
						class="w-full px-4 py-3 bg-white/5 border border-white/20 rounded-lg text-white placeholder:text-white/30 focus:outline-none focus:ring-2 focus:ring-purple-500/50 transition-all"
					/>
				</div>
			</div>

			{#if error}
				<div
					class="p-4 bg-red-500/20 border border-red-500/30 rounded-lg text-red-400 text-sm"
				>
					{error}
				</div>
			{/if}

			<div class="flex gap-3">
				<button
					type="button"
					onclick={() => (currentStep = "seed-entry")}
					class="flex-1 px-4 py-3 bg-white/10 hover:bg-white/20 rounded-lg text-white transition-colors"
				>
					Back
				</button>
				<button
					type="button"
					onclick={signInWithUsernameAndSeed}
					class="flex-1 px-4 py-3 bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-500 hover:to-pink-500 rounded-lg text-white font-medium transition-all"
				>
					Sign In
				</button>
			</div>
		</div>
	{/if}

	<!-- Step 4: Processing -->
	{#if currentStep === "processing"}
		<div class="space-y-4 text-center py-8">
			<div class="text-6xl animate-bounce">🔐</div>
			<h3 class="text-2xl font-bold text-white">Signing You In</h3>
			<p class="text-white/60">Please wait...</p>
			<div class="flex justify-center">
				<div
					class="w-8 h-8 border-4 border-purple-500/30 border-t-purple-500 rounded-full animate-spin"
				></div>
			</div>
		</div>
	{/if}

	<!-- Step 5: Success -->
	{#if currentStep === "success" && loadedAccount}
		<div class="space-y-4 text-center py-8">
			<div class="text-6xl">👋</div>
			<h3 class="text-2xl font-bold text-white">Welcome Back!</h3>
			<p class="text-white/60">
				Signed in as <span class="text-white font-medium"
					>@{loadedAccount.username}</span
				>
			</p>

			<div class="pt-4">
				<button
					type="button"
					onclick={handleSuccess}
					class="px-8 py-3 bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-500 hover:to-pink-500 rounded-lg text-white font-medium transition-all"
				>
					Go to Dashboard
				</button>
			</div>
		</div>
	{/if}
</div>
