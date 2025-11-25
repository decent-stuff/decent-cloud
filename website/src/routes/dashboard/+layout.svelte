<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { authStore } from '$lib/stores/auth';
	import DashboardSidebar from '$lib/components/DashboardSidebar.svelte';
	import AuthPromptBanner from '$lib/components/AuthPromptBanner.svelte';

	let { children } = $props();
	let isAuthenticated = $state(false);
	let isInitialized = $state(false);
	let isSidebarOpen = $state(false);
	let unsubscribe: (() => void) | null = null;

	onMount(async () => {
		// Wait for auth to initialize before checking authentication
		await authStore.initialize();
		isInitialized = true;

		unsubscribe = authStore.isAuthenticated.subscribe((value) => {
			isAuthenticated = value;
		});
	});

	onDestroy(() => {
		unsubscribe?.();
	});

	function toggleSidebar() {
		isSidebarOpen = !isSidebarOpen;
	}
</script>

<div class="min-h-screen bg-gradient-to-br from-gray-900 via-blue-900 to-purple-900">
	<!-- Sidebar - always visible -->
	<DashboardSidebar bind:isOpen={isSidebarOpen} {isAuthenticated} />

	<!-- Mobile header with menu button - always visible -->
	<header class="fixed top-0 left-0 right-0 h-16 bg-gray-900/95 backdrop-blur-lg border-b border-white/10 flex items-center px-4 md:hidden z-30">
		<button
			type="button"
			onclick={toggleSidebar}
			class="text-white p-2 hover:bg-white/10 rounded-lg transition-colors"
			aria-label="Toggle menu"
		>
			<svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16"></path>
			</svg>
		</button>
		<span class="ml-3 text-white font-bold text-lg">Decent Cloud</span>
	</header>

	<!-- Auth prompt banner for anonymous users -->
	{#if !isAuthenticated}
		<AuthPromptBanner />
	{/if}

	<!-- Main content area -->
	<main class="md:ml-64 p-4 md:p-8 pt-20 md:pt-8 {!isAuthenticated ? 'md:pt-24' : ''}">
		<div class="max-w-7xl mx-auto">
			{@render children()}
		</div>
	</main>
</div>
