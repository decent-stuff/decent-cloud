<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { authStore } from '$lib/stores/auth';
	import DashboardSidebar from '$lib/components/DashboardSidebar.svelte';

	let { children } = $props();
	let isAuthenticated = $state(false);
	let isInitialized = $state(false);
	let unsubscribe: (() => void) | null = null;

	onMount(async () => {
		// Wait for auth to initialize before checking authentication
		await authStore.initialize();
		isInitialized = true;

		unsubscribe = authStore.isAuthenticated.subscribe((value) => {
			isAuthenticated = value;
			if (isInitialized && !value) {
				goto('/');
			}
		});
	});

	onDestroy(() => {
		unsubscribe?.();
	});
</script>

<div class="min-h-screen bg-gradient-to-br from-gray-900 via-blue-900 to-purple-900">
	{#if isAuthenticated}
		<DashboardSidebar />
		<main class="ml-64 p-8">
			<div class="max-w-7xl mx-auto">
				{@render children()}
			</div>
		</main>
	{/if}
</div>
