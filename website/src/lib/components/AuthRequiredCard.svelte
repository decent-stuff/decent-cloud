<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { page } from '$app/stores';
	import { navigateToLogin } from '$lib/utils/navigation';
	import { authCardVisible } from '$lib/stores/auth-card';

	let { heading = 'Login Required', subtext = 'Please sign in to access this page.' } = $props();

	onMount(() => authCardVisible.set(true));
	onDestroy(() => authCardVisible.set(false));

	function handleLogin() {
		navigateToLogin($page.url.pathname);
	}
</script>

<div class="card p-8 border border-neutral-800 text-center">
	<div class="max-w-md mx-auto space-y-6">
		<span class="text-6xl">🔑</span>
		<h2 class="text-2xl font-bold text-white">{heading}</h2>
		<p class="text-neutral-400">{subtext}</p>
		<button
			onclick={handleLogin}
			class="btn-primary"
		>
			Login / Create Account
		</button>
	</div>
</div>
