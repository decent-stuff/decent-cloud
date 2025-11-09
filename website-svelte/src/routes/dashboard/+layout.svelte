<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { authStore } from '$lib/stores/auth';
	import DashboardSidebar from '$lib/components/DashboardSidebar.svelte';

	let { children } = $props();
	let isAuthenticated = false;

	onMount(() => {
		authStore.isAuthenticated.subscribe((value) => {
			isAuthenticated = value;
			if (!value) {
				goto('/');
			}
		});
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
