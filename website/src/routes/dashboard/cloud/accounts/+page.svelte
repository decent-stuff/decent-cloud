<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { authStore } from "$lib/stores/auth";
	import { signRequest } from "$lib/services/auth-api";
	import {
		listCloudAccounts,
		addCloudAccount,
		deleteCloudAccount,
		type CloudAccount
	} from "$lib/services/api";
	import Icon from "$lib/components/Icons.svelte";
	import type { IdentityInfo } from "$lib/stores/auth";

	let accounts = $state<CloudAccount[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let currentIdentity = $state<IdentityInfo | null>(null);
	let unsubscribeAuth: (() => void) | null = null;

	let showAddModal = $state(false);
	let addBackendType = $state("hetzner");
	let addName = $state("");
	let addCredentials = $state("");
	let adding = $state(false);
	let addError = $state<string | null>(null);

	let deleteAccountId = $state<string | null>(null);
	let deleting = $state(false);

	async function loadAccounts() {
		if (!currentIdentity) return;
		loading = true;
		error = null;
		try {
			const { headers } = await signRequest(
				currentIdentity.identity,
				"GET",
				"/api/v1/cloud-accounts"
			);
			accounts = await listCloudAccounts(headers);
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load cloud accounts";
		} finally {
			loading = false;
		}
	}

	async function handleAddAccount() {
		if (!currentIdentity) return;
		if (!addName.trim() || !addCredentials.trim()) {
			addError = "Please fill in all required fields";
			return;
		}

		adding = true;
		addError = null;
		try {
			const body = {
				backendType: addBackendType,
				name: addName.trim(),
				credentials: addCredentials.trim()
			};
			const { headers } = await signRequest(
				currentIdentity.identity,
				"POST",
				"/api/v1/cloud-accounts",
				body
			);
			await addCloudAccount(body, headers);
			showAddModal = false;
			addName = "";
			addCredentials = "";
			await loadAccounts();
		} catch (e) {
			addError = e instanceof Error ? e.message : "Failed to add account";
		} finally {
			adding = false;
		}
	}

	async function handleDeleteAccount() {
		if (!deleteAccountId || !currentIdentity) return;
		deleting = true;
		try {
			const { headers } = await signRequest(
				currentIdentity.identity,
				"DELETE",
				`/api/v1/cloud-accounts/${deleteAccountId}`
			);
			await deleteCloudAccount(deleteAccountId, headers);
			deleteAccountId = null;
			await loadAccounts();
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to delete account";
		} finally {
			deleting = false;
		}
	}

	function closeModal() {
		showAddModal = false;
		addName = "";
		addCredentials = "";
		addError = null;
	}

	onMount(() => {
		unsubscribeAuth = authStore.currentIdentity.subscribe((value) => {
			currentIdentity = value;
			if (value) {
				loadAccounts();
			}
		});
	});

	onDestroy(() => {
		unsubscribeAuth?.();
	});

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString();
	}

	function getBackendLabel(type: string): string {
		switch (type) {
			case "hetzner":
				return "Hetzner Cloud";
			case "proxmox_api":
				return "Proxmox VE";
			default:
				return type;
		}
	}
</script>

<div class="p-6">
	<div class="max-w-6xl mx-auto">
		<div class="flex items-center justify-between mb-6">
			<div>
				<h1 class="text-2xl font-bold text-white">Cloud Accounts</h1>
				<p class="text-neutral-400 mt-1">
					Connect your Hetzner or Proxmox accounts for self-provisioning
				</p>
			</div>
			<button
				type="button"
				onclick={() => (showAddModal = true)}
				class="flex items-center gap-2 px-4 py-2 bg-primary-500 text-neutral-900 font-semibold hover:bg-primary-400 transition-colors"
			>
				<Icon name="plus" size={18} />
				Add Account
			</button>
		</div>

		{#if loading}
			<div class="flex items-center justify-center py-12">
				<div class="text-neutral-400">Loading...</div>
			</div>
		{:else if error}
			<div class="p-4 bg-red-900/30 border border-red-700 text-red-300">
				{error}
			</div>
		{:else if accounts.length === 0}
			<div class="text-center py-12">
				<div class="text-neutral-400 mb-4">No cloud accounts connected</div>
				<button
					type="button"
					onclick={() => (showAddModal = true)}
					class="text-primary-400 hover:text-primary-300"
				>
					Add your first cloud account
				</button>
			</div>
		{:else}
			<div class="grid gap-4">
				{#each accounts as account}
					<div class="bg-surface border border-neutral-800 p-4">
						<div class="flex items-start justify-between">
							<div>
								<div class="flex items-center gap-2">
									<h3 class="font-medium text-white">{account.name}</h3>
									{#if account.isValid}
										<span class="px-2 py-0.5 text-xs bg-green-900/50 text-green-400">Valid</span>
									{:else}
										<span class="px-2 py-0.5 text-xs bg-red-900/50 text-red-400">Invalid</span>
									{/if}
								</div>
								<div class="text-sm text-neutral-400 mt-1">
									{getBackendLabel(account.backendType)} &middot; Added {formatDate(account.createdAt)}
								</div>
								{#if account.validationError}
									<div class="text-sm text-red-400 mt-2">
										{account.validationError}
									</div>
								{/if}
							</div>
							<button
								type="button"
								onclick={() => (deleteAccountId = account.id)}
								class="p-2 text-neutral-400 hover:text-red-400 transition-colors"
								title="Delete account"
							>
								<Icon name="trash" size={18} />
							</button>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>

{#if showAddModal}
	<div class="fixed inset-0 bg-black/70 flex items-center justify-center z-50" role="dialog" aria-modal="true">
		<div class="bg-surface border border-neutral-700 w-full max-w-md mx-4">
			<div class="p-4 border-b border-neutral-700">
				<h2 class="text-lg font-semibold text-white">Add Cloud Account</h2>
			</div>
			<form onsubmit={(e) => { e.preventDefault(); handleAddAccount(); }} class="p-4 space-y-4">
				{#if addError}
					<div class="p-3 bg-red-900/30 border border-red-700 text-red-300 text-sm">
						{addError}
					</div>
				{/if}
				<div>
					<label for="backendType" class="block text-sm font-medium text-neutral-300 mb-1">Provider</label>
					<select id="backendType" bind:value={addBackendType} class="w-full px-3 py-2 bg-neutral-800 border border-neutral-700 text-white">
						<option value="hetzner">Hetzner Cloud</option>
						<option value="proxmox_api">Proxmox VE</option>
					</select>
				</div>
				<div>
					<label for="accountName" class="block text-sm font-medium text-neutral-300 mb-1">Account Name</label>
					<input
						id="accountName"
						type="text"
						bind:value={addName}
						placeholder="My Hetzner Account"
						class="w-full px-3 py-2 bg-neutral-800 border border-neutral-700 text-white placeholder-neutral-500"
					/>
				</div>
				<div>
					<label for="credentials" class="block text-sm font-medium text-neutral-300 mb-1">
						{#if addBackendType === 'hetzner'}
							API Token
						{:else}
							API Credentials (JSON)
						{/if}
					</label>
					{#if addBackendType === 'hetzner'}
						<input
							id="credentials"
							type="password"
							bind:value={addCredentials}
							placeholder="hcloud_xxx..."
							class="w-full px-3 py-2 bg-neutral-800 border border-neutral-700 text-white placeholder-neutral-500"
						/>
						<p class="text-xs text-neutral-500 mt-1">
							Generate at <a href="https://console.hetzner.cloud/" target="_blank" rel="noopener" class="text-primary-400 hover:underline">Hetzner Console</a> - Security - API Tokens
						</p>
					{:else}
						<textarea
							id="credentials"
							bind:value={addCredentials}
							rows="3"
							class="w-full px-3 py-2 bg-neutral-800 border border-neutral-700 text-white placeholder-neutral-500 font-mono text-sm"
						></textarea>
						<p class="text-xs text-neutral-500 mt-1">JSON with url, token, and optional node fields</p>
					{/if}
				</div>
				<div class="flex justify-end gap-3 pt-2">
					<button type="button" onclick={closeModal} class="px-4 py-2 text-neutral-400 hover:text-white">
						Cancel
					</button>
					<button
						type="submit"
						disabled={adding}
						class="px-4 py-2 bg-primary-500 text-neutral-900 font-semibold hover:bg-primary-400 disabled:opacity-50"
					>
						{adding ? "Adding..." : "Add Account"}
					</button>
				</div>
			</form>
		</div>
	</div>
{/if}

{#if deleteAccountId}
	<div class="fixed inset-0 bg-black/70 flex items-center justify-center z-50" role="dialog" aria-modal="true">
		<div class="bg-surface border border-neutral-700 w-full max-w-sm mx-4 p-4">
			<h3 class="text-lg font-semibold text-white mb-2">Delete Cloud Account?</h3>
			<p class="text-neutral-400 text-sm mb-4">
				This will remove the account connection. Existing resources will not be affected.
			</p>
			<div class="flex justify-end gap-3">
				<button
					type="button"
					onclick={() => (deleteAccountId = null)}
					class="px-4 py-2 text-neutral-400 hover:text-white"
				>
					Cancel
				</button>
				<button
					type="button"
					onclick={handleDeleteAccount}
					disabled={deleting}
					class="px-4 py-2 bg-red-600 text-white font-semibold hover:bg-red-500 disabled:opacity-50"
				>
					{deleting ? "Deleting..." : "Delete"}
				</button>
			</div>
		</div>
	</div>
{/if}
