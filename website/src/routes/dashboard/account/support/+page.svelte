<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { page } from "$app/stores";
	import { authStore } from "$lib/stores/auth";
	import { navigateToLogin } from "$lib/utils/navigation";
	import { getSupportPortalStatus, resetSupportPortalPassword, type SupportPortalStatus } from "$lib/services/chatwoot-api";
	import type { IdentityInfo } from "$lib/stores/auth";

	let currentIdentity = $state<IdentityInfo | null>(null);
	let isAuthenticated = $state(false);
	let unsubscribe: (() => void) | null = null;
	let unsubscribeAuth: (() => void) | null = null;

	let status = $state<SupportPortalStatus | null>(null);
	let loading = $state(true);
	let resetting = $state(false);
	let newPassword = $state<string | null>(null);
	let error = $state<string | null>(null);

	onMount(() => {
		unsubscribeAuth = authStore.isAuthenticated.subscribe((isAuth) => {
			isAuthenticated = isAuth;
		});
		unsubscribe = authStore.currentIdentity.subscribe((value) => {
			currentIdentity = value;
			if (value?.identity) loadStatus();
		});
	});

	onDestroy(() => {
		unsubscribe?.();
		unsubscribeAuth?.();
	});

	function handleLogin() {
		navigateToLogin($page.url.pathname);
	}

	async function loadStatus() {
		if (!currentIdentity?.identity) return;
		loading = true;
		error = null;
		status = await getSupportPortalStatus(currentIdentity.identity);
		loading = false;
	}

	async function handleReset() {
		if (!currentIdentity?.identity) return;
		resetting = true;
		error = null;
		newPassword = null;
		const result = await resetSupportPortalPassword(currentIdentity.identity);
		resetting = false;
		if (result) {
			newPassword = result.password;
			await loadStatus();
		} else {
			error = "Failed to reset password. Please try again.";
		}
	}

	function copyPassword() {
		if (newPassword) navigator.clipboard.writeText(newPassword);
	}
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">Support Portal</h1>
		<p class="text-white/60">Access your support portal account for ticket management</p>
	</div>

	{#if !isAuthenticated}
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-8 border border-white/20 text-center">
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">🎫</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-white/70">Login to access your support portal settings.</p>
				<button onclick={handleLogin}
					class="px-8 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 hover:scale-105 transition-all">
					Login / Create Account
				</button>
			</div>
		</div>
	{:else if loading}
		<p class="text-white/60">Loading...</p>
	{:else if status}
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20 space-y-6">
			<h2 class="text-2xl font-bold text-white">Account Status</h2>
			<div class="space-y-3">
				<div>
					<p class="text-white/70 text-sm">Status</p>
					<p class="text-white font-semibold">
						{status.hasAccount ? "Active" : "Not created"}
					</p>
				</div>
				{#if status.userId}
					<div>
						<p class="text-white/70 text-sm">User ID</p>
						<p class="text-white font-mono">{status.userId}</p>
					</div>
				{/if}
				{#if status.email}
					<div>
						<p class="text-white/70 text-sm">Email</p>
						<p class="text-white">{status.email}</p>
					</div>
				{/if}
				<div>
					<p class="text-white/70 text-sm">Login URL</p>
					<a href={status.loginUrl} target="_blank" rel="noopener noreferrer"
						class="text-blue-400 hover:text-blue-300 underline">{status.loginUrl}</a>
				</div>
			</div>
		</div>

		{#if status.hasAccount}
			<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20 space-y-4">
				<h2 class="text-xl font-semibold text-white">Password Reset</h2>
				<p class="text-white/70 text-sm">
					Generate a new password for your support portal account. The password will be displayed once - save it securely.
				</p>
				{#if newPassword}
					<div class="bg-green-500/20 border border-green-500/50 rounded-lg p-4 space-y-3">
						<p class="text-green-300 font-semibold">New password generated:</p>
						<div class="flex items-center gap-2">
							<code class="bg-black/30 px-3 py-2 rounded font-mono text-white flex-1">{newPassword}</code>
							<button onclick={copyPassword}
								class="px-4 py-2 bg-white/10 rounded hover:bg-white/20 text-white transition-colors">
								Copy
							</button>
						</div>
						<p class="text-white/60 text-xs">This password will not be shown again. Save it now.</p>
					</div>
				{/if}
				{#if error}
					<p class="text-red-400">{error}</p>
				{/if}
				<button onclick={handleReset} disabled={resetting}
					class="px-6 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 transition-all disabled:opacity-50">
					{resetting ? "Resetting..." : "Reset Password"}
				</button>
			</div>
		{:else}
			<div class="bg-yellow-500/20 border border-yellow-500/50 rounded-xl p-6">
				<p class="text-yellow-300">
					Your support portal account will be created when you contact support for the first time.
				</p>
			</div>
		{/if}
	{:else}
		<div class="bg-red-500/20 border border-red-500/50 rounded-xl p-6">
			<p class="text-red-300">Failed to load support portal status. Please try again later.</p>
		</div>
	{/if}
</div>
