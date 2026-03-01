<script lang="ts">
	import { page } from '$app/stores';
	import { navigateToLogin } from '$lib/utils/navigation';
	import { authCardVisible } from '$lib/stores/auth-card';
	import Icon from './Icons.svelte';
	import Button from './Button.svelte';

	function handleAuth() {
		navigateToLogin($page.url.pathname);
	}
</script>

<!-- Mobile: Compact login button in top-right -->
{#if !$authCardVisible}
<Button
	variant="sm"
	onclick={handleAuth}
	class="fixed top-4 right-4 z-50 md:hidden bg-primary-500 hover:bg-primary-400 text-neutral-900"
>
	Sign In
</Button>
{/if}

<!-- Desktop: Full banner -->
{#if !$authCardVisible}
<div class="hidden md:block fixed top-0 left-0 md:left-64 right-0 z-50 bg-primary-500 px-6 py-3">
	<div class="max-w-7xl mx-auto flex items-center justify-between gap-4">
		<p class="text-base text-sm">
			Create an account to rent resources and manage your cloud infrastructure
		</p>
		<Button variant="tertiary" onclick={handleAuth} class="inline-flex items-center gap-2 shrink-0 border border-base/50">
			<Icon name="login" size={20} />
			<span>Sign In</span>
		</Button>
	</div>
</div>
{/if}
