<script lang="ts">
	import type { AccountInfo, IdentityInfo } from "$lib/stores/auth";
	import { authStore } from "$lib/stores/auth";
	import { updateDeviceName } from "$lib/services/account-api";
	import type { Ed25519KeyIdentity } from "@dfinity/identity";

	let { account } = $props<{ account: AccountInfo }>();

	let copiedField = $state<string | null>(null);
	let editingKeyId = $state<string | null>(null);
	let editingName = $state("");
	let saveError = $state<string | null>(null);
	let saving = $state(false);
	let currentIdentity = $state<IdentityInfo | null>(null);

	authStore.currentIdentity.subscribe((value) => {
		currentIdentity = value;
	});

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

	function getDeviceName(key: { deviceName?: string }): string {
		return key.deviceName || "Unnamed Device";
	}

	function startEdit(key: { id: string; deviceName?: string }) {
		editingKeyId = key.id;
		editingName = key.deviceName || "";
		saveError = null;
	}

	function cancelEdit() {
		editingKeyId = null;
		editingName = "";
		saveError = null;
	}

	async function saveDeviceName(keyId: string) {
		if (!currentIdentity?.identity) {
			saveError = "No signing identity";
			return;
		}

		saving = true;
		saveError = null;

		try {
			await updateDeviceName(
				currentIdentity.identity as Ed25519KeyIdentity,
				account.username,
				keyId,
				editingName.trim(),
			);
			// Reload account to get updated data
			await authStore.loadAccountByUsername(account.username);
			editingKeyId = null;
		} catch (err) {
			saveError =
				err instanceof Error
					? err.message
					: "Failed to update device name";
		} finally {
			saving = false;
		}
	}
</script>

<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
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
					onclick={() =>
						copyToClipboard(account.username, "username")}
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

	<!-- Device Keys Section -->
	{#if account.publicKeys.length > 0}
		<div class="mt-6 pt-6 border-t border-white/10">
			<h3 class="text-lg font-semibold text-white mb-4">Devices</h3>
			<div class="space-y-3">
				{#each account.publicKeys as key}
					<div
						class="flex items-center justify-between p-3 bg-white/5 rounded-lg"
					>
						<div class="flex items-center gap-3">
							<span class="text-xl"
								>{key.isActive ? "🔑" : "🔒"}</span
							>
							<div class="flex-1">
								{#if editingKeyId === key.id}
									<div class="flex items-center gap-2">
										<input
											type="text"
											bind:value={editingName}
											placeholder="Device name"
											class="px-2 py-1 bg-white/10 border border-white/20 rounded text-white text-sm w-40"
											disabled={saving}
										/>
										<button
											type="button"
											onclick={() =>
												saveDeviceName(key.id)}
											disabled={saving}
											class="px-2 py-1 bg-green-600 hover:bg-green-500 text-white text-xs rounded disabled:opacity-50"
										>
											{saving ? "..." : "Save"}
										</button>
										<button
											type="button"
											onclick={cancelEdit}
											disabled={saving}
											class="px-2 py-1 bg-white/10 hover:bg-white/20 text-white text-xs rounded disabled:opacity-50"
										>
											Cancel
										</button>
									</div>
									{#if saveError}
										<div class="text-red-400 text-xs mt-1">
											{saveError}
										</div>
									{/if}
								{:else}
									<button
										type="button"
										onclick={() => startEdit(key)}
										class="text-white font-medium hover:text-purple-300 transition-colors text-left"
										title="Click to edit device name"
									>
										{getDeviceName(key)}
									</button>
								{/if}
								<div class="text-xs text-white/50 font-mono">
									{truncate(key.publicKey, 20)}
								</div>
							</div>
						</div>
						<div class="flex items-center gap-2">
							{#if key.isActive}
								<span
									class="px-2 py-1 bg-green-500/20 text-green-400 text-xs rounded"
									>Active</span
								>
							{:else}
								<span
									class="px-2 py-1 bg-red-500/20 text-red-400 text-xs rounded"
									>Disabled</span
								>
							{/if}
						</div>
					</div>
				{/each}
			</div>
		</div>
	{/if}
</div>
