<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { page } from "$app/stores";
	import { authStore } from "$lib/stores/auth";
	import { navigateToLogin } from "$lib/utils/navigation";
	import SettingsTabs from "$lib/components/SettingsTabs.svelte";
	import { deleteMyAccount } from "$lib/services/account-api";
	import type { IdentityInfo } from "$lib/stores/auth";

	let currentIdentity = $state<IdentityInfo | null>(null);
	let isAuthenticated = $state(false);
	let unsubscribe: (() => void) | null = null;
	let unsubscribeAuth: (() => void) | null = null;
	let currentPath = $state("");

	let showDeleteModal = $state(false);
	let deleteConfirmText = $state('');
	let deleteLoading = $state(false);
	let deleteError = $state<string | null>(null);

	onMount(() => {
		unsubscribeAuth = authStore.isAuthenticated.subscribe((isAuth) => {
			isAuthenticated = isAuth;
		});

		unsubscribe = authStore.currentIdentity.subscribe((value) => {
			currentIdentity = value;
		});
	});

	onDestroy(() => {
		unsubscribe?.();
		unsubscribeAuth?.();
	});

	function handleLogin() {
		navigateToLogin($page.url.pathname);
	}

	page.subscribe((p) => {
		currentPath = p.url.pathname;
	});

	async function handleDeleteAccount() {
		if (deleteConfirmText !== 'DELETE') return;
		const identity = currentIdentity?.identity;
		if (!identity) {
			deleteError = 'Not authenticated';
			return;
		}
		deleteLoading = true;
		deleteError = null;
		try {
			await deleteMyAccount(identity);
			await authStore.logout();
			window.location.href = '/';
		} catch (e) {
			deleteError = e instanceof Error ? e.message : 'Failed to delete account';
		} finally {
			deleteLoading = false;
		}
	}

	const settingsTabs = [
		{
			href: "/dashboard/account/security",
			label: "Security",
			icon: "🔐",
			description: "Manage devices and authentication keys",
		},
		{
			href: "/dashboard/account/profile",
			label: "Public Profile",
			icon: "👤",
			description: "Edit your public profile information",
		},
		{
			href: "/dashboard/account/subscription",
			label: "Subscription",
			icon: "⭐",
			description: "Manage your subscription plan",
		},
		{
			href: "/dashboard/account/billing",
			label: "Billing",
			icon: "💳",
			description: "Billing address and VAT settings for invoices",
		},
		{
			href: "/dashboard/account/notifications",
			label: "Notifications",
			icon: "🔔",
			description: "Configure email, Telegram, and SMS alerts",
		},
	];
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-2xl font-bold text-white tracking-tight">Account Settings</h1>
		<p class="text-neutral-500">
			Manage your account, security, and public profile
		</p>
	</div>

	<SettingsTabs />

	{#if !isAuthenticated}
		<!-- Anonymous user view - login prompt -->
		<div
			class="card p-8 border border-neutral-800 text-center"
		>
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">🔐</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-neutral-400">
					Create an account or login to access your account settings,
					manage security, and edit your public profile.
				</p>
				<button
					onclick={handleLogin}
					class="px-8 py-3 bg-gradient-to-r from-primary-500 to-primary-600  font-semibold text-white hover:brightness-110 hover:scale-105 transition-all"
				>
					Login / Create Account
				</button>
			</div>
		</div>
	{:else if currentIdentity?.account}
		<!-- Account Overview Card -->
		<div
			class="card p-6 border border-neutral-800"
		>
			<h2 class="text-2xl font-bold text-white mb-4">Account Overview</h2>
			<div class="space-y-3">
				<div>
					<p class="text-neutral-400 text-sm">Username</p>
					<p class="text-white font-semibold text-lg">
						@{currentIdentity.account.username}
					</p>
				</div>
				<div>
					<p class="text-neutral-400 text-sm">Public Profile</p>
					<a
						href="/dashboard/reputation/{currentIdentity.account.username}"
						class="text-primary-400 hover:text-primary-300 text-sm transition-colors"
					>
						View My Reputation →
					</a>
				</div>
				<div>
					<p class="text-neutral-400 text-sm">Created</p>
					<p class="text-white">
						{new Date(
							currentIdentity.account.createdAt / 1_000_000,
						).toLocaleDateString("en-US", {
							year: "numeric",
							month: "long",
							day: "numeric",
						})}
					</p>
				</div>
				<div>
					<p class="text-neutral-400 text-sm">Active Keys</p>
					<p class="text-white">
						{currentIdentity.account.publicKeys.filter(
							(k) => k.isActive,
						).length}
						{currentIdentity.account.publicKeys.filter(
							(k) => k.isActive,
						).length === 1
							? "key"
							: "keys"}
					</p>
				</div>
				<div>
					<p class="text-neutral-400 text-sm">Email</p>
					{#if currentIdentity.account.email}
						<div class="flex items-center gap-2 mt-0.5">
							<span class="text-white text-sm">{currentIdentity.account.email}</span>
							{#if currentIdentity.account.emailVerified}
								<span class="inline-flex items-center gap-1 px-1.5 py-0.5 text-xs bg-green-500/20 text-green-400 border border-green-500/30">
									✓ Verified
								</span>
							{:else}
								<span class="inline-flex items-center gap-1 px-1.5 py-0.5 text-xs bg-amber-500/20 text-amber-400 border border-amber-500/30">
									⚠ Unverified
								</span>
							{/if}
						</div>
						{#if !currentIdentity.account.emailVerified}
							<p class="text-neutral-500 text-xs mt-1">Check your inbox or <a href="/dashboard/account/notifications" class="text-primary-400 hover:text-primary-300">manage notifications →</a></p>
						{/if}
					{:else}
						<div class="flex items-center gap-2 mt-0.5">
							<span class="text-neutral-500 text-sm">Not set</span>
							<a href="/dashboard/account/profile" class="text-xs text-primary-400 hover:text-primary-300">Add email →</a>
						</div>
						<p class="text-neutral-500 text-xs mt-1">Required for notifications and account recovery.</p>
					{/if}
				</div>
			</div>
		</div>

		<!-- Settings -->
		<div
			class="card p-6 border border-neutral-800"
		>
			<h2 class="text-xl font-semibold text-white mb-4">Settings</h2>
			<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
				{#each settingsTabs as tab}
					<a
						href={tab.href}
						class="flex items-center gap-4 p-6 bg-surface-elevated  border border-neutral-800 hover:bg-surface-elevated hover:border-primary-500/50 transition-all group"
					>
						<span class="text-4xl">{tab.icon}</span>
						<div>
							<h3
								class="text-lg font-semibold text-white group-hover:text-primary-400 transition-colors"
							>
								{tab.label}
							</h3>
							<p class="text-neutral-500 text-sm">
								{tab.description}
							</p>
						</div>
					</a>
				{/each}
			</div>
		</div>

		<!-- Danger Zone -->
		<div class="card p-6 border border-red-800/50">
			<h2 class="text-xl font-semibold text-red-400 mb-2">Danger Zone</h2>
			<p class="text-neutral-500 text-sm mb-4">
				Permanently delete your account and all associated data. This cannot be undone.
			</p>
			<button
				onclick={() => { showDeleteModal = true; deleteConfirmText = ''; deleteError = null; }}
				class="px-4 py-2 bg-red-900/30 border border-red-700 text-red-400 hover:bg-red-900/50 transition-all text-sm"
			>
				Delete Account
			</button>
		</div>

		{#if showDeleteModal}
		<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
			<div class="card p-8 border border-red-800 max-w-md w-full mx-4 space-y-4">
				<h3 class="text-xl font-bold text-red-400">Delete Account</h3>
				<p class="text-neutral-300 text-sm">
					This will permanently delete your account, all offerings, and all associated data.
					Active contracts will be preserved but unlinked from your account.
					<strong class="text-white"> This cannot be undone.</strong>
				</p>
				<div>
					<label for="delete-confirm" class="block text-neutral-400 text-sm mb-1">
						Type <span class="font-mono font-bold text-white">DELETE</span> to confirm:
					</label>
					<input
						type="text"
						id="delete-confirm"
						bind:value={deleteConfirmText}
						placeholder="DELETE"
						class="w-full px-3 py-2 bg-neutral-900 border border-neutral-700 text-white font-mono"
					/>
				</div>
				{#if deleteError}
					<p class="text-red-400 text-sm">{deleteError}</p>
				{/if}
				<div class="flex gap-3 justify-end">
					<button
						onclick={() => showDeleteModal = false}
						class="px-4 py-2 text-neutral-400 hover:text-white transition-colors text-sm"
						disabled={deleteLoading}
					>
						Cancel
					</button>
					<button
						onclick={handleDeleteAccount}
						disabled={deleteConfirmText !== 'DELETE' || deleteLoading}
						class="px-4 py-2 bg-red-700 text-white hover:bg-red-600 disabled:opacity-40 disabled:cursor-not-allowed transition-all text-sm"
					>
						{deleteLoading ? 'Deleting...' : 'Delete Account'}
					</button>
				</div>
			</div>
		</div>
		{/if}
	{:else}
		<p class="text-neutral-500">Loading...</p>
	{/if}
</div>
