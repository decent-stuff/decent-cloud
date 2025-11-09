<script lang="ts">
	import '../app.css';

	let { children } = $props();
</script>

<svelte:head>
	<title>Decent Cloud</title>
	<link rel="icon" href="/favicon.svg" />
	<meta name="description" content="Decentralized Cloud Computing Platform" />
</svelte:head>

{@render children()}
