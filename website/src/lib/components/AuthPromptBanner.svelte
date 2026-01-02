<script lang="ts">
	import { page } from '$app/stores';
	import { navigateToLogin } from '$lib/utils/navigation';
	import Icon from './Icons.svelte';

	function handleAuth() {
		navigateToLogin($page.url.pathname);
	}
</script>

<!-- Mobile: Compact login button in top-right -->
<button
	onclick={handleAuth}
	class="fixed top-4 right-4 z-50 md:hidden px-3 py-1.5 bg-primary-500 hover:bg-primary-400 text-base text-xs font-medium transition-colors"
>
	Sign In
</button>

<!-- Desktop: Full banner -->
<div class="hidden md:block fixed top-0 left-0 md:left-64 right-0 z-50 bg-primary-500 px-6 py-3">
	<div class="max-w-7xl mx-auto flex items-center justify-between gap-4">
		<p class="text-base text-sm">
			Create an account to rent resources and manage your cloud infrastructure
		</p>
		<button
			onclick={handleAuth}
			class="inline-flex items-center gap-2 px-4 py-2 bg-base hover:bg-surface text-white text-sm font-medium transition-colors shrink-0"
		>
			<Icon name="login" size={16} />
			<span>Sign In</span>
		</button>
	</div>
</div>
