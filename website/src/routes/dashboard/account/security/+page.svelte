<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { authStore } from "$lib/stores/auth";
	import SettingsTabs from "$lib/components/SettingsTabs.svelte";
	import AccountOverview from "$lib/components/AccountOverview.svelte";
	import AuthRequiredCard from "$lib/components/AuthRequiredCard.svelte";
	import ExternalKeysEditor from "$lib/components/ExternalKeysEditor.svelte";
	import { UserApiClient } from "$lib/services/user-api";
	import { Ed25519KeyIdentity } from "@dfinity/identity";
	import type { IdentityInfo } from "$lib/stores/auth";
	import { bytesToHex as hexEncode } from "$lib/utils/identity";

	let currentIdentity = $state<IdentityInfo | null>(null);
	let isAuthenticated = $state(false);
	let unsubscribe: (() => void) | null = null;
	let unsubscribeAuth: (() => void) | null = null;

	// TOTP 2FA state
	let totpEnabled = $state(false);
	let totpHasBackupCodes = $state(false);
	let totpLoading = $state(false);
	let totpError = $state<string | null>(null);
	// setup flow
	let totpSetupUri = $state<string | null>(null);
	let totpSetupSecret = $state<string | null>(null);
	let totpSetupCode = $state('');
	let totpSetupStep = $state<'idle' | 'scan' | 'confirm' | 'backup'>('idle');
	let totpBackupCodes = $state<string[]>([]);
	// disable flow
	let totpDisableCode = $state('');
	let totpDisabling = $state(false);
	// regenerate backup codes flow
	let totpRegenCode = $state('');
	let totpRegenStep = $state<'idle' | 'confirm'>('idle');

	// API token state
	type ApiToken = { id: string; name: string; createdAt: number; lastUsedAt?: number; expiresAt?: number; isActive: boolean };
	let tokens = $state<ApiToken[]>([]);
	let tokensLoading = $state(false);
	let tokensError = $state<string | null>(null);

	// Create token modal state
	let showCreateModal = $state(false);
	let newTokenName = $state('');
	let newTokenExpiry = $state<number | null>(null);
	let creating = $state(false);
	let createdToken = $state<string | null>(null);
	let copied = $state(false);

	// Revoke confirmation state
	let revokingId = $state<string | null>(null);

	onMount(() => {
		unsubscribeAuth = authStore.isAuthenticated.subscribe((isAuth) => {
			isAuthenticated = isAuth;
		});

		unsubscribe = authStore.currentIdentity.subscribe((value) => {
			currentIdentity = value;
			if (value) {
				loadTokens();
				loadTotpStatus();
			} else {
				tokens = [];
				totpEnabled = false;
			}
		});
	});

	onDestroy(() => {
		unsubscribe?.();
		unsubscribeAuth?.();
	});

	function apiClient(): UserApiClient | null {
		if (!currentIdentity?.identity) return null;
		return new UserApiClient(currentIdentity.identity as Ed25519KeyIdentity);
	}

	function pubkeyHex(): string | null {
		if (!currentIdentity?.publicKeyBytes) return null;
		return hexEncode(currentIdentity.publicKeyBytes);
	}

	async function loadTokens() {
		const client = apiClient();
		const pk = pubkeyHex();
		if (!client || !pk) return;
		tokensLoading = true;
		tokensError = null;
		try {
			tokens = await client.listApiTokens(pk);
		} catch (e) {
			tokensError = e instanceof Error ? e.message : 'Failed to load tokens';
		} finally {
			tokensLoading = false;
		}
	}

	// ── TOTP functions ───────────────────────────────────────────────────────

	async function loadTotpStatus() {
		const client = apiClient();
		if (!client) return;
		try {
			const status = await client.getTotpStatus();
			totpEnabled = status.enabled;
			totpHasBackupCodes = status.hasBackupCodes;
		} catch {
			// non-critical; silently ignore if TOTP not configured on server
		}
	}

	async function startTotpSetup() {
		const client = apiClient();
		if (!client) return;
		totpLoading = true;
		totpError = null;
		try {
			const result = await client.setupTotp();
			totpSetupUri = result.otpauthUri;
			totpSetupSecret = result.secret;
			totpSetupStep = 'scan';
		} catch (e) {
			totpError = e instanceof Error ? e.message : 'Failed to start TOTP setup';
		} finally {
			totpLoading = false;
		}
	}

	async function confirmTotpEnable() {
		const client = apiClient();
		if (!client || !totpSetupCode.trim()) return;
		totpLoading = true;
		totpError = null;
		try {
			const result = await client.enableTotp(totpSetupCode.trim());
			totpBackupCodes = result.backupCodes;
			totpEnabled = true;
			totpHasBackupCodes = true;
			totpSetupStep = 'backup';
			totpSetupCode = '';
		} catch (e) {
			totpError = e instanceof Error ? e.message : 'Invalid code — try again';
		} finally {
			totpLoading = false;
		}
	}

	function finishTotpSetup() {
		totpSetupStep = 'idle';
		totpSetupUri = null;
		totpSetupSecret = null;
		totpBackupCodes = [];
	}

	async function disableTotp() {
		const client = apiClient();
		if (!client || !totpDisableCode.trim()) return;
		totpDisabling = true;
		totpError = null;
		try {
			await client.disableTotp(totpDisableCode.trim());
			totpEnabled = false;
			totpHasBackupCodes = false;
			totpDisableCode = '';
		} catch (e) {
			totpError = e instanceof Error ? e.message : 'Invalid code';
		} finally {
			totpDisabling = false;
		}
	}

	async function regenerateBackupCodes() {
		const client = apiClient();
		if (!client || !totpRegenCode.trim()) return;
		totpLoading = true;
		totpError = null;
		try {
			const result = await client.regenerateBackupCodes(totpRegenCode.trim());
			totpBackupCodes = result.backupCodes;
			totpHasBackupCodes = true;
			totpSetupStep = 'backup';
			totpRegenStep = 'idle';
			totpRegenCode = '';
		} catch (e) {
			totpError = e instanceof Error ? e.message : 'Invalid code';
		} finally {
			totpLoading = false;
		}
	}

	async function createToken() {
		const client = apiClient();
		const pk = pubkeyHex();
		if (!client || !pk || !newTokenName.trim()) return;
		creating = true;
		tokensError = null;
		try {
			const result = await client.createApiToken(pk, newTokenName.trim(), newTokenExpiry ?? undefined);
			createdToken = result.token;
			await loadTokens();
		} catch (e) {
			tokensError = e instanceof Error ? e.message : 'Failed to create token';
		} finally {
			creating = false;
		}
	}

	async function copyToken() {
		if (!createdToken) return;
		await navigator.clipboard.writeText(createdToken);
		copied = true;
		setTimeout(() => { copied = false; }, 2000);
	}

	function closeCreateModal() {
		showCreateModal = false;
		newTokenName = '';
		newTokenExpiry = null;
		createdToken = null;
		copied = false;
	}

	async function revokeToken(tokenId: string) {
		if (revokingId !== tokenId) {
			revokingId = tokenId;
			return;
		}
		const client = apiClient();
		const pk = pubkeyHex();
		if (!client || !pk) return;
		tokensError = null;
		try {
			await client.revokeApiToken(pk, tokenId);
			revokingId = null;
			await loadTokens();
		} catch (e) {
			tokensError = e instanceof Error ? e.message : 'Failed to revoke token';
			revokingId = null;
		}
	}

	function formatDate(ns: number): string {
		return new Date(ns / 1_000_000).toLocaleDateString();
	}

	function expiryLabel(days: number | null): string {
		if (days === null) return 'Never';
		if (days === 30) return '30 days';
		if (days === 90) return '90 days';
		if (days === 365) return '1 year';
		return `${days} days`;
	}
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-2xl font-bold text-white tracking-tight">Security</h1>
		<p class="text-neutral-500">
			Manage your account credentials and device access
		</p>
	</div>

	<SettingsTabs />

	{#if !isAuthenticated}
		<AuthRequiredCard subtext="Create an account or login to manage your security settings, view active devices, and control access keys." />
	{:else if currentIdentity?.account}
		<AccountOverview account={currentIdentity.account} />

		<div>
			<h2 class="text-xl font-semibold text-white mb-4">SSH Keys</h2>
			<ExternalKeysEditor
				username={currentIdentity.account.username}
				apiClient={new UserApiClient(currentIdentity.identity as Ed25519KeyIdentity)}
			/>
		</div>

		<!-- TOTP 2FA section -->
		<div>
			<div class="flex items-center justify-between mb-4">
				<div>
					<h2 class="text-xl font-semibold text-white">Two-Factor Authentication</h2>
					<p class="text-neutral-500 text-sm mt-1">
						Add an authenticator app for extra security.
					</p>
				</div>
				{#if totpEnabled}
					<span class="px-2 py-1 text-xs bg-green-900/40 text-green-400 border border-green-800">Enabled</span>
				{:else}
					<span class="px-2 py-1 text-xs bg-neutral-800 text-neutral-400 border border-neutral-700">Disabled</span>
				{/if}
			</div>

			{#if totpError}
				<div class="p-3 bg-red-900/30 border border-red-800 text-red-400 text-sm mb-4">{totpError}</div>
			{/if}

			{#if totpSetupStep === 'idle'}
				{#if !totpEnabled}
					<button
						onclick={startTotpSetup}
						disabled={totpLoading}
						class="px-4 py-2 bg-primary-600 hover:bg-primary-500 text-white text-sm font-medium transition-colors disabled:opacity-50"
					>
						{totpLoading ? 'Loading...' : 'Set up authenticator'}
					</button>
				{:else}
					<div class="border border-neutral-800 p-4 space-y-4">
						<p class="text-sm text-neutral-400">
							Two-factor authentication is active. Use your authenticator app to log in and authorise sensitive operations.
						</p>
						{#if !totpHasBackupCodes}
							<p class="text-sm text-amber-400">No backup codes remaining. Regenerate now to avoid being locked out.</p>
						{/if}

						<!-- Regenerate backup codes -->
						{#if totpRegenStep === 'idle'}
							<button
								onclick={() => { totpRegenStep = 'confirm'; totpError = null; }}
								class="text-sm text-neutral-400 hover:text-white underline transition-colors"
							>
								Regenerate backup codes
							</button>
						{:else}
							<div class="space-y-2">
								<p class="text-sm text-neutral-400">Enter your TOTP code to generate new backup codes:</p>
								<div class="flex gap-2">
									<input
										type="text"
										bind:value={totpRegenCode}
										placeholder="6-digit code"
										maxlength="6"
										class="bg-neutral-800 border border-neutral-700 text-white px-3 py-2 text-sm font-mono w-32 focus:outline-none focus:border-primary-500"
									/>
									<button
										onclick={regenerateBackupCodes}
										disabled={totpLoading || totpRegenCode.length < 6}
										class="px-3 py-2 bg-primary-600 hover:bg-primary-500 text-white text-sm transition-colors disabled:opacity-50"
									>
										{totpLoading ? 'Working...' : 'Regenerate'}
									</button>
									<button
										onclick={() => { totpRegenStep = 'idle'; totpRegenCode = ''; totpError = null; }}
										class="px-3 py-2 border border-neutral-700 text-neutral-400 hover:text-white text-sm transition-colors"
									>
										Cancel
									</button>
								</div>
							</div>
						{/if}

						<!-- Disable TOTP -->
						<div class="border-t border-neutral-800 pt-4 space-y-2">
							<p class="text-sm text-neutral-500">Disable two-factor authentication:</p>
							<div class="flex gap-2">
								<input
									type="text"
									bind:value={totpDisableCode}
									placeholder="TOTP or backup code"
									class="bg-neutral-800 border border-neutral-700 text-white px-3 py-2 text-sm font-mono w-48 focus:outline-none focus:border-red-500"
								/>
								<button
									onclick={disableTotp}
									disabled={totpDisabling || !totpDisableCode.trim()}
									class="px-3 py-2 bg-red-700 hover:bg-red-600 text-white text-sm transition-colors disabled:opacity-50"
								>
									{totpDisabling ? 'Disabling...' : 'Disable 2FA'}
								</button>
							</div>
						</div>
					</div>
				{/if}

			{:else if totpSetupStep === 'scan'}
				<!-- Step 1: Scan QR code -->
				<div class="border border-neutral-800 p-4 space-y-4">
					<p class="text-sm text-neutral-400">
						Scan this QR code with your authenticator app (Google Authenticator, Authy, etc.):
					</p>
					<!-- QR code rendered client-side via otpauth URI shown as link -->
					<div class="bg-neutral-950 border border-neutral-700 p-4 text-center">
						<p class="text-xs text-neutral-500 mb-2">QR code (copy to authenticator)</p>
						<a
							href={totpSetupUri ?? '#'}
							class="text-xs text-primary-400 break-all font-mono"
						>{totpSetupUri}</a>
					</div>
					<div class="bg-neutral-900 border border-neutral-700 p-3">
						<p class="text-xs text-neutral-500 mb-1">Or enter manually:</p>
						<code class="text-sm text-white font-mono tracking-widest">{totpSetupSecret}</code>
					</div>
					<button
						onclick={() => { totpSetupStep = 'confirm'; totpError = null; }}
						class="px-4 py-2 bg-primary-600 hover:bg-primary-500 text-white text-sm font-medium transition-colors"
					>
						I've scanned it — next
					</button>
					<button
						onclick={() => { totpSetupStep = 'idle'; totpSetupUri = null; totpSetupSecret = null; totpError = null; }}
						class="ml-3 text-sm text-neutral-400 hover:text-white"
					>
						Cancel
					</button>
				</div>

			{:else if totpSetupStep === 'confirm'}
				<!-- Step 2: Verify code -->
				<div class="border border-neutral-800 p-4 space-y-4">
					<p class="text-sm text-neutral-400">
						Enter the 6-digit code from your authenticator app to confirm setup:
					</p>
					<div class="flex gap-2">
						<input
							type="text"
							bind:value={totpSetupCode}
							placeholder="000000"
							maxlength="6"
							class="bg-neutral-800 border border-neutral-700 text-white px-3 py-2 text-sm font-mono w-32 focus:outline-none focus:border-primary-500"
						/>
						<button
							onclick={confirmTotpEnable}
							disabled={totpLoading || totpSetupCode.length < 6}
							class="px-4 py-2 bg-primary-600 hover:bg-primary-500 text-white text-sm font-medium transition-colors disabled:opacity-50"
						>
							{totpLoading ? 'Verifying...' : 'Enable 2FA'}
						</button>
					</div>
					{#if totpError}
						<p class="text-sm text-red-400">{totpError}</p>
					{/if}
				</div>

			{:else if totpSetupStep === 'backup'}
				<!-- Step 3: Show backup codes -->
				<div class="border border-neutral-800 p-4 space-y-4">
					<p class="text-sm font-semibold text-white">Two-factor authentication enabled!</p>
					<p class="text-sm text-amber-400 border border-amber-700 bg-amber-900/20 p-3">
						Save these backup codes somewhere safe. Each code can only be used once if you lose access to your authenticator.
					</p>
					<div class="grid grid-cols-2 gap-2 font-mono text-sm text-green-400 bg-neutral-950 border border-neutral-700 p-3">
						{#each totpBackupCodes as code}
							<span>{code}</span>
						{/each}
					</div>
					<button
						onclick={finishTotpSetup}
						class="px-4 py-2 bg-primary-600 hover:bg-primary-500 text-white text-sm font-medium transition-colors"
					>
						Done — I've saved my codes
					</button>
				</div>
			{/if}
		</div>

		<!-- API Tokens section -->
		<div>
			<div class="flex items-center justify-between mb-4">
				<div>
					<h2 class="text-xl font-semibold text-white">API Tokens</h2>
					<p class="text-neutral-500 text-sm mt-1">
						Long-lived tokens for programmatic access (scripts, CI/CD).
					</p>
				</div>
				<button
					onclick={() => { showCreateModal = true; }}
					class="px-4 py-2 bg-primary-600 hover:bg-primary-500 text-white text-sm font-medium transition-colors"
				>
					Create Token
				</button>
			</div>

			{#if tokensError}
				<div class="p-3 bg-red-900/30 border border-red-800 text-red-400 text-sm mb-4">
					{tokensError}
				</div>
			{/if}

			{#if tokensLoading}
				<p class="text-neutral-500 text-sm">Loading tokens...</p>
			{:else if tokens.length === 0}
				<div class="border border-neutral-800 p-6 text-center">
					<p class="text-neutral-500 text-sm">No API tokens yet. Create one to get started.</p>
				</div>
			{:else}
				<div class="border border-neutral-800 divide-y divide-neutral-800">
					{#each tokens as token (token.id)}
						<div class="p-4 flex items-center justify-between gap-4">
							<div class="min-w-0">
								<div class="flex items-center gap-2">
									<span class="text-white font-medium truncate">{token.name}</span>
									{#if token.isActive}
										<span class="text-xs px-1.5 py-0.5 bg-green-900/40 text-green-400 border border-green-800">Active</span>
									{:else}
										<span class="text-xs px-1.5 py-0.5 bg-neutral-800 text-neutral-500 border border-neutral-700">Inactive</span>
									{/if}
								</div>
								<div class="text-xs text-neutral-500 mt-1 space-x-3">
									<span>Created {formatDate(token.createdAt)}</span>
									{#if token.lastUsedAt}
										<span>Last used {formatDate(token.lastUsedAt)}</span>
									{/if}
									{#if token.expiresAt}
										<span>Expires {formatDate(token.expiresAt)}</span>
									{:else}
										<span>Never expires</span>
									{/if}
								</div>
							</div>
							<div class="shrink-0">
								{#if revokingId === token.id}
									<div class="flex items-center gap-2">
										<span class="text-xs text-neutral-400">Confirm revoke?</span>
										<button
											onclick={() => revokeToken(token.id)}
											class="text-xs px-3 py-1.5 bg-red-700 hover:bg-red-600 text-white transition-colors"
										>
											Yes, Revoke
										</button>
										<button
											onclick={() => { revokingId = null; }}
											class="text-xs px-3 py-1.5 border border-neutral-700 text-neutral-400 hover:text-white transition-colors"
										>
											Cancel
										</button>
									</div>
								{:else}
									<button
										onclick={() => revokeToken(token.id)}
										disabled={!token.isActive}
										class="text-xs px-3 py-1.5 border border-neutral-700 text-neutral-400 hover:text-red-400 hover:border-red-700 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
									>
										Revoke
									</button>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	{:else}
		<p class="text-neutral-500">Loading...</p>
	{/if}
</div>

<!-- Create Token Modal -->
{#if showCreateModal}
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" role="dialog" aria-modal="true" aria-labelledby="create-token-title">
		<div class="bg-neutral-900 border border-neutral-700 w-full max-w-md p-6 space-y-5">
			<h3 id="create-token-title" class="text-lg font-semibold text-white">
				{#if createdToken}
					Token Created
				{:else}
					Create API Token
				{/if}
			</h3>

			{#if createdToken}
				<!-- Show the raw token once -->
				<div class="space-y-3">
					<p class="text-sm text-amber-400 border border-amber-700 bg-amber-900/20 p-3">
						Store this token securely — it will not be shown again.
					</p>
					<div class="relative">
						<pre class="bg-neutral-950 border border-neutral-700 p-3 text-xs text-green-400 font-mono break-all whitespace-pre-wrap select-all">{createdToken}</pre>
					</div>
					<button
						onclick={copyToken}
						class="w-full py-2 bg-neutral-800 hover:bg-neutral-700 text-sm text-neutral-300 transition-colors"
					>
						{copied ? 'Copied!' : 'Copy Token'}
					</button>
				</div>
				<button
					onclick={closeCreateModal}
					class="w-full py-2 bg-primary-600 hover:bg-primary-500 text-white text-sm font-medium transition-colors"
				>
					Done
				</button>
			{:else}
				<!-- Token creation form -->
				<div class="space-y-4">
					<div>
						<label for="token-name" class="block text-sm text-neutral-400 mb-1">Token name</label>
						<input
							id="token-name"
							type="text"
							bind:value={newTokenName}
							placeholder="e.g. ci-deploy, local-script"
							class="w-full bg-neutral-800 border border-neutral-700 text-white px-3 py-2 text-sm placeholder-neutral-600 focus:outline-none focus:border-primary-500"
							maxlength="100"
						/>
					</div>
					<div>
						<label for="token-expiry" class="block text-sm text-neutral-400 mb-1">Expiry</label>
						<select
							id="token-expiry"
							bind:value={newTokenExpiry}
							class="w-full bg-neutral-800 border border-neutral-700 text-white px-3 py-2 text-sm focus:outline-none focus:border-primary-500"
						>
							<option value={null}>Never expires</option>
							<option value={30}>{expiryLabel(30)}</option>
							<option value={90}>{expiryLabel(90)}</option>
							<option value={365}>{expiryLabel(365)}</option>
						</select>
					</div>
				</div>

				{#if tokensError}
					<p class="text-sm text-red-400">{tokensError}</p>
				{/if}

				<div class="flex gap-3">
					<button
						onclick={closeCreateModal}
						class="flex-1 py-2 border border-neutral-700 text-neutral-400 hover:text-white text-sm transition-colors"
					>
						Cancel
					</button>
					<button
						onclick={createToken}
						disabled={creating || !newTokenName.trim()}
						class="flex-1 py-2 bg-primary-600 hover:bg-primary-500 text-white text-sm font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
					>
						{creating ? 'Creating...' : 'Create Token'}
					</button>
				</div>
			{/if}
		</div>
	</div>
{/if}
