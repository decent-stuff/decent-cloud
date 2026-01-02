<script lang="ts">
	import { onMount } from "svelte";
	import { API_BASE_URL } from "$lib/services/api";
	import {
		handleApiResponse,
		type UserApiClient,
	} from "$lib/services/user-api";

	interface ExternalKey {
		id: number;
		keyType: string;
		keyData: string;
		keyFingerprint: string | null;
		label: string | null;
	}

	interface Props {
		username: string;
		apiClient: UserApiClient;
	}

	let { username, apiClient }: Props = $props();

	let keys = $state<ExternalKey[]>([]);
	let newKey = $state({
		type: "ssh-ed25519",
		data: "",
		fingerprint: "",
		label: "",
	});
	let loading = $state(false);
	let error = $state<string | null>(null);
	let successMessage = $state<string | null>(null);

	onMount(() => {
		loadKeys();
	});

	async function loadKeys() {
		try {
			const res = await fetch(
				`${API_BASE_URL}/api/v1/accounts/${username}/external-keys`,
			);
			if (res.ok) {
				const data = await res.json();
				if (data.success && data.data) {
					keys = data.data;
				}
			}
		} catch (err) {
			console.error("Failed to load external keys:", err);
		}
	}

	async function handleAdd() {
		if (!newKey.data.trim()) return;

		loading = true;
		error = null;
		successMessage = null;

		try {
			const res = await apiClient.addExternalKey(username, {
				keyType: newKey.type,
				keyData: newKey.data,
				keyFingerprint: newKey.fingerprint || undefined,
				label: newKey.label || undefined,
			});

			if (!res.ok) {
				await handleApiResponse(res);
				return;
			}

			const data = await res.json();
			if (!data.success) {
				throw new Error(data.error || "Failed to add external key");
			}

			newKey = {
				type: "ssh-ed25519",
				data: "",
				fingerprint: "",
				label: "",
			};
			await loadKeys();
			successMessage = "External key added successfully!";
			setTimeout(() => {
				successMessage = null;
			}, 3000);
		} catch (err: unknown) {
			error =
				err instanceof Error
					? err.message
					: "Failed to add external key";
		} finally {
			loading = false;
		}
	}

	async function handleDelete(keyId: number) {
		if (!confirm("Delete this external key?")) return;

		error = null;
		successMessage = null;

		try {
			const res = await apiClient.deleteExternalKey(username, keyId);

			if (!res.ok) {
				await handleApiResponse(res);
				return;
			}

			const data = await res.json();
			if (!data.success) {
				throw new Error(data.error || "Failed to delete external key");
			}
			await loadKeys();
			successMessage = "External key deleted successfully!";
			setTimeout(() => {
				successMessage = null;
			}, 3000);
		} catch (err: unknown) {
			error =
				err instanceof Error
					? err.message
					: "Failed to delete external key";
		}
	}
</script>

<div class="card p-6 border border-neutral-800">
	<h2 class="text-2xl font-bold text-white mb-4">External Keys (SSH/GPG)</h2>

	<!-- Keys list -->
	<div class="space-y-2 mb-4">
		{#if keys.length === 0}
			<p class="text-neutral-500 text-sm">No external keys added yet.</p>
		{/if}
		{#each keys as key}
			<div class="p-3 bg-surface-elevated  border border-neutral-800">
				<div class="flex items-start justify-between mb-2">
					<div>
						<span class="font-medium text-sm text-white"
							>{key.keyType}</span
						>
						{#if key.label}
							<span
								class="ml-2 text-xs bg-primary-500/20 text-primary-400 px-2 py-1 rounded border border-primary-500/30"
							>
								{key.label}
							</span>
						{/if}
					</div>
					<button
						onclick={() => handleDelete(key.id)}
						class="text-red-400 hover:text-red-300 transition-colors text-sm"
					>
						Delete
					</button>
				</div>
				<div class="text-xs text-neutral-500 font-mono break-all">
					{key.keyData.substring(0, 80)}{key.keyData.length > 80
						? "..."
						: ""}
				</div>
				{#if key.keyFingerprint}
					<div class="text-xs text-neutral-500 mt-1">
						Fingerprint: {key.keyFingerprint}
					</div>
				{/if}
			</div>
		{/each}
	</div>

	<!-- Add new key -->
	<div class="space-y-2">
		<div class="flex gap-2">
			<select
				bind:value={newKey.type}
				class="px-3 py-2 bg-surface-elevated border border-neutral-800  text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
			>
				<option value="ssh-ed25519">SSH Ed25519</option>
				<option value="ssh-rsa">SSH RSA</option>
				<option value="gpg">GPG</option>
			</select>
			<input
				type="text"
				bind:value={newKey.label}
				class="flex-1 px-3 py-2 bg-surface-elevated border border-neutral-800  text-white placeholder-white/40 focus:ring-2 focus:ring-primary-500 focus:border-transparent"
				placeholder="Label (optional)"
			/>
		</div>
		<textarea
			bind:value={newKey.data}
			class="w-full px-3 py-2 bg-surface-elevated border border-neutral-800  text-white placeholder-white/40 focus:ring-2 focus:ring-primary-500 focus:border-transparent font-mono text-sm"
			rows={3}
			placeholder="Paste your public key here"
		></textarea>
		<div class="flex gap-2">
			<input
				type="text"
				bind:value={newKey.fingerprint}
				class="flex-1 px-3 py-2 bg-surface-elevated border border-neutral-800  text-white placeholder-white/40 focus:ring-2 focus:ring-primary-500 focus:border-transparent"
				placeholder="Fingerprint (optional)"
			/>
			<button
				onclick={handleAdd}
				disabled={!newKey.data.trim() || loading}
				class="px-4 py-2 bg-primary-600 text-white  hover:bg-primary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
			>
				Add
			</button>
		</div>
	</div>

	{#if error}
		<div
			class="mt-4 p-3 bg-red-500/20 border border-red-500/30 rounded text-red-400"
		>
			{error}
		</div>
	{/if}

	{#if successMessage}
		<div
			class="mt-4 p-3 bg-green-500/20 border border-green-500/30 rounded text-green-400"
		>
			{successMessage}
		</div>
	{/if}
</div>
