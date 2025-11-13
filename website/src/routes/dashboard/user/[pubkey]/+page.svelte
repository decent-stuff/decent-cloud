<script lang="ts">
	import { onMount } from "svelte";
	import { page } from "$app/stores";
	import { getUserActivity, type UserActivity } from "$lib/services/api-user-activity";
	import { formatContractDate } from "$lib/utils/contract-format";

	const pubkey = $page.params.pubkey ?? "";

	let activity = $state<UserActivity | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);

	onMount(async () => {
		try {
			loading = true;
			error = null;
			activity = await getUserActivity(pubkey);
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load user activity";
			console.error("Error loading user activity:", e);
		} finally {
			loading = false;
		}
	});

	function shortPubkey(fullPubkey: string): string {
		if (fullPubkey.length <= 12) return fullPubkey;
		return `${fullPubkey.slice(0, 6)}...${fullPubkey.slice(-6)}`;
	}
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">User Info</h1>
		<p class="text-white/60">
			Public Key: <span class="font-mono text-sm">{shortPubkey(pubkey)}</span>
		</p>
	</div>

	{#if error}
		<div
			class="bg-red-500/20 border border-red-500/30 rounded-lg p-4 text-red-400"
		>
			<p class="font-semibold">Error loading user info</p>
			<p class="text-sm mt-1">{error}</p>
		</div>
	{/if}

	{#if loading}
		<div class="flex justify-center items-center p-8">
			<div
				class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-400"
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
				<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
					{#each activity.offerings_provided as offering}
						<div
							class="bg-white/10 backdrop-blur-lg rounded-xl p-4 border border-white/20"
						>
							<h3 class="text-lg font-semibold text-white mb-2">
								{offering.offer_name}
							</h3>
							<p class="text-sm text-white/60 mb-2">
								{offering.product_type}
							</p>
							<p class="text-white font-medium">
								{offering.monthly_price.toFixed(2)} {offering.currency}/mo
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
							class="bg-white/10 backdrop-blur-lg rounded-lg p-4 border border-white/20"
						>
							<div class="flex justify-between items-start mb-2">
								<div>
									<p class="text-white font-semibold">
										Offering: {contract.offering_id}
									</p>
									<p class="text-sm text-white/60">
										Provider: {shortPubkey(contract.provider_pubkey_hash)}
									</p>
								</div>
								<span
									class="px-3 py-1 rounded-full text-xs font-medium bg-blue-500/20 text-blue-400"
								>
									{contract.status}
								</span>
							</div>
							<p class="text-sm text-white/60">
								Created: {formatContractDate(contract.created_at_ns)}
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
							class="bg-white/10 backdrop-blur-lg rounded-lg p-4 border border-white/20"
						>
							<div class="flex justify-between items-start mb-2">
								<div>
									<p class="text-white font-semibold">
										Offering: {contract.offering_id}
									</p>
									<p class="text-sm text-white/60">
										Requester: {shortPubkey(contract.requester_pubkey_hash)}
									</p>
								</div>
								<span
									class="px-3 py-1 rounded-full text-xs font-medium bg-purple-500/20 text-purple-400"
								>
									{contract.status}
								</span>
							</div>
							<p class="text-sm text-white/60">
								Created: {formatContractDate(contract.created_at_ns)}
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
