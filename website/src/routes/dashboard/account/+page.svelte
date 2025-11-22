<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { page } from "$app/stores";
	import { authStore } from "$lib/stores/auth";
	import type { IdentityInfo } from "$lib/stores/auth";

	let currentIdentity = $state<IdentityInfo | null>(null);
	let unsubscribe: (() => void) | null = null;
	let currentPath = $state("");

	onMount(() => {
		unsubscribe = authStore.currentIdentity.subscribe((value) => {
			currentIdentity = value;
		});
	});

	onDestroy(() => {
		unsubscribe?.();
	});

	page.subscribe((p) => {
		currentPath = p.url.pathname;
	});

	const tabs = [
		{ href: "/dashboard/account/security", label: "Security", icon: "🔐" },
		{
			href: "/dashboard/account/profile",
			label: "Public Profile",
			icon: "👤",
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

	{#if currentIdentity?.account}
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

		<!-- Navigation Tabs -->
		<div
			class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20"
		>
			<h2 class="text-xl font-semibold text-white mb-4">Settings</h2>
			<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
				{#each tabs as tab}
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
								{#if tab.label === "Security"}
									Manage devices and authentication keys
								{:else if tab.label === "Public Profile"}
									Edit your public profile information
								{/if}
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
