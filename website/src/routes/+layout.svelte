<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import { authStore } from '$lib/stores/auth';

	let { children } = $props();

	onMount(() => {
		authStore.initialize();
	});
</script>

<svelte:head>
	<title>Decent Cloud</title>
	<link rel="icon" href="/favicon.svg" />
	<meta name="description" content="Decentralized Cloud Computing Platform" />
</svelte:head>

{@render children()}
