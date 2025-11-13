<script lang="ts">
	import { onMount } from "svelte";
	import { getUserContracts, type Contract, hexEncode } from "$lib/services/api";
	import { authStore } from "$lib/stores/auth";
	import { signRequest } from "$lib/services/auth-api";

	let contracts = $state<Contract[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	onMount(async () => {
		try {
			loading = true;
			error = null;
			
			const signingIdentityInfo = await authStore.getSigningIdentity();
			if (!signingIdentityInfo) {
				error = "You must be authenticated to view rentals";
				return;
			}
			
			const { headers } = await signRequest(
				signingIdentityInfo.identity as any,
				"GET",
				`/api/v1/users/${hexEncode(signingIdentityInfo.publicKeyBytes)}/contracts`
			);
			
			contracts = await getUserContracts(headers, hexEncode(signingIdentityInfo.publicKeyBytes));
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load rentals";
			console.error("Error loading rentals:", e);
		} finally {
			loading = false;
		}
	});

	function getStatusBadge(status: string): { text: string; class: string; icon: string } {
		switch (status.toLowerCase()) {
			case "requested":
				return {
					text: "Requested",
					class: "bg-yellow-500/20 text-yellow-400 border-yellow-500/30",
					icon: "🟡",
				};
			case "pending":
				return {
					text: "Pending",
					class: "bg-blue-500/20 text-blue-400 border-blue-500/30",
					icon: "🔵",
				};
			case "accepted":
				return {
					text: "Accepted",
					class: "bg-green-500/20 text-green-400 border-green-500/30",
					icon: "🟢",
				};
			case "provisioning":
				return {
					text: "Provisioning",
					class: "bg-purple-500/20 text-purple-400 border-purple-500/30",
					icon: "⚙️",
				};
			case "provisioned":
			case "active":
				return {
					text: "Active",
					class: "bg-emerald-500/20 text-emerald-400 border-emerald-500/30",
					icon: "✅",
				};
			case "rejected":
				return {
					text: "Rejected",
					class: "bg-red-500/20 text-red-400 border-red-500/30",
					icon: "🔴",
				};
			default:
				return {
					text: status,
					class: "bg-gray-500/20 text-gray-400 border-gray-500/30",
					icon: "⚪",
				};
		}
	}

	function formatDate(timestamp_ns?: number): string {
		if (!timestamp_ns) return "N/A";
		const date = new Date(timestamp_ns / 1_000_000);
		return date.toLocaleDateString() + " " + date.toLocaleTimeString();
	}

	function formatPrice(amount_e9s: number): string {
		return (amount_e9s / 1_000_000_000).toFixed(2) + " ICP";
	}

	function truncateHash(hash: string): string {
		if (hash.length <= 12) return hash;
		return hash.slice(0, 6) + "..." + hash.slice(-6);
	}
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">My Rentals</h1>
		<p class="text-white/60">
			View and manage your resource rental requests
		</p>
	</div>

	{#if error}
		<div
			class="bg-red-500/20 border border-red-500/30 rounded-lg p-4 text-red-400"
		>
			<p class="font-semibold">Error loading rentals</p>
			<p class="text-sm mt-1">{error}</p>
		</div>
	{/if}

	{#if loading}
		<div class="flex justify-center items-center p-8">
			<div
				class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-400"
			></div>
		</div>
	{:else if contracts.length === 0}
		<div class="text-center py-16">
			<span class="text-6xl mb-4 block">📋</span>
			<h3 class="text-2xl font-bold text-white mb-2">No Rentals Yet</h3>
			<p class="text-white/60 mb-6">
				You haven't created any rental requests yet
			</p>
			<a
				href="/dashboard/marketplace"
				class="inline-block px-6 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold hover:brightness-110 transition-all"
			>
				Browse Marketplace
			</a>
		</div>
	{:else}
		<div class="space-y-4">
			{#each contracts as contract}
				{@const statusBadge = getStatusBadge(contract.status)}
				<div
					class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20 hover:border-blue-400 transition-all"
				>
					<div class="flex items-start justify-between mb-4">
						<div class="flex-1">
							<div class="flex items-center gap-3 mb-2">
								<h3 class="text-xl font-bold text-white">
									{contract.offering_id}
								</h3>
								<span
									class="inline-flex items-center gap-1 px-3 py-1 rounded-full text-xs font-medium border {statusBadge.class}"
								>
									<span>{statusBadge.icon}</span>
									{statusBadge.text}
								</span>
							</div>
							<p class="text-white/60 text-sm">
								Contract ID: {truncateHash(contract.contract_id)}
							</p>
						</div>
						<div class="text-right">
							<div class="text-2xl font-bold text-white">
								{formatPrice(contract.payment_amount_e9s)}
							</div>
							{#if contract.duration_hours}
								<div class="text-white/60 text-sm">
									{contract.duration_hours} hours
								</div>
							{/if}
						</div>
					</div>

					<div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
						<div
							class="bg-white/5 rounded-lg p-3 border border-white/10"
						>
							<div class="text-white/60 text-xs mb-1">
								Created
							</div>
							<div class="text-white text-sm">
								{formatDate(contract.created_at_ns)}
							</div>
						</div>
						{#if contract.region_name}
							<div
								class="bg-white/5 rounded-lg p-3 border border-white/10"
							>
								<div class="text-white/60 text-xs mb-1">
									Region
								</div>
								<div class="text-white text-sm">
									{contract.region_name}
								</div>
							</div>
						{/if}
						{#if contract.requester_ssh_pubkey}
							<div
								class="bg-white/5 rounded-lg p-3 border border-white/10"
							>
								<div class="text-white/60 text-xs mb-1">
									SSH Key
								</div>
								<div
									class="text-white text-sm font-mono truncate"
								>
									{truncateHash(contract.requester_ssh_pubkey)}
								</div>
							</div>
						{/if}
						<div
							class="bg-white/5 rounded-lg p-3 border border-white/10"
						>
							<div class="text-white/60 text-xs mb-1">
								Provider
							</div>
							<div class="text-white text-sm font-mono">
								{truncateHash(contract.provider_pubkey_hash)}
							</div>
						</div>
					</div>

					{#if contract.request_memo}
						<div
							class="bg-white/5 rounded-lg p-3 border border-white/10 mb-4"
						>
							<div class="text-white/60 text-xs mb-1">Memo</div>
							<div class="text-white text-sm">
								{contract.request_memo}
							</div>
						</div>
					{/if}

					{#if contract.provisioning_instance_details}
						<div
							class="bg-green-500/10 border border-green-500/30 rounded-lg p-4"
						>
							<div class="text-green-400 font-semibold mb-2">
								Instance Details
							</div>
							<div class="text-white text-sm whitespace-pre-wrap">
								{contract.provisioning_instance_details}
							</div>
							{#if contract.provisioning_completed_at_ns}
								<div class="text-green-400/60 text-xs mt-2">
									Provisioned: {formatDate(
										contract.provisioning_completed_at_ns,
									)}
								</div>
							{/if}
						</div>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>
