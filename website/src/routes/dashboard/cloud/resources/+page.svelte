<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { authStore } from "$lib/stores/auth";
	import { signRequest } from "$lib/services/auth-api";
	import {
		listCloudResources,
		listCloudAccounts,
		provisionCloudResource,
		deleteCloudResource,
		getCloudAccountCatalog,
		type CloudResourceWithDetails,
		type CloudAccount,
		type ServerType,
		type Location,
		type Image
	} from "$lib/services/api";
	import Icon from "$lib/components/Icons.svelte";
	import type { IdentityInfo } from "$lib/stores/auth";

	let resources = $state<CloudResourceWithDetails[]>([]);
	let accounts = $state<CloudAccount[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let currentIdentity = $state<IdentityInfo | null>(null);
	let unsubscribeAuth: (() => void) | null = null;

	let showProvisionModal = $state(false);
	let provisionAccountId = $state("");
	let provisionName = $state("");
	let provisionServerType = $state("");
	let provisionLocation = $state("");
	let provisionImage = $state("");
	let provisionSshPubkey = $state("");
	let provisioning = $state(false);
	let provisionError = $state<string | null>(null);

	let catalogLoading = $state(false);
	let serverTypes = $state<ServerType[]>([]);
	let locations = $state<Location[]>([]);
	let images = $state<Image[]>([]);

	let deleteResourceId = $state<string | null>(null);
	let deleting = $state(false);

	async function loadData() {
		if (!currentIdentity) return;
		loading = true;
		error = null;
		try {
			const { headers } = await signRequest(
				currentIdentity.identity,
				"GET",
				"/api/v1/cloud-resources"
			);
			const { headers: headers2 } = await signRequest(
				currentIdentity.identity,
				"GET",
				"/api/v1/cloud-accounts"
			);
			[resources, accounts] = await Promise.all([
				listCloudResources(headers),
				listCloudAccounts(headers2)
			]);
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load data";
		} finally {
			loading = false;
		}
	}

	async function loadCatalog(accountId: string) {
		if (!currentIdentity || !accountId) {
			serverTypes = [];
			locations = [];
			images = [];
			return;
		}
		catalogLoading = true;
		try {
			const { headers } = await signRequest(
				currentIdentity.identity,
				"GET",
				`/api/v1/cloud-accounts/${accountId}/catalog`
			);
			const catalog = await getCloudAccountCatalog(accountId, headers);
			serverTypes = catalog.serverTypes;
			locations = catalog.locations;
			images = catalog.images;
			if (serverTypes.length > 0 && !provisionServerType) {
				provisionServerType = serverTypes[0].id;
			}
			if (locations.length > 0 && !provisionLocation) {
				provisionLocation = locations[0].id;
			}
			if (images.length > 0 && !provisionImage) {
				provisionImage = images[0].id;
			}
		} catch (e) {
			provisionError = e instanceof Error ? e.message : "Failed to load catalog";
		} finally {
			catalogLoading = false;
		}
	}

	async function handleProvision() {
		if (!currentIdentity) return;
		if (!provisionAccountId || !provisionName.trim() || !provisionSshPubkey.trim()) {
			provisionError = "Please fill in all required fields";
			return;
		}

		provisioning = true;
		provisionError = null;
		try {
			const body = {
				cloudAccountId: provisionAccountId,
				name: provisionName.trim(),
				serverType: provisionServerType,
				location: provisionLocation,
				image: provisionImage,
				sshPubkey: provisionSshPubkey.trim()
			};
			const { headers } = await signRequest(
				currentIdentity.identity,
				"POST",
				"/api/v1/cloud-resources",
				body
			);
			await provisionCloudResource(body, headers);
			showProvisionModal = false;
			provisionName = "";
			provisionSshPubkey = "";
			await loadData();
		} catch (e) {
			provisionError = e instanceof Error ? e.message : "Failed to provision";
		} finally {
			provisioning = false;
		}
	}

	async function handleDeleteResource() {
		if (!deleteResourceId || !currentIdentity) return;
		deleting = true;
		try {
			const { headers } = await signRequest(
				currentIdentity.identity,
				"DELETE",
				`/api/v1/cloud-resources/${deleteResourceId}`
			);
			await deleteCloudResource(deleteResourceId, headers);
			deleteResourceId = null;
			await loadData();
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to delete resource";
		} finally {
			deleting = false;
		}
	}

	function closeProvisionModal() {
		showProvisionModal = false;
		provisionAccountId = "";
		provisionName = "";
		provisionServerType = "";
		provisionLocation = "";
		provisionImage = "";
		provisionSshPubkey = "";
		provisionError = null;
		serverTypes = [];
		locations = [];
		images = [];
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString();
	}

	function getStatusColor(status: string): string {
		switch (status.toLowerCase()) {
			case "running":
				return "bg-green-900/50 text-green-400";
			case "provisioning":
				return "bg-yellow-900/50 text-yellow-400";
			case "deleting":
				return "bg-orange-900/50 text-orange-400";
			case "failed":
				return "bg-red-900/50 text-red-400";
			case "deleted":
				return "bg-neutral-700 text-neutral-400";
			default:
				return "bg-neutral-700 text-neutral-300";
		}
	}

	let validAccounts = $derived(accounts.filter((a) => a.isValid));

	onMount(() => {
		unsubscribeAuth = authStore.currentIdentity.subscribe((value) => {
			currentIdentity = value;
			if (value) {
				loadData();
			}
		});
	});

	onDestroy(() => {
		unsubscribeAuth?.();
	});
</script>

<div class="p-6">
	<div class="max-w-6xl mx-auto">
		<div class="flex items-center justify-between mb-6">
			<div>
				<h1 class="text-2xl font-bold text-white">Cloud Resources</h1>
				<p class="text-neutral-400 mt-1">
					Self-provisioned VMs on your connected cloud accounts
				</p>
			</div>
			<button
				type="button"
				onclick={() => (showProvisionModal = true)}
				disabled={validAccounts.length === 0}
				class="flex items-center gap-2 px-4 py-2 bg-primary-500 text-neutral-900 font-semibold hover:bg-primary-400 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
			>
				<Icon name="plus" size={18} />
				Provision VM
			</button>
		</div>

		{#if validAccounts.length === 0 && !loading}
			<div class="p-4 bg-yellow-900/30 border border-yellow-700 text-yellow-300 mb-6">
				You need to add a valid cloud account before you can provision resources.
				<a href="/dashboard/cloud/accounts" class="underline hover:no-underline">Add a cloud account</a>
			</div>
		{/if}

		{#if loading}
			<div class="flex items-center justify-center py-12">
				<div class="text-neutral-400">Loading...</div>
			</div>
		{:else if error}
			<div class="p-4 bg-red-900/30 border border-red-700 text-red-300">
				{error}
			</div>
		{:else if resources.length === 0}
			<div class="text-center py-12">
				<div class="text-neutral-400 mb-4">No cloud resources provisioned</div>
				{#if validAccounts.length > 0}
					<button
						type="button"
						onclick={() => (showProvisionModal = true)}
						class="text-primary-400 hover:text-primary-300"
					>
						Provision your first VM
					</button>
				{/if}
			</div>
		{:else}
			<div class="grid gap-4">
				{#each resources as resource}
					<div class="bg-surface border border-neutral-800 p-4">
						<div class="flex items-start justify-between">
							<div class="flex-1">
								<div class="flex items-center gap-2">
									<h3 class="font-medium text-white">{resource.name}</h3>
									<span class="px-2 py-0.5 text-xs {getStatusColor(resource.status)}">
										{resource.status}
									</span>
								</div>
								<div class="text-sm text-neutral-400 mt-1">
									{resource.cloudAccountName} &middot; {resource.serverType} &middot; {resource.location}
								</div>
								{#if resource.publicIp}
									<div class="mt-3 p-3 bg-neutral-800/50 text-sm">
										<div class="text-neutral-300 font-mono">
											ssh -p {resource.sshPort} {resource.sshUsername}@{resource.publicIp}
										</div>
										{#if resource.gatewaySlug}
											<div class="text-neutral-400 mt-1 text-xs">
												Gateway: {resource.gatewaySlug}.gw.decent-cloud.org:{resource.gatewaySshPort}
											</div>
										{/if}
									</div>
								{/if}
							</div>
							<div class="flex items-center gap-2">
								{#if resource.status === 'provisioning'}
									<span class="text-xs text-yellow-400">Setting up...</span>
								{:else if resource.status === 'running'}
									<button
										type="button"
										onclick={() => (deleteResourceId = resource.id)}
										class="p-2 text-neutral-400 hover:text-red-400 transition-colors"
										title="Terminate"
									>
										<Icon name="trash" size={18} />
									</button>
								{:else if resource.status === 'deleting'}
									<span class="text-xs text-orange-400">Terminating...</span>
								{/if}
							</div>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>

{#if showProvisionModal}
	<div class="fixed inset-0 bg-black/70 flex items-center justify-center z-50 overflow-y-auto py-8" role="dialog" aria-modal="true">
		<div class="bg-surface border border-neutral-700 w-full max-w-lg mx-4 my-auto">
			<div class="p-4 border-b border-neutral-700">
				<h2 class="text-lg font-semibold text-white">Provision New VM</h2>
			</div>
			<form onsubmit={(e) => { e.preventDefault(); handleProvision(); }} class="p-4 space-y-4">
				{#if provisionError}
					<div class="p-3 bg-red-900/30 border border-red-700 text-red-300 text-sm">
						{provisionError}
					</div>
				{/if}
				<div>
					<label for="cloudAccount" class="block text-sm font-medium text-neutral-300 mb-1">Cloud Account</label>
					<select
						id="cloudAccount"
						bind:value={provisionAccountId}
						onchange={() => loadCatalog(provisionAccountId)}
						class="w-full px-3 py-2 bg-neutral-800 border border-neutral-700 text-white"
					>
						<option value="">Select account...</option>
						{#each validAccounts as account}
							<option value={account.id}>{account.name} ({account.backendType})</option>
						{/each}
					</select>
				</div>
				{#if catalogLoading}
					<div class="text-neutral-400 text-sm">Loading catalog...</div>
				{:else if provisionAccountId && serverTypes.length > 0}
					<div>
						<label for="serverType" class="block text-sm font-medium text-neutral-300 mb-1">Server Type</label>
						<select id="serverType" bind:value={provisionServerType} class="w-full px-3 py-2 bg-neutral-800 border border-neutral-700 text-white">
							{#each serverTypes as st}
								<option value={st.id}>{st.name} ({st.cores} vCPU, {st.memoryGb}GB RAM, {st.diskGb}GB disk)</option>
							{/each}
						</select>
					</div>
					<div>
						<label for="location" class="block text-sm font-medium text-neutral-300 mb-1">Location</label>
						<select id="location" bind:value={provisionLocation} class="w-full px-3 py-2 bg-neutral-800 border border-neutral-700 text-white">
							{#each locations as loc}
								<option value={loc.id}>{loc.name} ({loc.city}, {loc.country})</option>
							{/each}
						</select>
					</div>
					<div>
						<label for="image" class="block text-sm font-medium text-neutral-300 mb-1">Image</label>
						<select id="image" bind:value={provisionImage} class="w-full px-3 py-2 bg-neutral-800 border border-neutral-700 text-white">
							{#each images as img}
								<option value={img.id}>{img.name}</option>
							{/each}
						</select>
					</div>
				{/if}
				<div>
					<label for="vmName" class="block text-sm font-medium text-neutral-300 mb-1">VM Name</label>
					<input
						id="vmName"
						type="text"
						bind:value={provisionName}
						placeholder="my-server"
						class="w-full px-3 py-2 bg-neutral-800 border border-neutral-700 text-white placeholder-neutral-500"
					/>
				</div>
				<div>
					<label for="sshPubkey" class="block text-sm font-medium text-neutral-300 mb-1">SSH Public Key</label>
					<textarea
						id="sshPubkey"
						bind:value={provisionSshPubkey}
						placeholder="ssh-ed25519 AAAA..."
						rows="2"
						class="w-full px-3 py-2 bg-neutral-800 border border-neutral-700 text-white placeholder-neutral-500 font-mono text-sm"
					></textarea>
					<p class="text-xs text-neutral-500 mt-1">This key will be added to root's authorized_keys</p>
				</div>
				<div class="flex justify-end gap-3 pt-2">
					<button type="button" onclick={closeProvisionModal} class="px-4 py-2 text-neutral-400 hover:text-white">
						Cancel
					</button>
					<button
						type="submit"
						disabled={provisioning || !provisionAccountId || catalogLoading}
						class="px-4 py-2 bg-primary-500 text-neutral-900 font-semibold hover:bg-primary-400 disabled:opacity-50"
					>
						{provisioning ? "Provisioning..." : "Provision"}
					</button>
				</div>
			</form>
		</div>
	</div>
{/if}

{#if deleteResourceId}
	<div class="fixed inset-0 bg-black/70 flex items-center justify-center z-50" role="dialog" aria-modal="true">
		<div class="bg-surface border border-neutral-700 w-full max-w-sm mx-4 p-4">
			<h3 class="text-lg font-semibold text-white mb-2">Terminate VM?</h3>
			<p class="text-neutral-400 text-sm mb-4">
				This will permanently delete the VM and all its data. This action cannot be undone.
			</p>
			<div class="flex justify-end gap-3">
				<button
					type="button"
					onclick={() => (deleteResourceId = null)}
					class="px-4 py-2 text-neutral-400 hover:text-white"
				>
					Cancel
				</button>
				<button
					type="button"
					onclick={handleDeleteResource}
					disabled={deleting}
					class="px-4 py-2 bg-red-600 text-white font-semibold hover:bg-red-500 disabled:opacity-50"
				>
					{deleting ? "Terminating..." : "Terminate"}
				</button>
			</div>
		</div>
	</div>
{/if}
