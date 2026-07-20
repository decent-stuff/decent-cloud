<script lang="ts">
	import { page } from '$app/stores';

	// Friendly labels for common HTTP status codes the app surfaces.
	function statusLabel(status: number): string {
		if (status === 404) return 'Page not found';
		if (status === 403) return 'Access denied';
		if (status >= 500) return 'Something went wrong';
		if (status === 401) return 'Sign in required';
		return 'Unexpected error';
	}
</script>

<svelte:head>
	<title>{statusLabel($page.status)} - Decent Cloud</title>
</svelte:head>

<div class="min-h-screen bg-base flex items-center justify-center p-4">
	<div class="w-full max-w-lg text-center">
		<a href="/" class="inline-block mb-8">
			<h1 class="text-2xl font-bold text-white tracking-tight">Decent Cloud</h1>
		</a>

		<div class="card p-8 border border-neutral-800 space-y-4">
			<p class="text-5xl font-bold text-primary-400 font-mono">{$page.status}</p>
			<h2 class="text-xl font-semibold text-white">{statusLabel($page.status)}</h2>
			{#if $page.error?.message && $page.status >= 500}
				<p class="text-neutral-400 text-sm">{$page.error.message}</p>
			{:else}
				<p class="text-neutral-400 text-sm">
					The page you're looking for doesn't exist or has been moved.
				</p>
			{/if}

			<div class="flex flex-col sm:flex-row gap-3 justify-center pt-2">
				<a
					href="/"
					class="px-5 py-2.5 bg-primary-500 hover:bg-primary-400 text-neutral-900 font-semibold transition-colors"
				>
					Back to home
				</a>
				<a
					href="/dashboard/marketplace"
					class="px-5 py-2.5 border border-neutral-700 bg-surface-elevated hover:border-neutral-500 text-white font-medium transition-colors"
				>
					Browse marketplace
				</a>
			</div>
		</div>
	</div>
</div>
