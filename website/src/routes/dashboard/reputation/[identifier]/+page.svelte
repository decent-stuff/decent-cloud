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
	import {
		getProviderTrustMetrics,
		getProviderResponseMetrics,
		type ProviderTrustMetrics,
		type ProviderResponseMetrics,
	} from "$lib/services/api";
	import TrustDashboard from "$lib/components/TrustDashboard.svelte";
	import Icon from "$lib/components/Icons.svelte";
	import type { UserProfile } from "$lib/types/generated/UserProfile";
	import type { UserContact } from "$lib/types/generated/UserContact";
	import type { UserSocial } from "$lib/types/generated/UserSocial";
	import {
		formatContractDate,
		computePubkey,
		derivePrincipalFromPubkey,
		calculateActualDuration,
		formatDuration,
	} from "$lib/utils/contract-format";
	import { truncatePubkey, isPubkeyHex } from "$lib/utils/identity";
	import { authStore } from "$lib/stores/auth";
	import type { IdentityInfo } from "$lib/stores/auth";

	const identifier = $page.params.identifier ?? "";

	let pubkey = $state<string>("");
	let username = $state<string | null>(null);
	let activity = $state<UserActivity | null>(null);
	let reputation = $state<ReputationInfo | null>(null);
	let balance = $state<number>(0);
	let transfers = $state<TokenTransfer[]>([]);
	let profile = $state<UserProfile | null>(null);
	let contacts = $state<UserContact[]>([]);
	let socials = $state<UserSocial[]>([]);
	let trustMetrics = $state<ProviderTrustMetrics | null>(null);
	let responseMetrics = $state<ProviderResponseMetrics | null>(null);
	let accountInfo = $state<{ emailVerified: boolean; email?: string } | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let isNotFound = $state(false);
	let currentIdentity = $state<IdentityInfo | null>(null);

	const filteredContacts = $derived(contacts.filter(c => c.contactType !== 'email'));

	authStore.currentIdentity.subscribe((value) => {
		currentIdentity = value;
	});

	const isOwnProfile = $derived(
		currentIdentity?.publicKeyBytes &&
			computePubkey(currentIdentity.publicKeyBytes) === pubkey,
	);

	const derivedPrincipal = $derived(
		(() => {
			if (!pubkey || pubkey.length !== 64) return null;
			try {
				const pubkeyBytes = new Uint8Array(
					pubkey.match(/.{1,2}/g)!.map((byte) => parseInt(byte, 16)),
				);
				return derivePrincipalFromPubkey(pubkeyBytes).toText();
			} catch {
				return null;
			}
		})(),
	);

	function formatBalance(balanceE9s: number, currency: string): string {
		return (balanceE9s / 1_000_000_000).toFixed(4);
	}

	function formatTimestamp(timestampNs: number): string {
		const date = new Date(timestampNs / 1_000_000);
		return date.toLocaleString();
	}

	function calculateTransactionStats(transfers: TokenTransfer[], account: string) {
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

	function calculateCancellationMetrics(contracts: any[]) {
		if (contracts.length === 0) return null;

		const cancelled = contracts.filter((c) => c.status === 'cancelled');
		if (cancelled.length === 0) return {
			total: 0,
			within1h: 0,
			within24h: 0,
			within7d: 0,
			within180d: 0,
			pct1h: 0,
			pct24h: 0,
			pct7d: 0,
			pct180d: 0,
		};

		const ONE_HOUR_NS = 60 * 60 * 1_000_000_000;
		const ONE_DAY_NS = 24 * ONE_HOUR_NS;
		const ONE_WEEK_NS = 7 * ONE_DAY_NS;
		const SIX_MONTHS_NS = 180 * ONE_DAY_NS;

		let within1h = 0;
		let within24h = 0;
		let within7d = 0;
		let within180d = 0;

		for (const contract of cancelled) {
			if (!contract.status_updated_at_ns) continue;
			const duration = contract.status_updated_at_ns - contract.created_at_ns;

			if (duration <= ONE_HOUR_NS) within1h++;
			if (duration <= ONE_DAY_NS) within24h++;
			if (duration <= ONE_WEEK_NS) within7d++;
			if (duration <= SIX_MONTHS_NS) within180d++;
		}

		return {
			total: cancelled.length,
			within1h,
			within24h,
			within7d,
			within180d,
			pct1h: (within1h / cancelled.length) * 100,
			pct24h: (within24h / cancelled.length) * 100,
			pct7d: (within7d / cancelled.length) * 100,
			pct180d: (within180d / cancelled.length) * 100,
		};
	}

	onMount(async () => {
		try {
			loading = true;
			error = null;
			isNotFound = false;

			const { getAccount, getAccountByPublicKey } = await import('$lib/services/account-api');

			let accountExists = false;
			let resolvedPubkey: string | null = null;

			if (isPubkeyHex(identifier)) {
				resolvedPubkey = identifier;
				const account = await getAccountByPublicKey(identifier).catch(() => null);
				if (account) {
					accountExists = true;
					username = account.username;
					accountInfo = {
						emailVerified: account.emailVerified,
						email: account.email,
					};
					if (username && username !== identifier) {
						history.replaceState(history.state, '', `/dashboard/reputation/${username}`);
					}
				}
			} else {
				const account = await getAccount(identifier).catch(() => null);
				if (account && account.publicKeys && account.publicKeys.length > 0) {
					accountExists = true;
					username = account.username;
					accountInfo = {
						emailVerified: account.emailVerified,
						email: account.email,
					};
					const activeKey = account.publicKeys.find((k) => k.isActive);
					resolvedPubkey = activeKey?.publicKey ?? account.publicKeys[0].publicKey;
				}
			}

			if (!resolvedPubkey) {
				isNotFound = true;
				error = "Account not found";
				loading = false;
				return;
			}

			pubkey = resolvedPubkey;

			const [
				activityData,
				reputationData,
				balanceData,
				transfersData,
				profileData,
				socialsData,
				contactsData,
				trustMetricsData,
				responseMetricsData,
			] = await Promise.all([
				getUserActivity(pubkey).catch(() => null),
				getReputation(pubkey).catch(() => null),
				getAccountBalance(pubkey).catch(() => 0),
				getAccountTransfers(pubkey, 100).catch(() => []),
				getUserProfile(pubkey).catch(() => null),
				getUserSocials(pubkey).catch(() => []),
				getUserContacts(pubkey).catch(() => []),
				getProviderTrustMetrics(pubkey).catch(() => null),
				getProviderResponseMetrics(pubkey).catch(() => null),
			]);

			activity = activityData;
			reputation = reputationData;
			balance = balanceData;
			transfers = transfersData;
			profile = profileData;
			contacts = contactsData;
			socials = socialsData;
			trustMetrics = trustMetricsData;
			responseMetrics = responseMetricsData;

			if (!profile && username) {
				profile = {
					displayName: username,
					bio: null,
					avatarUrl: null,
					updated_at_ns: Date.now() * 1_000_000
				};
			}

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
	const requesterCancellations = $derived(
		activity?.rentals_as_requester
			? calculateCancellationMetrics(activity.rentals_as_requester)
			: null,
	);
	const providerCancellations = $derived(
		activity?.rentals_as_provider
			? calculateCancellationMetrics(activity.rentals_as_provider)
			: null,
	);
</script>

<div class="space-y-6">
	{#if loading}
		<div class="flex justify-center items-center p-8">
			<div class="w-8 h-8 border-2 border-primary-500/30 border-t-primary-500 animate-spin"></div>
		</div>
	{:else if error && isNotFound}
		<div class="bg-warning/10 border border-warning/20 p-6">
			<div class="text-center">
				<div class="icon-box mx-auto mb-4">
					<Icon name="search" size={20} />
				</div>
				<h2 class="text-lg font-semibold text-white mb-2">No Account Data</h2>
				<p class="text-neutral-400 text-sm mb-4">
					The identifier <span class="font-mono text-neutral-300">{identifier}</span> is not registered in the system.
				</p>
				<p class="text-xs text-neutral-500">Please verify:</p>
				<ul class="text-xs text-neutral-500 list-disc list-inside mt-2">
					<li>The public key address is correct</li>
					<li>The account has been registered with a username</li>
				</ul>
				<div class="mt-6">
					<a href="/dashboard/marketplace" class="btn-secondary inline-flex items-center gap-2">
						<Icon name="arrow-left" size={14} />
						<span>Back to Marketplace</span>
					</a>
				</div>
			</div>
		</div>
	{:else}
		<!-- Page Header -->
		<div class="card p-5">
			<div class="flex items-start justify-between gap-4 mb-4">
				<div class="flex-1">
					<h1 class="text-2xl font-bold text-white tracking-tight mb-2">
						{profile?.displayName || "Account Reputation"}
					</h1>
					{#if profile?.bio}
						<p class="text-neutral-400 text-sm mb-3">{profile.bio}</p>
					{/if}
				</div>
				{#if profile?.avatarUrl}
					<img
						src={profile.avatarUrl}
						alt="Avatar"
						class="w-16 h-16 border border-neutral-800"
					/>
				{/if}
			</div>
			<div class="space-y-3">
				<div>
					<p class="data-label mb-1">Public Key</p>
					<p class="font-mono text-xs text-neutral-300 break-all">{pubkey}</p>
				</div>
				{#if derivedPrincipal}
					<div>
						<p class="data-label mb-1">IC Principal</p>
						<p class="font-mono text-xs text-neutral-300 break-all">{derivedPrincipal}</p>
					</div>
				{/if}
			</div>

			<!-- Contact Information & Socials -->
			{#if filteredContacts.length > 0 || socials.length > 0 || accountInfo?.emailVerified}
				<div class="mt-4 pt-4 border-t border-neutral-800/80">
					<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
						{#if filteredContacts.length > 0 || accountInfo?.emailVerified}
							<div>
								<h3 class="data-label mb-2">Contact</h3>
								<div class="space-y-2">
									{#if accountInfo?.emailVerified}
										<div class="flex items-center gap-2 text-sm">
											<span class="text-neutral-500">Email:</span>
											<span class="badge badge-success">Verified</span>
										</div>
									{/if}
									{#each filteredContacts as contact}
										<div class="flex items-center gap-2 text-sm">
											<span class="text-neutral-500 capitalize">{contact.contactType}:</span>
											<span class="text-neutral-200">{contact.contactValue}</span>
											{#if contact.verified}
												<Icon name="check" size={12} class="text-success" />
											{/if}
										</div>
									{/each}
								</div>
							</div>
						{/if}

						{#if socials.length > 0}
							<div>
								<h3 class="data-label mb-2">Social</h3>
								<div class="space-y-2">
									{#each socials as social}
										<div class="flex items-center gap-2 text-sm">
											<span class="text-neutral-500 capitalize">{social.platform}:</span>
											{#if social.profileUrl}
												<a
													href={social.profileUrl}
													target="_blank"
													rel="noopener noreferrer"
													class="text-primary-400 hover:text-primary-300"
												>
													@{social.username}
													<Icon name="external" size={10} class="inline ml-1" />
												</a>
											{:else}
												<span class="text-neutral-200">@{social.username}</span>
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

		<!-- Overview Stats -->
		<div class="grid grid-cols-2 md:grid-cols-4 gap-3">
			<div class="metric-card">
				<div class="metric-label">Balance</div>
				<div class="metric-value">
					{formatBalance(balance, 'dct')}
					<span class="text-lg text-neutral-500">DC</span>
				</div>
			</div>

			<div class="metric-card">
				<div class="metric-label">Reputation</div>
				<div class="metric-value">{reputation?.total_reputation ?? 0}</div>
				{#if reputation}
					<div class="metric-subtext">{reputation.change_count} changes</div>
				{/if}
			</div>

			<div class="metric-card">
				<div class="metric-label">Contracts</div>
				<div class="metric-value">{totalContracts}</div>
				<div class="metric-subtext">
					{activity?.rentals_as_requester.length ?? 0} req / {activity?.rentals_as_provider.length ?? 0} prov
				</div>
			</div>

			<div class="metric-card">
				<div class="metric-label">Offerings</div>
				<div class="metric-value">{activity?.offerings_provided.length ?? 0}</div>
			</div>
		</div>

		<!-- Trust Dashboard -->
		{#if trustMetrics}
			<TrustDashboard metrics={trustMetrics} {responseMetrics} />
		{/if}

		<!-- Cancellation Metrics -->
		{#if (requesterCancellations && requesterCancellations.total > 0) || (providerCancellations && providerCancellations.total > 0)}
			<div class="card p-5">
				<h2 class="text-lg font-semibold text-white mb-4">Cancellation Patterns</h2>
				<div class="grid grid-cols-1 md:grid-cols-2 gap-6">
					{#if requesterCancellations && requesterCancellations.total > 0}
						<div>
							<h3 class="text-sm font-medium text-neutral-400 mb-3">As Requester</h3>
							<div class="space-y-2">
								<div class="flex justify-between items-center">
									<span class="text-sm text-neutral-500">Total cancelled:</span>
									<span class="text-base font-semibold text-white">{requesterCancellations.total}</span>
								</div>
								<div class="flex justify-between items-center">
									<span class="text-sm text-neutral-500">Within 1 hour:</span>
									<span class="text-base font-semibold {requesterCancellations.pct1h > 50 ? 'text-danger' : 'text-white'}">
										{requesterCancellations.within1h} ({requesterCancellations.pct1h.toFixed(0)}%)
									</span>
								</div>
								<div class="flex justify-between items-center">
									<span class="text-sm text-neutral-500">Within 24 hours:</span>
									<span class="text-base font-semibold {requesterCancellations.pct24h > 80 ? 'text-warning' : 'text-white'}">
										{requesterCancellations.within24h} ({requesterCancellations.pct24h.toFixed(0)}%)
									</span>
								</div>
								<div class="flex justify-between items-center">
									<span class="text-sm text-neutral-500">Within 7 days:</span>
									<span class="text-base font-semibold text-white">
										{requesterCancellations.within7d} ({requesterCancellations.pct7d.toFixed(0)}%)
									</span>
								</div>
							</div>
						</div>
					{/if}
					{#if providerCancellations && providerCancellations.total > 0}
						<div>
							<h3 class="text-sm font-medium text-neutral-400 mb-3">As Provider</h3>
							<div class="space-y-2">
								<div class="flex justify-between items-center">
									<span class="text-sm text-neutral-500">Total cancelled:</span>
									<span class="text-base font-semibold text-white">{providerCancellations.total}</span>
								</div>
								<div class="flex justify-between items-center">
									<span class="text-sm text-neutral-500">Within 1 hour:</span>
									<span class="text-base font-semibold {providerCancellations.pct1h > 50 ? 'text-danger' : 'text-white'}">
										{providerCancellations.within1h} ({providerCancellations.pct1h.toFixed(0)}%)
									</span>
								</div>
								<div class="flex justify-between items-center">
									<span class="text-sm text-neutral-500">Within 24 hours:</span>
									<span class="text-base font-semibold {providerCancellations.pct24h > 80 ? 'text-warning' : 'text-white'}">
										{providerCancellations.within24h} ({providerCancellations.pct24h.toFixed(0)}%)
									</span>
								</div>
								<div class="flex justify-between items-center">
									<span class="text-sm text-neutral-500">Within 7 days:</span>
									<span class="text-base font-semibold text-white">
										{providerCancellations.within7d} ({providerCancellations.pct7d.toFixed(0)}%)
									</span>
								</div>
							</div>
						</div>
					{/if}
				</div>
			</div>
		{/if}

		<!-- Transaction Statistics -->
		{#if txStats}
			<div class="card p-5">
				<h2 class="text-lg font-semibold text-white mb-4">Transaction Statistics</h2>
				<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
					<div>
						<div class="data-label mb-1">Total Spent</div>
						<div class="text-xl font-semibold text-danger">{formatBalance(txStats.totalSent, 'dct')} DC</div>
					</div>
					<div>
						<div class="data-label mb-1">Total Received</div>
						<div class="text-xl font-semibold text-success">{formatBalance(txStats.totalReceived, 'dct')} DC</div>
					</div>
					<div>
						<div class="data-label mb-1">Total Transactions</div>
						<div class="text-xl font-semibold text-white">{transfers.length}</div>
					</div>
				</div>
			</div>
		{/if}

		<!-- Recent Transactions -->
		{#if transfers.length > 0}
			<div class="card p-5">
				<h2 class="text-lg font-semibold text-white mb-4">Recent Transactions</h2>
				<div class="space-y-3 max-h-96 overflow-y-auto">
					{#each transfers.slice(0, 20) as transfer}
						<div class="bg-surface-elevated p-4 border border-neutral-800">
							<div class="flex justify-between items-start mb-2">
								<div class="flex-1">
									<div class="flex items-center gap-2 mb-1">
										<span class="text-neutral-500 text-sm">From:</span>
										<a
											href="/dashboard/reputation/{transfer.from_account}"
											class="font-mono text-sm text-primary-400 hover:text-primary-300"
										>
											{truncatePubkey(transfer.from_account)}
										</a>
									</div>
									<div class="flex items-center gap-2">
										<span class="text-neutral-500 text-sm">To:</span>
										<a
											href="/dashboard/reputation/{transfer.to_account}"
											class="font-mono text-sm text-primary-400 hover:text-primary-300"
										>
											{truncatePubkey(transfer.to_account)}
										</a>
									</div>
								</div>
								<div class="text-right">
									<div class="text-lg font-semibold text-white">{formatBalance(transfer.amount_e9s, 'dct')} DC</div>
									<div class="text-xs text-neutral-600">Fee: {formatBalance(transfer.fee_e9s, 'dct')} DC</div>
								</div>
							</div>
							<div class="text-xs text-neutral-600">{formatTimestamp(transfer.created_at_ns)}</div>
							{#if transfer.memo}
								<div class="mt-2 text-sm text-neutral-400 italic">{transfer.memo}</div>
							{/if}
						</div>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Offerings Provided -->
		{#if activity && activity.offerings_provided.length > 0}
			<div class="card p-5">
				<h2 class="text-lg font-semibold text-white mb-4">
					Offerings Provided ({activity.offerings_provided.length})
				</h2>
				<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
					{#each activity.offerings_provided as offering}
						<div class="bg-surface-elevated p-4 border border-neutral-800">
							<h3 class="text-base font-semibold text-white mb-2">{offering.offer_name}</h3>
							<p class="text-sm text-neutral-500 mb-2">{offering.product_type}</p>
							<p class="text-white font-medium">{offering.monthly_price.toFixed(2)} {offering.currency}/mo</p>
							<span class="badge mt-2 {offering.stock_status === 'in_stock' ? 'badge-success' : 'badge-danger'}">
								{offering.stock_status}
							</span>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Rentals as Requester -->
		{#if activity && activity.rentals_as_requester.length > 0}
			<div class="card p-5">
				<h2 class="text-lg font-semibold text-white mb-4">
					Rentals as Requester ({activity.rentals_as_requester.length})
				</h2>
				<div class="space-y-3">
					{#each activity.rentals_as_requester as contract}
						<div class="bg-surface-elevated p-4 border border-neutral-800">
							<div class="flex justify-between items-start mb-2">
								<div>
									<p class="text-white font-semibold">Offering: {contract.offering_id}</p>
									<p class="text-sm text-neutral-500">
										Provider:
										<a
											href="/dashboard/reputation/{contract.provider_pubkey}"
											class="text-primary-400 hover:text-primary-300"
										>
											{truncatePubkey(contract.provider_pubkey)}
										</a>
									</p>
									<p class="text-sm text-neutral-500">
										Amount: {formatBalance(contract.payment_amount_e9s, contract.currency)} {contract.currency.toUpperCase()}
									</p>
								</div>
								<span class="badge badge-primary">{contract.status}</span>
							</div>
							<p class="text-sm text-neutral-500">Created: {formatContractDate(contract.created_at_ns)}</p>
							{#if contract.duration_hours}
								<p class="text-sm text-neutral-500">Planned: {contract.duration_hours}h</p>
							{/if}
							<p class="text-sm text-neutral-500">
								Actual runtime: {formatDuration(
									calculateActualDuration(
										contract.created_at_ns,
										contract.status,
										contract.status_updated_at_ns,
										contract.provisioning_completed_at_ns,
									),
								)}
							</p>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Rentals as Provider -->
		{#if activity && activity.rentals_as_provider.length > 0}
			<div class="card p-5">
				<h2 class="text-lg font-semibold text-white mb-4">
					Rentals as Provider ({activity.rentals_as_provider.length})
				</h2>
				<div class="space-y-3">
					{#each activity.rentals_as_provider as contract}
						<div class="bg-surface-elevated p-4 border border-neutral-800">
							<div class="flex justify-between items-start mb-2">
								<div>
									<p class="text-white font-semibold">Offering: {contract.offering_id}</p>
									<p class="text-sm text-neutral-500">
										Requester:
										<a
											href="/dashboard/reputation/{contract.requester_pubkey}"
											class="text-primary-400 hover:text-primary-300"
										>
											{truncatePubkey(contract.requester_pubkey)}
										</a>
									</p>
									<p class="text-sm text-neutral-500">
										Amount: {formatBalance(contract.payment_amount_e9s, contract.currency)} {contract.currency.toUpperCase()}
									</p>
								</div>
								<span class="badge badge-neutral">{contract.status}</span>
							</div>
							<p class="text-sm text-neutral-500">Created: {formatContractDate(contract.created_at_ns)}</p>
							{#if contract.duration_hours}
								<p class="text-sm text-neutral-500">Planned: {contract.duration_hours}h</p>
							{/if}
							<p class="text-sm text-neutral-500">
								Actual runtime: {formatDuration(
									calculateActualDuration(
										contract.created_at_ns,
										contract.status,
										contract.status_updated_at_ns,
										contract.provisioning_completed_at_ns,
									),
								)}
							</p>
						</div>
					{/each}
				</div>
			</div>
		{/if}
	{/if}
</div>
