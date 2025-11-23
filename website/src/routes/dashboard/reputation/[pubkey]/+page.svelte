<script lang="ts">
	import { onMount } from "svelte";
	import { page } from "$app/stores";
	import {
		getUserActivity,
		type UserActivity,
	} from "$lib/services/api-user-activity";
	import {
		getReputation,
		getAccountBalance,
		getAccountTransfers,
		type ReputationInfo,
		type TokenTransfer,
	} from "$lib/services/api-reputation";
	import {
		getUserProfile,
		getUserContacts,
		getUserSocials,
	} from "$lib/services/api-user-profile";
	import type { UserProfile } from "$lib/types/generated/UserProfile";
	import type { UserContact } from "$lib/types/generated/UserContact";
	import type { UserSocial } from "$lib/types/generated/UserSocial";
	import {
		formatContractDate,
		computePubkey,
		derivePrincipalFromPubkey,
	} from "$lib/utils/contract-format";
	import { authStore } from "$lib/stores/auth";
	import type { IdentityInfo } from "$lib/stores/auth";

	const pubkey = $page.params.pubkey ?? "";

	let activity = $state<UserActivity | null>(null);
	let reputation = $state<ReputationInfo | null>(null);
	let balance = $state<number>(0);
	let transfers = $state<TokenTransfer[]>([]);
	let profile = $state<UserProfile | null>(null);
	let contacts = $state<UserContact[]>([]);
	let socials = $state<UserSocial[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let isNotFound = $state(false);
	let currentIdentity = $state<IdentityInfo | null>(null);

	authStore.currentIdentity.subscribe((value) => {
		currentIdentity = value;
	});

	// Check if viewing own profile and derive principal
	const isOwnProfile = $derived(
		currentIdentity?.publicKeyBytes &&
			computePubkey(currentIdentity.publicKeyBytes) === pubkey,
	);

	// Derive IC Principal from the public key hex string in the URL
	const derivedPrincipal = $derived(
		(() => {
			if (!pubkey || pubkey.length !== 64) return null; // Ed25519 keys are 32 bytes = 64 hex chars
			try {
				// Convert hex string to bytes
				const pubkeyBytes = new Uint8Array(
					pubkey.match(/.{1,2}/g)!.map((byte) => parseInt(byte, 16)),
				);
				return derivePrincipalFromPubkey(pubkeyBytes).toText();
			} catch {
				return null;
			}
		})(),
	);

	// Helper to format account addresses
	function shortPubkey(fullPubkey: string): string {
		if (fullPubkey.length <= 12) return fullPubkey;
		return `${fullPubkey.slice(0, 6)}...${fullPubkey.slice(-6)}`;
	}

	// Format balance from e9s to tokens
	function formatBalance(balanceE9s: number): string {
		return (balanceE9s / 1_000_000_000).toFixed(4);
	}

	// Format timestamp
	function formatTimestamp(timestampNs: number): string {
		const date = new Date(timestampNs / 1_000_000);
		return date.toLocaleString();
	}

	// Calculate total spent and received
	function calculateTransactionStats(
		transfers: TokenTransfer[],
		account: string,
	) {
		let totalSent = 0;
		let totalReceived = 0;

		for (const transfer of transfers) {
			if (transfer.from_account === account) {
				totalSent += transfer.amount_e9s + transfer.fee_e9s;
			}
			if (transfer.to_account === account) {
				totalReceived += transfer.amount_e9s;
			}
		}

		return { totalSent, totalReceived };
	}

	onMount(async () => {
		try {
			loading = true;
			error = null;
			isNotFound = false;

			// Fetch all data in parallel
			const [
				activityData,
				reputationData,
				balanceData,
				transfersData,
				profileData,
				socialsData,
				contactsData,
			] = await Promise.all([
				getUserActivity(pubkey).catch(() => null),
				getReputation(pubkey).catch(() => null),
				getAccountBalance(pubkey).catch(() => 0),
				getAccountTransfers(pubkey, 100).catch(() => []),
				getUserProfile(pubkey).catch(() => null),
				getUserSocials(pubkey).catch(() => []),
				getUserContacts(pubkey).catch(() => []),
			]);

			activity = activityData;
			reputation = reputationData;
			balance = balanceData;
			transfers = transfersData;
			profile = profileData;
			contacts = contactsData;
			socials = socialsData;

			// Check if account exists in the new account system
			// Try to fetch account by public key
			let accountExists = false;
			try {
				const { getAccountByPublicKey } = await import('$lib/services/account-api');
				const account = await getAccountByPublicKey(pubkey);
				if (account) {
					accountExists = true;
					// Use account username as display name if no profile
					if (!profile) {
						profile = {
							displayName: account.username,
							bio: null,
							avatarUrl: null,
							updated_at_ns: Date.now() * 1_000_000
						};
					}
				}
			} catch {
				// Account lookup failed, continue with old logic
			}

			// If we have no data at all AND account doesn't exist, mark as not found
			const hasActivity =
				activity &&
				(activity.offerings_provided.length > 0 ||
					activity.rentals_as_requester.length > 0 ||
					activity.rentals_as_provider.length > 0);

			if (
				!accountExists &&
				!hasActivity &&
				!reputation &&
				balance === 0 &&
				transfers.length === 0 &&
				!profile
			) {
				isNotFound = true;
				error = "Account not found";
			}
		} catch (e) {
			const errorMessage =
				e instanceof Error
					? e.message
					: "Failed to load account information";
			error = errorMessage;
			isNotFound =
				errorMessage.includes("404") ||
				errorMessage.includes("Not Found") ||
				errorMessage.includes("not found");
			console.error("Error loading account information:", e);
		} finally {
			loading = false;
		}
	});

	const txStats = $derived(
		transfers.length > 0
			? calculateTransactionStats(transfers, pubkey)
			: null,
	);
	const totalContracts = $derived(
		(activity?.rentals_as_requester.length ?? 0) +
			(activity?.rentals_as_provider.length ?? 0),
	);
</script>

<div class="space-y-8 max-w-7xl mx-auto p-6">
	<!-- Page Header -->
	<div
		class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20"
	>
		<div class="flex items-start justify-between gap-4 mb-4">
			<div class="flex-1">
				<h1 class="text-4xl font-bold text-white mb-2">
					{profile?.displayName || "Account Reputation"}
				</h1>
				{#if profile?.bio}
					<p class="text-white/70 text-sm mb-3">{profile.bio}</p>
				{/if}
			</div>
			{#if profile?.avatarUrl}
				<img
					src={profile.avatarUrl}
					alt="Avatar"
					class="w-20 h-20 rounded-full border-2 border-white/20"
				/>
			{/if}
		</div>
		<div class="space-y-3">
			<div>
				<p class="text-white/80 text-sm">Public Key:</p>
				<p class="font-mono text-sm text-white/90 break-all">
					{pubkey}
				</p>
			</div>
			{#if derivedPrincipal}
				<div>
					<p class="text-white/80 text-sm">IC Principal:</p>
					<p class="font-mono text-sm text-white/90 break-all">
						{derivedPrincipal}
					</p>
				</div>
			{/if}
		</div>

		<!-- Contact Information & Socials -->
		{#if contacts.length > 0 || socials.length > 0}
			<div class="mt-4 pt-4 border-t border-white/10">
				<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
					<!-- Contacts -->
					{#if contacts.length > 0}
						<div>
							<h3
								class="text-sm font-semibold text-white/80 mb-2"
							>
								Contact
							</h3>
							<div class="space-y-2">
								{#each contacts as contact}
									<div
										class="flex items-center gap-2 text-sm"
									>
										<span class="text-white/60 capitalize"
											>{contact.contactType}:</span
										>
										{#if contact.contactType === "email"}
											<a
												href="mailto:{contact.contactValue}"
												class="text-blue-400 hover:text-blue-300"
											>
												{contact.contactValue}
											</a>
										{:else if contact.contactType === "discord"}
											<span class="text-white/90"
												>{contact.contactValue}</span
											>
										{:else}
											<span class="text-white/90"
												>{contact.contactValue}</span
											>
										{/if}
										{#if contact.verified}
											<span class="text-green-400 text-xs"
												>✓</span
											>
										{/if}
									</div>
								{/each}
							</div>
						</div>
					{/if}

					<!-- Social Links -->
					{#if socials.length > 0}
						<div>
							<h3
								class="text-sm font-semibold text-white/80 mb-2"
							>
								Social
							</h3>
							<div class="space-y-2">
								{#each socials as social}
									<div
										class="flex items-center gap-2 text-sm"
									>
										<span class="text-white/60 capitalize"
											>{social.platform}:</span
										>
										{#if social.profileUrl}
											<a
												href={social.profileUrl}
												target="_blank"
												rel="noopener noreferrer"
												class="text-blue-400 hover:text-blue-300"
											>
												@{social.username} →
											</a>
										{:else}
											<span class="text-white/90"
												>@{social.username}</span
											>
										{/if}
									</div>
								{/each}
							</div>
						</div>
					{/if}
				</div>
			</div>
		{/if}
	</div>

	{#if loading}
		<div class="flex justify-center items-center p-8">
			<div
				class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-400"
			></div>
		</div>
	{:else if error && isNotFound}
		<div
			class="bg-yellow-500/20 border border-yellow-500/30 rounded-lg p-6 text-yellow-300"
		>
			<div class="text-center">
				<div class="text-6xl mb-4">🔍</div>
				<h2 class="text-2xl font-bold mb-2">No Account Data</h2>
				<p class="mb-4">
					The public key <span class="font-mono text-sm"
						>{shortPubkey(pubkey)}</span
					>
					is not registered in the system.
				</p>
				<p class="text-sm text-yellow-300/70">Please verify:</p>
				<ul class="text-sm text-yellow-300/70 list-disc list-inside mt-2">
					<li>The public key address is correct</li>
					<li>The account has been registered with a username</li>
				</ul>
				<div class="mt-6">
					<a
						href="/dashboard/marketplace"
						class="inline-flex items-center px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
					>
						← Back to Marketplace
					</a>
				</div>
			</div>
		</div>
	{:else}
		<!-- Overview Stats -->
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
			<!-- Balance -->
			<div
				class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20"
			>
				<div class="text-white/60 text-sm mb-1">Account Balance</div>
				<div class="text-3xl font-bold text-white">
					{formatBalance(balance)}
					<span class="text-xl text-white/60">DC</span>
				</div>
			</div>

			<!-- Reputation -->
			<div
				class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20"
			>
				<div class="text-white/60 text-sm mb-1">Reputation Score</div>
				<div class="text-3xl font-bold text-white">
					{reputation?.total_reputation ?? 0}
				</div>
				{#if reputation}
					<div class="text-xs text-white/50 mt-1">
						{reputation.change_count} reputation changes
					</div>
				{/if}
			</div>

			<!-- Total Contracts -->
			<div
				class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20"
			>
				<div class="text-white/60 text-sm mb-1">Total Contracts</div>
				<div class="text-3xl font-bold text-white">
					{totalContracts}
				</div>
				<div class="text-xs text-white/50 mt-1">
					{activity?.rentals_as_requester.length ?? 0} as requester, {activity
						?.rentals_as_provider.length ?? 0} as provider
				</div>
			</div>

			<!-- Total Offerings -->
			<div
				class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20"
			>
				<div class="text-white/60 text-sm mb-1">Offerings</div>
				<div class="text-3xl font-bold text-white">
					{activity?.offerings_provided.length ?? 0}
				</div>
			</div>
		</div>

		<!-- Transaction Statistics -->
		{#if txStats}
			<div
				class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20"
			>
				<h2 class="text-2xl font-bold text-white mb-4">
					Transaction Statistics
				</h2>
				<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
					<div>
						<div class="text-white/60 text-sm mb-1">
							Total Spent
						</div>
						<div class="text-2xl font-bold text-red-400">
							{formatBalance(txStats.totalSent)} DC
						</div>
					</div>
					<div>
						<div class="text-white/60 text-sm mb-1">
							Total Received
						</div>
						<div class="text-2xl font-bold text-green-400">
							{formatBalance(txStats.totalReceived)} DC
						</div>
					</div>
					<div>
						<div class="text-white/60 text-sm mb-1">
							Total Transactions
						</div>
						<div class="text-2xl font-bold text-white">
							{transfers.length}
						</div>
					</div>
				</div>
			</div>
		{/if}

		<!-- Recent Transactions -->
		{#if transfers.length > 0}
			<div
				class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20"
			>
				<h2 class="text-2xl font-bold text-white mb-4">
					Recent Transactions
				</h2>
				<div class="space-y-3 max-h-96 overflow-y-auto">
					{#each transfers.slice(0, 20) as transfer}
						<div
							class="bg-white/5 rounded-lg p-4 border border-white/10"
						>
							<div class="flex justify-between items-start mb-2">
								<div class="flex-1">
									<div class="flex items-center gap-2 mb-1">
										<span class="text-white/60 text-sm"
											>From:</span
										>
										<a
											href="/dashboard/reputation/{transfer.from_account}"
											class="font-mono text-sm text-blue-400 hover:text-blue-300"
										>
											{shortPubkey(transfer.from_account)}
										</a>
									</div>
									<div class="flex items-center gap-2">
										<span class="text-white/60 text-sm"
											>To:</span
										>
										<a
											href="/dashboard/reputation/{transfer.to_account}"
											class="font-mono text-sm text-blue-400 hover:text-blue-300"
										>
											{shortPubkey(transfer.to_account)}
										</a>
									</div>
								</div>
								<div class="text-right">
									<div class="text-lg font-bold text-white">
										{formatBalance(transfer.amount_e9s)} DC
									</div>
									<div class="text-xs text-white/50">
										Fee: {formatBalance(transfer.fee_e9s)} DC
									</div>
								</div>
							</div>
							<div class="text-xs text-white/50">
								{formatTimestamp(transfer.created_at_ns)}
							</div>
							{#if transfer.memo}
								<div class="mt-2 text-sm text-white/70 italic">
									{transfer.memo}
								</div>
							{/if}
						</div>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Offerings Provided -->
		{#if activity && activity.offerings_provided.length > 0}
			<div
				class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20"
			>
				<h2 class="text-2xl font-bold text-white mb-4">
					Offerings Provided ({activity.offerings_provided.length})
				</h2>
				<div
					class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4"
				>
					{#each activity.offerings_provided as offering}
						<div
							class="bg-white/5 rounded-xl p-4 border border-white/10"
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
			</div>
		{/if}

		<!-- Rentals as Requester -->
		{#if activity && activity.rentals_as_requester.length > 0}
			<div
				class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20"
			>
				<h2 class="text-2xl font-bold text-white mb-4">
					Rentals as Requester ({activity.rentals_as_requester
						.length})
				</h2>
				<div class="space-y-3">
					{#each activity.rentals_as_requester as contract}
						<div
							class="bg-white/5 rounded-lg p-4 border border-white/10"
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
											class="text-blue-400 hover:text-blue-300"
										>
											{shortPubkey(
												contract.provider_pubkey,
											)}
										</a>
									</p>
									<p class="text-sm text-white/60">
										Amount: {formatBalance(
											contract.payment_amount_e9s,
										)} DC
									</p>
								</div>
								<span
									class="px-3 py-1 rounded-full text-xs font-medium bg-blue-500/20 text-blue-400"
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
			</div>
		{/if}

		<!-- Rentals as Provider -->
		{#if activity && activity.rentals_as_provider.length > 0}
			<div
				class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20"
			>
				<h2 class="text-2xl font-bold text-white mb-4">
					Rentals as Provider ({activity.rentals_as_provider.length})
				</h2>
				<div class="space-y-3">
					{#each activity.rentals_as_provider as contract}
						<div
							class="bg-white/5 rounded-lg p-4 border border-white/10"
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
											class="text-blue-400 hover:text-blue-300"
										>
											{shortPubkey(
												contract.requester_pubkey,
											)}
										</a>
									</p>
									<p class="text-sm text-white/60">
										Amount: {formatBalance(
											contract.payment_amount_e9s,
										)} DC
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
			</div>
		{/if}
	{/if}
</div>
