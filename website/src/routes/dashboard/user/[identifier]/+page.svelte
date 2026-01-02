<script lang="ts">
	import { onMount } from "svelte";
	import { page } from "$app/stores";
	import { goto } from "$app/navigation";
	import {
		getUserActivity,
		type UserActivity,
	} from "$lib/services/api-user-activity";
	import { formatContractDate } from "$lib/utils/contract-format";
	import { truncatePubkey, isPubkeyHex } from "$lib/utils/identity";

	// Identifier can be a pubkey (64 hex chars) OR a username
	const identifier = $page.params.identifier ?? "";

	// Resolved pubkey (set after resolution in onMount)
	let pubkey = $state<string>("");
	// Username (set if identifier was a username or fetched from account)
	let username = $state<string | null>(null);

	let activity = $state<UserActivity | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let isNotFound = $state(false);

	onMount(async () => {
		try {
			loading = true;
			error = null;
			isNotFound = false;

			// Step 1: Resolve identifier to pubkey
			const { getAccount, getAccountByPublicKey } = await import(
				"$lib/services/account-api"
			);

			let resolvedPubkey: string | null = null;

			if (isPubkeyHex(identifier)) {
				// It's a pubkey - use it directly
				resolvedPubkey = identifier;
				// Try to get username from account for display and redirect
				const account = await getAccountByPublicKey(identifier).catch(
					() => null,
				);
				if (account) {
					username = account.username;
					// Redirect to username-based URL for cleaner URLs
					if (username && username !== identifier) {
						goto(`/dashboard/user/${username}`, {
							replaceState: true,
						});
						return;
					}
				}
			} else {
				// It's a username - look up the account
				const account = await getAccount(identifier).catch(() => null);
				if (
					account &&
					account.publicKeys &&
					account.publicKeys.length > 0
				) {
					username = account.username;
					// Get the first active public key
					const activeKey = account.publicKeys.find(
						(k) => k.isActive,
					);
					resolvedPubkey =
						activeKey?.publicKey ?? account.publicKeys[0].publicKey;
				}
			}

			if (!resolvedPubkey) {
				isNotFound = true;
				error = "User not found";
				loading = false;
				return;
			}

			pubkey = resolvedPubkey;

			// Step 2: Fetch user activity
			activity = await getUserActivity(pubkey);
		} catch (e) {
			const errorMessage =
				e instanceof Error ? e.message : "Failed to load user activity";
			error = errorMessage;
			isNotFound =
				errorMessage.includes("404") ||
				errorMessage.includes("Not Found");
			console.error("Error loading user activity:", e);
		} finally {
			loading = false;
		}
	});
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">User Info</h1>
		<p class="text-white/60">
			{#if username}
				Username: <span class="font-semibold">{username}</span>
			{:else if pubkey}
				Public Key: <span class="font-mono text-sm"
					>{truncatePubkey(pubkey)}</span
				>
			{:else}
				<span class="font-mono text-sm">{identifier}</span>
			{/if}
		</p>
	</div>

	{#if error}
		<div
			class="bg-red-500/20 border border-red-500/30 rounded-lg p-6 text-red-400"
		>
			{#if isNotFound}
				<div class="text-center">
					<div class="text-6xl mb-4">🔍</div>
					<h2 class="text-2xl font-bold mb-2">User Not Found</h2>
					<p class="mb-4">
						The user <span class="font-mono text-sm"
							>{identifier}</span
						> was not found in the system.
					</p>
					<p class="text-sm text-red-300/70">This could mean:</p>
					<ul
						class="text-sm text-red-300/70 list-disc list-inside mt-2"
					>
						<li>
							The user hasn't created any offerings or contracts
							yet
						</li>
						<li>The username or pubkey is incorrect</li>
						<li>The user is new to the platform</li>
					</ul>
					<div class="mt-6">
						<a
							href="/dashboard/marketplace"
							class="inline-flex items-center px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors"
						>
							← Back to Marketplace
						</a>
					</div>
				</div>
			{:else}
				<p class="font-semibold">Error loading user info</p>
				<p class="text-sm mt-1">{error}</p>
			{/if}
		</div>
	{/if}

	{#if loading}
		<div class="flex justify-center items-center p-8">
			<div
				class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-primary-400"
			></div>
		</div>
	{:else if activity}
		<!-- Offerings Provided -->
		<div class="space-y-4">
			<h2 class="text-2xl font-bold text-white">
				Offerings Provided ({activity.offerings_provided.length})
			</h2>
			{#if activity.offerings_provided.length === 0}
				<p class="text-white/60">No offerings provided yet.</p>
			{:else}
				<div
					class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4"
				>
					{#each activity.offerings_provided as offering}
						<div
							class="bg-glass/10 backdrop-blur-lg rounded-xl p-4 border border-glass/15"
						>
							<h3 class="text-lg font-semibold text-white mb-2">
								{offering.offer_name}
							</h3>
							<p class="text-sm text-white/60 mb-2">
								{offering.product_type}
							</p>
							<p class="text-white font-medium">
								{offering.monthly_price.toFixed(2)}
								{offering.currency}/mo
							</p>
							<span
								class="inline-block mt-2 px-2 py-1 rounded text-xs font-medium {offering.stock_status ===
								'in_stock'
									? 'bg-green-500/20 text-green-400'
									: 'bg-red-500/20 text-red-400'}"
							>
								{offering.stock_status}
							</span>
						</div>
					{/each}
				</div>
			{/if}
		</div>

		<!-- Rentals as Requester -->
		<div class="space-y-4">
			<h2 class="text-2xl font-bold text-white">
				Rentals (as Requester) ({activity.rentals_as_requester.length})
			</h2>
			{#if activity.rentals_as_requester.length === 0}
				<p class="text-white/60">No rental requests made yet.</p>
			{:else}
				<div class="space-y-3">
					{#each activity.rentals_as_requester as contract}
						<div
							class="bg-glass/10 backdrop-blur-lg rounded-lg p-4 border border-glass/15"
						>
							<div class="flex justify-between items-start mb-2">
								<div>
									<p class="text-white font-semibold">
										Offering: {contract.offering_id}
									</p>
									<p class="text-sm text-white/60">
										Provider:
										<a
											href="/dashboard/reputation/{contract.provider_pubkey}"
											class="text-primary-400 hover:text-primary-300"
										>
											{truncatePubkey(
												contract.provider_pubkey,
											)}
										</a>
									</p>
								</div>
								<span
									class="px-3 py-1 rounded-full text-xs font-medium bg-primary-500/20 text-primary-400"
								>
									{contract.status}
								</span>
							</div>
							<p class="text-sm text-white/60">
								Created: {formatContractDate(
									contract.created_at_ns,
								)}
							</p>
							{#if contract.duration_hours}
								<p class="text-sm text-white/60">
									Duration: {contract.duration_hours} hours
								</p>
							{/if}
						</div>
					{/each}
				</div>
			{/if}
		</div>

		<!-- Rentals as Provider -->
		<div class="space-y-4">
			<h2 class="text-2xl font-bold text-white">
				Rentals (as Provider) ({activity.rentals_as_provider.length})
			</h2>
			{#if activity.rentals_as_provider.length === 0}
				<p class="text-white/60">No rentals provided yet.</p>
			{:else}
				<div class="space-y-3">
					{#each activity.rentals_as_provider as contract}
						<div
							class="bg-glass/10 backdrop-blur-lg rounded-lg p-4 border border-glass/15"
						>
							<div class="flex justify-between items-start mb-2">
								<div>
									<p class="text-white font-semibold">
										Offering: {contract.offering_id}
									</p>
									<p class="text-sm text-white/60">
										Requester:
										<a
											href="/dashboard/reputation/{contract.requester_pubkey}"
											class="text-primary-400 hover:text-primary-300"
										>
											{truncatePubkey(
												contract.requester_pubkey,
											)}
										</a>
									</p>
								</div>
								<span
									class="px-3 py-1 rounded-full text-xs font-medium bg-purple-500/20 text-purple-400"
								>
									{contract.status}
								</span>
							</div>
							<p class="text-sm text-white/60">
								Created: {formatContractDate(
									contract.created_at_ns,
								)}
							</p>
							{#if contract.duration_hours}
								<p class="text-sm text-white/60">
									Duration: {contract.duration_hours} hours
								</p>
							{/if}
						</div>
					{/each}
				</div>
			{/if}
		</div>
	{/if}
</div>
