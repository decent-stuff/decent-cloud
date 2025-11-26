<script lang="ts">
	import { page } from "$app/stores";
	import { authStore } from "$lib/stores/auth";
	import { navigateToLogin } from "$lib/utils/navigation";
	import { onMount, onDestroy } from "svelte";
	import type { IdentityInfo } from "$lib/stores/auth";

	let { isOpen = $bindable(false), isAuthenticated = false } = $props();

	const navItems = $derived([
		{ href: "/dashboard/marketplace", icon: "🛒", label: "Marketplace" },
		{
			href: "/dashboard/reputation",
			icon: "⭐",
			label: "Reputation",
		},
		{ href: "/dashboard/offerings", icon: "📦", label: "My Offerings" },
		{ href: "/dashboard/validators", icon: "✓", label: "Validators" },
	]);

	let currentPath = $state("");
	let currentIdentity = $state<IdentityInfo | null>(null);
	let unsubscribeIdentity: (() => void) | null = null;

	page.subscribe((p) => {
		currentPath = p.url.pathname;
	});

	onMount(() => {
		unsubscribeIdentity = authStore.currentIdentity.subscribe((value) => {
			currentIdentity = value;
		});
	});

	onDestroy(() => {
		unsubscribeIdentity?.();
	});

	async function handleLogout() {
		await authStore.logout();
		window.location.href = "/";
	}

	function handleLogin() {
		closeSidebar();
		navigateToLogin(currentPath);
	}

	function closeSidebar() {
		isOpen = false;
	}
</script>

<!-- Mobile overlay -->
{#if isOpen}
	<button
		type="button"
		class="fixed inset-0 bg-black/50 z-40 md:hidden"
		onclick={closeSidebar}
		aria-label="Close sidebar"
	></button>
{/if}

<aside
	class="fixed left-0 top-0 h-screen w-64 bg-gray-900/95 backdrop-blur-lg border-r border-white/10 flex flex-col z-50 transition-transform duration-300 {isOpen
		? 'translate-x-0'
		: '-translate-x-full md:translate-x-0'}"
>
	<!-- Logo -->
	<div class="p-6 border-b border-white/10">
		<a
			href="/"
			class="text-2xl font-bold text-white hover:text-blue-400 transition-colors"
		>
			Decent Cloud
		</a>
	</div>

	<!-- Navigation -->
	<nav class="flex-1 p-4 space-y-2">
		{#each navItems as item}
			{@const isActive =
				currentPath === item.href ||
				(item.label === "Reputation" &&
					currentPath.startsWith("/dashboard/reputation"))}
			<a
				href={item.href}
				onclick={closeSidebar}
				class="flex items-center gap-3 px-4 py-3 rounded-lg transition-all {isActive
					? 'bg-blue-600 text-white'
					: 'text-white/70 hover:bg-white/10 hover:text-white'}"
			>
				<span class="text-xl">{item.icon}</span>
				<span class="font-medium">{item.label}</span>
			</a>
		{/each}
	</nav>

	<!-- User Section -->
	<div class="p-4 border-t border-white/10 space-y-2">
		{#if isAuthenticated}
			{#if currentIdentity?.account}
				<a
					href="/dashboard/account"
					onclick={closeSidebar}
					class="block px-4 py-2 text-white/90 hover:text-white transition-colors text-center border-b border-white/10 mb-2"
					title="View account settings"
				>
					<span class="font-medium">@{currentIdentity.account.username}</span>
				</a>
			{/if}
			<a
				href="/dashboard/account"
				onclick={closeSidebar}
				class="flex items-center gap-3 px-4 py-3 rounded-lg transition-all {currentPath.startsWith(
					'/dashboard/account',
				)
					? 'bg-blue-600 text-white'
					: 'text-white/70 hover:bg-white/10 hover:text-white'}"
			>
				<span class="text-xl">⚙️</span>
				<span class="font-medium">Account</span>
			</a>
			<button
				type="button"
				onclick={handleLogout}
				class="w-full px-4 py-3 text-left rounded-lg text-white/70 hover:bg-white/10 hover:text-white transition-all flex items-center gap-3"
			>
				<span class="text-xl">🚪</span>
				<span class="font-medium">Logout</span>
			</button>
		{:else}
			<button
				type="button"
				onclick={handleLogin}
				class="w-full px-4 py-3 rounded-lg bg-gradient-to-r from-blue-500 to-purple-600 text-white font-semibold hover:brightness-110 transition-all flex items-center gap-3 justify-center"
			>
				<span class="text-xl">🔐</span>
				<span class="font-medium">Login / Create Account</span>
			</button>
		{/if}
	</div>
</aside>
