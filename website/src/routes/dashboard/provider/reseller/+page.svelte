<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { page } from "$app/stores";
	import { navigateToLogin } from "$lib/utils/navigation";
	import {
		getExternalProviders,
		getResellerRelationships,
		createResellerRelationship,
		updateResellerRelationship,
		deleteResellerRelationship,
		hexEncode,
		type ExternalProvider,
		type ResellerRelationship,
		type CreateResellerRelationshipParams,
		type UpdateResellerRelationshipParams,
	} from "$lib/services/api";
	import { signRequest } from "$lib/services/auth-api";
	import { authStore } from "$lib/stores/auth";
	import { Ed25519KeyIdentity } from "@dfinity/identity";

	let externalProviders = $state<ExternalProvider[]>([]);
	let relationships = $state<ResellerRelationship[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let successMessage = $state<string | null>(null);
	let isAuthenticated = $state(false);
	let unsubscribeAuth: (() => void) | null = null;
	let providerHex = $state("");

	// Track which provider is being added as reseller
	let creatingRelationship = $state<Record<string, boolean>>({});
	let commissionInputs = $state<Record<string, number>>({});

	// Track editing state
	let editingRelationship = $state<Record<string, boolean>>({});
	let editCommissionInputs = $state<Record<string, number>>({});
	let deletingRelationship = $state<Record<string, boolean>>({});

	type SigningIdentity = {
		identity: Ed25519KeyIdentity;
		publicKeyBytes: Uint8Array;
	};

	let signingIdentityInfo = $state<SigningIdentity | null>(null);

	onMount(() => {
		unsubscribeAuth = authStore.isAuthenticated.subscribe((isAuth) => {
			isAuthenticated = isAuth;
			if (isAuth) {
				loadData();
			} else {
				loading = false;
			}
		});
	});

	async function loadData() {
		if (!isAuthenticated) {
			loading = false;
			return;
		}

		try {
			loading = true;
			error = null;
			const info = await authStore.getSigningIdentity();
			if (!info) {
				error = "You must be authenticated to manage reseller relationships";
				return;
			}
			if (!(info.identity instanceof Ed25519KeyIdentity)) {
				error = "Only Ed25519 identities can sign reseller actions";
				return;
			}
			const normalizedIdentity: SigningIdentity = {
				identity: info.identity,
				publicKeyBytes: info.publicKeyBytes,
			};
			signingIdentityInfo = normalizedIdentity;
			providerHex = hexEncode(normalizedIdentity.publicKeyBytes);

			// Fetch external providers (no auth required - public endpoint)
			externalProviders = await getExternalProviders();

			// Fetch current relationships (requires auth)
			const relationshipsSigned = await signRequest(
				normalizedIdentity.identity,
				"GET",
				"/api/v1/reseller/relationships",
			);
			relationships = await getResellerRelationships(relationshipsSigned.headers);
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load reseller data";
		} finally {
			loading = false;
		}
	}

	function handleLogin() {
		navigateToLogin($page.url.pathname);
	}

	function getCommissionValue(pubkey: string): number {
		return commissionInputs[pubkey] ?? 10; // Default 10%
	}

	function getEditCommissionValue(pubkey: string): number {
		return editCommissionInputs[pubkey] ?? 10;
	}

	function isAlreadyReseller(providerPubkey: string): boolean {
		return relationships.some(
			(r) => r.external_provider_pubkey === providerPubkey && r.status === "active"
		);
	}

	async function handleBecomeReseller(provider: ExternalProvider) {
		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) {
			error = "Missing signing identity";
			return;
		}

		const commission = getCommissionValue(provider.pubkey);
		if (commission < 0 || commission > 50) {
			error = "Commission must be between 0 and 50%";
			return;
		}

		error = null;
		successMessage = null;
		creatingRelationship = { ...creatingRelationship, [provider.pubkey]: true };

		try {
			const payload: CreateResellerRelationshipParams = {
				external_provider_pubkey: provider.pubkey,
				commission_percent: commission,
			};

			const signed = await signRequest(
				activeIdentity.identity,
				"POST",
				"/api/v1/reseller/relationships",
				payload,
			);

			await createResellerRelationship(payload, signed.headers);
			successMessage = `Successfully became reseller for ${provider.name} with ${commission}% commission`;
			await loadData();
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to create reseller relationship";
		} finally {
			creatingRelationship = { ...creatingRelationship, [provider.pubkey]: false };
		}
	}

	function startEdit(relationship: ResellerRelationship) {
		editingRelationship = { ...editingRelationship, [relationship.external_provider_pubkey]: true };
		editCommissionInputs = { ...editCommissionInputs, [relationship.external_provider_pubkey]: relationship.commission_percent };
	}

	function cancelEdit(providerPubkey: string) {
		editingRelationship = { ...editingRelationship, [providerPubkey]: false };
	}

	async function handleUpdateRelationship(relationship: ResellerRelationship) {
		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) {
			error = "Missing signing identity";
			return;
		}

		const newCommission = getEditCommissionValue(relationship.external_provider_pubkey);
		if (newCommission < 0 || newCommission > 50) {
			error = "Commission must be between 0 and 50%";
			return;
		}

		error = null;
		successMessage = null;

		try {
			const payload: UpdateResellerRelationshipParams = {
				commission_percent: newCommission,
			};

			const signed = await signRequest(
				activeIdentity.identity,
				"PUT",
				`/api/v1/reseller/relationships/${relationship.external_provider_pubkey}`,
				payload,
			);

			await updateResellerRelationship(relationship.external_provider_pubkey, payload, signed.headers);
			successMessage = `Updated commission to ${newCommission}%`;
			editingRelationship = { ...editingRelationship, [relationship.external_provider_pubkey]: false };
			await loadData();
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to update relationship";
		}
	}

	async function handleDeleteRelationship(relationship: ResellerRelationship) {
		const activeIdentity = signingIdentityInfo;
		if (!activeIdentity) {
			error = "Missing signing identity";
			return;
		}

		if (!confirm(`Are you sure you want to stop reselling for this provider?`)) {
			return;
		}

		error = null;
		successMessage = null;
		deletingRelationship = { ...deletingRelationship, [relationship.external_provider_pubkey]: true };

		try {
			const signed = await signRequest(
				activeIdentity.identity,
				"DELETE",
				`/api/v1/reseller/relationships/${relationship.external_provider_pubkey}`,
			);

			await deleteResellerRelationship(relationship.external_provider_pubkey, signed.headers);
			successMessage = "Reseller relationship deleted";
			await loadData();
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to delete relationship";
		} finally {
			deletingRelationship = { ...deletingRelationship, [relationship.external_provider_pubkey]: false };
		}
	}

	function getProviderName(providerPubkey: string): string {
		const provider = externalProviders.find(p => p.pubkey === providerPubkey);
		return provider?.name ?? providerPubkey.substring(0, 8);
	}

	onDestroy(() => {
		unsubscribeAuth?.();
	});
</script>

<div class="space-y-8">
	<header>
		<h1 class="text-4xl font-bold text-white mb-2">Reseller Program</h1>
		<p class="text-white/60">
			Become a reseller for external providers and earn commission on each order
		</p>
	</header>

	{#if !isAuthenticated}
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-8 border border-white/20 text-center">
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">💼</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-white/70">
					Create an account or login to become a reseller and manage your reseller relationships.
				</p>
				<button
					onclick={handleLogin}
					class="px-8 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 hover:scale-105 transition-all"
				>
					Login / Create Account
				</button>
			</div>
		</div>
	{:else}
		{#if error}
			<div class="bg-red-500/20 border border-red-500/30 rounded-lg p-4 text-red-300">
				{error}
			</div>
		{/if}
		{#if successMessage}
			<div class="bg-emerald-500/15 border border-emerald-500/30 rounded-lg p-4 text-emerald-300">
				{successMessage}
			</div>
		{/if}

		{#if loading}
			<div class="flex justify-center items-center py-12">
				<div class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-400"></div>
			</div>
		{:else}
			<!-- Your Reseller Relationships -->
			<section class="space-y-4">
				<div class="flex items-center justify-between">
					<h2 class="text-2xl font-semibold text-white">Your Reseller Relationships</h2>
					<span class="text-white/60 text-sm">{relationships.length} active</span>
				</div>

				{#if relationships.length === 0}
					<div class="bg-white/5 border border-white/10 rounded-xl p-6 text-white/70">
						You are not reselling for any providers yet. Browse the available providers below to get started.
					</div>
				{:else}
					<div class="space-y-3">
						{#each relationships as relationship}
							<div class="bg-white/5 border border-white/10 rounded-xl p-6 hover:bg-white/10 transition-colors">
								<div class="flex items-center justify-between">
									<div class="flex-1">
										<h3 class="text-lg font-semibold text-white">{getProviderName(relationship.external_provider_pubkey)}</h3>
										{#if editingRelationship[relationship.external_provider_pubkey]}
											<div class="mt-2 flex items-center gap-2">
												<input
													type="number"
													min="0"
													max="50"
													bind:value={editCommissionInputs[relationship.external_provider_pubkey]}
													class="w-24 px-3 py-1.5 bg-white/10 border border-white/20 rounded-lg text-white focus:outline-none focus:border-blue-400"
												/>
												<span class="text-white/70">% commission</span>
											</div>
										{:else}
											<p class="text-white/60 mt-1">{relationship.commission_percent}% commission · {relationship.status}</p>
										{/if}
									</div>
									<div class="flex items-center gap-2">
										{#if editingRelationship[relationship.external_provider_pubkey]}
											<button
												onclick={() => handleUpdateRelationship(relationship)}
												class="px-4 py-2 bg-emerald-500/20 border border-emerald-500/30 rounded-lg text-emerald-300 hover:bg-emerald-500/30 transition-colors"
											>
												Save
											</button>
											<button
												onclick={() => cancelEdit(relationship.external_provider_pubkey)}
												class="px-4 py-2 bg-white/10 border border-white/20 rounded-lg text-white/70 hover:bg-white/20 transition-colors"
											>
												Cancel
											</button>
										{:else}
											<button
												onclick={() => startEdit(relationship)}
												class="px-4 py-2 bg-blue-500/20 border border-blue-500/30 rounded-lg text-blue-300 hover:bg-blue-500/30 transition-colors"
											>
												Edit
											</button>
											<button
												onclick={() => handleDeleteRelationship(relationship)}
												disabled={deletingRelationship[relationship.external_provider_pubkey]}
												class="px-4 py-2 bg-red-500/20 border border-red-500/30 rounded-lg text-red-300 hover:bg-red-500/30 transition-colors disabled:opacity-50"
											>
												{deletingRelationship[relationship.external_provider_pubkey] ? "Deleting..." : "Delete"}
											</button>
										{/if}
									</div>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</section>

			<!-- Available External Providers -->
			<section class="space-y-4">
				<div class="flex items-center justify-between">
					<h2 class="text-2xl font-semibold text-white">Available External Providers</h2>
					<span class="text-white/60 text-sm">{externalProviders.length} providers</span>
				</div>

				{#if externalProviders.length === 0}
					<div class="bg-white/5 border border-white/10 rounded-xl p-6 text-white/70">
						No external providers available at this time.
					</div>
				{:else}
					<div class="space-y-3">
						{#each externalProviders as provider}
							{@const alreadyReseller = isAlreadyReseller(provider.pubkey)}
							<div class="bg-white/5 border border-white/10 rounded-xl p-6 hover:bg-white/10 transition-colors">
								<div class="flex items-center justify-between">
									<div class="flex-1">
										<div class="flex items-center gap-3">
											<h3 class="text-lg font-semibold text-white">{provider.name}</h3>
											{#if provider.logo_url}
												<img src={provider.logo_url} alt="{provider.name} logo" class="w-6 h-6 rounded" />
											{/if}
										</div>
										<p class="text-white/60 mt-1">
											{provider.offerings_count} offerings · {provider.domain}
										</p>
										{#if provider.website_url}
											<a href={provider.website_url} target="_blank" rel="noopener noreferrer" class="text-blue-400 hover:text-blue-300 text-sm mt-1 inline-block">
												{provider.website_url} ↗
											</a>
										{/if}
									</div>
									<div class="flex items-center gap-3">
										{#if !alreadyReseller}
											<div class="flex items-center gap-2">
												<input
													type="number"
													min="0"
													max="50"
													bind:value={commissionInputs[provider.pubkey]}
													placeholder="10"
													class="w-20 px-3 py-2 bg-white/10 border border-white/20 rounded-lg text-white focus:outline-none focus:border-blue-400"
												/>
												<span class="text-white/70 text-sm">%</span>
											</div>
											<button
												onclick={() => handleBecomeReseller(provider)}
												disabled={creatingRelationship[provider.pubkey]}
												class="px-6 py-2 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg text-white font-semibold hover:brightness-110 transition-all disabled:opacity-50"
											>
												{creatingRelationship[provider.pubkey] ? "Creating..." : "Become Reseller"}
											</button>
										{:else}
											<span class="px-4 py-2 bg-emerald-500/20 border border-emerald-500/30 rounded-lg text-emerald-300">
												✓ Active Reseller
											</span>
										{/if}
									</div>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</section>
		{/if}
	{/if}
</div>
