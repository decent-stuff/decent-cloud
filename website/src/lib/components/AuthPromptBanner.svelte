<script lang="ts">
	import { page } from '$app/stores';
	import { navigateToLogin } from '$lib/utils/navigation';

	function handleAuth() {
		navigateToLogin($page.url.pathname);
	}
</script>

<div class="fixed top-0 left-0 md:left-64 right-0 z-50 bg-gradient-to-r from-blue-600 to-purple-600 p-3 md:p-4 shadow-lg">
	<div class="max-w-7xl mx-auto flex flex-col md:flex-row items-center justify-between gap-3">
		<p class="text-white text-sm md:text-base text-center md:text-left">
			Create an account to rent resources and manage your cloud infrastructure
		</p>
		<div class="flex gap-2 flex-shrink-0">
			<button
				onclick={handleAuth}
				class="px-4 py-2 bg-white hover:bg-gray-100 rounded-lg text-blue-600 text-sm font-medium transition-colors"
			>
				Login / Create Account
			</button>
		</div>
	</div>
</div>
