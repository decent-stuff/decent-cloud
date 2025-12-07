<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { page } from "$app/stores";
	import { authStore } from "$lib/stores/auth";
	import { navigateToLogin } from "$lib/utils/navigation";
	import { computePubkey } from "$lib/utils/contract-format";
	import type { IdentityInfo } from "$lib/stores/auth";

	let currentIdentity = $state<IdentityInfo | null>(null);
	let isAuthenticated = $state(false);
	let unsubscribe: (() => void) | null = null;
	let unsubscribeAuth: (() => void) | null = null;
	let currentPath = $state("");

	const myPubkey = $derived(
		currentIdentity?.publicKeyBytes
			? computePubkey(currentIdentity.publicKeyBytes)
			: null,
	);

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
	];
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">Account Settings</h1>
		<p class="text-white/60">
			Manage your account, security, and public profile
		</p>
	</div>

	{#if !isAuthenticated}
		<!-- Anonymous user view - login prompt -->
		<div
			class="bg-white/10 backdrop-blur-lg rounded-xl p-8 border border-white/20 text-center"
		>
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">🔐</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-white/70">
					Create an account or login to access your account settings,
					manage security, and edit your public profile.
				</p>
				<button
					onclick={handleLogin}
					class="px-8 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 hover:scale-105 transition-all"
				>
					Login / Create Account
				</button>
			</div>
		</div>
	{:else if currentIdentity?.account}
		<!-- Account Overview Card -->
		<div
			class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20"
		>
			<h2 class="text-2xl font-bold text-white mb-4">Account Overview</h2>
			<div class="space-y-3">
				<div>
					<p class="text-white/70 text-sm">Username</p>
					<p class="text-white font-semibold text-lg">
						@{currentIdentity.account.username}
					</p>
				</div>
				<div>
					<p class="text-white/70 text-sm">Account ID</p>
					<p class="text-white/60 font-mono text-xs">
						{currentIdentity.account.id.substring(0, 16)}...
					</p>
				</div>
				<div>
					<p class="text-white/70 text-sm">Created</p>
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
					<p class="text-white/70 text-sm">Active Keys</p>
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
			</div>
		</div>

		<!-- Settings -->
		<div
			class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20"
		>
			<h2 class="text-xl font-semibold text-white mb-4">Settings</h2>
			<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
				{#each settingsTabs as tab}
					<a
						href={tab.href}
						class="flex items-center gap-4 p-6 bg-white/5 rounded-lg border border-white/20 hover:bg-white/10 hover:border-blue-500/50 transition-all group"
					>
						<span class="text-4xl">{tab.icon}</span>
						<div>
							<h3
								class="text-lg font-semibold text-white group-hover:text-blue-400 transition-colors"
							>
								{tab.label}
							</h3>
							<p class="text-white/60 text-sm">
								{tab.description}
							</p>
						</div>
					</a>
				{/each}
			</div>
		</div>
	{:else}
		<p class="text-white/60">Loading...</p>
	{/if}
</div>
