<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import { authStore } from '$lib/stores/auth';
	import ChatwootWidget from '$lib/components/ChatwootWidget.svelte';

	const CHATWOOT_TOKEN = import.meta.env.VITE_CHATWOOT_WEBSITE_TOKEN;
	const CHATWOOT_BASE_URL = import.meta.env.VITE_CHATWOOT_BASE_URL;

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

{#if CHATWOOT_TOKEN}
	<ChatwootWidget websiteToken={CHATWOOT_TOKEN} baseUrl={CHATWOOT_BASE_URL} />
{/if}
