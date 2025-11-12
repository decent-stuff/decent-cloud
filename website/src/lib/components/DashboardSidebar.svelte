<script lang="ts">
	import { page } from '$app/stores';
	import { authStore } from '$lib/stores/auth';

	let { isOpen = $bindable(false) } = $props();

	const navItems = [
		{ href: '/dashboard', icon: '📊', label: 'Overview' },
		{ href: '/dashboard/validators', icon: '✓', label: 'Validators' },
		{ href: '/dashboard/offerings', icon: '📦', label: 'My Offerings' },
		{ href: '/dashboard/marketplace', icon: '🛒', label: 'Marketplace' }
	];

	let currentPath = $state('');
	page.subscribe((p) => {
		currentPath = p.url.pathname;
	});

	async function handleLogout() {
		await authStore.logout();
		window.location.href = '/';
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
		<a href="/" class="text-2xl font-bold text-white hover:text-blue-400 transition-colors">
			Decent Cloud
		</a>
	</div>

	<!-- Navigation -->
	<nav class="flex-1 p-4 space-y-2">
		{#each navItems as item}
			<a
				href={item.href}
				onclick={closeSidebar}
				class="flex items-center gap-3 px-4 py-3 rounded-lg transition-all {currentPath ===
				item.href
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
		<a
			href="/dashboard/profile"
			onclick={closeSidebar}
			class="flex items-center gap-3 px-4 py-3 rounded-lg transition-all {currentPath ===
			'/dashboard/profile'
				? 'bg-blue-600 text-white'
				: 'text-white/70 hover:bg-white/10 hover:text-white'}"
		>
			<span class="text-xl">👤</span>
			<span class="font-medium">Profile</span>
		</a>
		<button
			type="button"
			onclick={handleLogout}
			class="w-full px-4 py-3 text-left rounded-lg text-white/70 hover:bg-white/10 hover:text-white transition-all flex items-center gap-3"
		>
			<span class="text-xl">🚪</span>
			<span class="font-medium">Logout</span>
		</button>
	</div>
</aside>
