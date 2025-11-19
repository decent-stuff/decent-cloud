<script lang="ts">
	import type { AccountInfo } from "$lib/stores/auth";

	let { account } = $props<{ account: AccountInfo }>();

	let copiedField = $state<string | null>(null);

	function truncate(str: string, length: number = 16): string {
		if (str.length <= length) return str;
		return `${str.slice(0, length / 2)}...${str.slice(-length / 2)}`;
	}

	function formatDate(timestamp: number): string {
		const date = new Date(timestamp);
		return date.toLocaleDateString("en-US", {
			year: "numeric",
			month: "long",
			day: "numeric",
		});
	}

	async function copyToClipboard(text: string, field: string) {
		try {
			await navigator.clipboard.writeText(text);
			copiedField = field;
			setTimeout(() => {
				copiedField = null;
			}, 2000);
		} catch (err) {
			console.error("Failed to copy:", err);
		}
	}

	const activeKeysCount = $derived(
		account.publicKeys.filter((k: { isActive: boolean }) => k.isActive)
			.length,
	);
</script>

<div
	class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20"
>
	<h2 class="text-2xl font-semibold text-white mb-6">Account Overview</h2>

	<div class="grid grid-cols-1 md:grid-cols-2 gap-6">
		<!-- Username -->
		<div>
			<div class="text-sm text-white/60 mb-1">Username</div>
			<div class="flex items-center gap-2">
				<span class="text-xl font-semibold text-white"
					>@{account.username}</span
				>
				<button
					type="button"
					onclick={() => copyToClipboard(account.username, "username")}
					class="text-white/60 hover:text-white transition-colors"
					title="Copy username"
				>
					{#if copiedField === "username"}
						<span class="text-green-400">✓</span>
					{:else}
						📋
					{/if}
				</button>
			</div>
		</div>

		<!-- Account ID -->
		<div>
			<div class="text-sm text-white/60 mb-1">Account ID</div>
			<div class="flex items-center gap-2">
				<span class="text-sm font-mono text-white/80"
					>{truncate(account.id, 24)}</span
				>
				<button
					type="button"
					onclick={() => copyToClipboard(account.id, "accountId")}
					class="text-white/60 hover:text-white transition-colors"
					title="Copy account ID"
				>
					{#if copiedField === "accountId"}
						<span class="text-green-400">✓</span>
					{:else}
						📋
					{/if}
				</button>
			</div>
		</div>

		<!-- Created Date -->
		<div>
			<div class="text-sm text-white/60 mb-1">Created</div>
			<div class="text-white">{formatDate(account.createdAt)}</div>
		</div>

		<!-- Active Keys -->
		<div>
			<div class="text-sm text-white/60 mb-1">Active Keys</div>
			<div class="text-white font-semibold">
				{activeKeysCount}
				{activeKeysCount === 1 ? "key" : "keys"}
			</div>
		</div>
	</div>
</div>
