<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { authStore } from "$lib/stores/auth";
	import type { IdentityInfo } from "$lib/stores/auth";
	import {
		getSentEmails,
		getFailedEmails,
		getEmailStats,
		resetEmail,
		retryAllFailed,
		sendTestEmail,
		getAccount,
		setEmailVerified,
		setAccountEmail,
		deleteAccount,
		listAccounts,
		setAdminStatus,
		listRefundRequests,
		approveRefundRequest,
		declineRefundRequest,
		type EmailQueueEntry,
		type EmailStats,
		type AdminAccountInfo,
		type AccountDeletionSummary,
		type AdminAccountListResponse,
		type AdminRefundRequestListResponse,
	} from "$lib/services/admin-api";

	let currentIdentity = $state<IdentityInfo | null>(null);
	let unsubscribe: (() => void) | null = null;

	let stats = $state<EmailStats | null>(null);
	let sentEmails = $state<EmailQueueEntry[]>([]);
	let failedEmails = $state<EmailQueueEntry[]>([]);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let retryingEmailId = $state<string | null>(null);
	let retryingAll = $state(false);

	// Test email state
	let testEmailAddress = $state("");
	let sendingTestEmail = $state(false);
	let testEmailResult = $state<{ success: boolean; message: string } | null>(null);

	// Expanded account state (for inline details)
	let expandedAccountUsername = $state<string | null>(null);
	let expandedAccountInfo = $state<AdminAccountInfo | null>(null);
	let loadingAccountDetails = $state(false);
	let accountError = $state<string | null>(null);
	let updatingEmailVerified = $state(false);

	// Email management state
	let editingEmail = $state(false);
	let newEmail = $state("");
	let updatingEmail = $state(false);

	// Account deletion state
	let showDeleteConfirm = $state(false);
	let deleteConfirmUsername = $state("");
	let deletingAccount = $state(false);
	let deletionResult = $state<AccountDeletionSummary | null>(null);

	// Accounts list state
	let accountsList = $state<AdminAccountListResponse | null>(null);
	let loadingAccounts = $state(false);
	let accountsError = $state<string | null>(null);
	let accountsPage = $state(0);
	let togglingAdminFor = $state<string | null>(null);
	const ACCOUNTS_PER_PAGE = 20;

	// Refund requests state
	let refundList = $state<AdminRefundRequestListResponse | null>(null);
	let loadingRefunds = $state(false);
	let refundsError = $state<string | null>(null);
	let refundStatusFilter = $state('pending');
	let refundPage = $state(0);
	let processingRefundId = $state<number | null>(null);
	let reviewTarget = $state<{ id: number; action: 'approve' | 'decline' } | null>(null);
	let reviewNote = $state('');
	const REFUNDS_PER_PAGE = 20;

	const isAdmin = $derived(currentIdentity?.account?.isAdmin ?? false);

	onMount(() => {
		unsubscribe = authStore.currentIdentity.subscribe((value) => {
			currentIdentity = value;
			if (value?.account?.isAdmin) {
				loadData();
				loadAccounts();
				loadRefundRequests();
			}
		});
	});

	onDestroy(() => {
		unsubscribe?.();
	});

	async function loadData() {
		if (!currentIdentity?.identity) return;

		loading = true;
		error = null;

		try {
			const [statsData, sentEmailsData, failedEmailsData] = await Promise.all([
				getEmailStats(currentIdentity.identity),
				getSentEmails(currentIdentity.identity, 50),
				getFailedEmails(currentIdentity.identity, 50),
			]);
			stats = statsData;
			sentEmails = sentEmailsData;
			failedEmails = failedEmailsData;
		} catch (err) {
			error = err instanceof Error ? err.message : "Failed to load data";
			console.error("Failed to load admin data:", err);
		} finally {
			loading = false;
		}
	}

	async function handleRetryEmail(email: EmailQueueEntry) {
		if (!currentIdentity?.identity) return;

		const emailId = computeEmailId(email);
		retryingEmailId = emailId;
		error = null;

		try {
			await resetEmail(currentIdentity.identity, emailId);
			await loadData();
		} catch (err) {
			error = err instanceof Error ? err.message : "Failed to retry email";
			console.error("Failed to retry email:", err);
		} finally {
			retryingEmailId = null;
		}
	}

	async function handleRetryAll() {
		if (!currentIdentity?.identity) return;

		retryingAll = true;
		error = null;

		try {
			await retryAllFailed(currentIdentity.identity);
			await loadData();
		} catch (err) {
			error =
				err instanceof Error ? err.message : "Failed to retry all emails";
			console.error("Failed to retry all emails:", err);
		} finally {
			retryingAll = false;
		}
	}

	function computeEmailId(email: EmailQueueEntry): string {
		const combined = email.toAddr + email.subject + email.createdAt.toString();
		const encoder = new TextEncoder();
		const data = encoder.encode(combined);
		return Array.from(data.slice(0, 16))
			.map((b) => b.toString(16).padStart(2, "0"))
			.join("");
	}

	function formatTimestamp(ts: number): string {
		return new Date(ts * 1000).toLocaleString();
	}

	async function handleSendTestEmail() {
		if (!currentIdentity?.identity || !testEmailAddress.trim()) return;

		sendingTestEmail = true;
		testEmailResult = null;

		try {
			const result = await sendTestEmail(currentIdentity.identity, testEmailAddress.trim());
			testEmailResult = { success: true, message: result };
		} catch (err) {
			testEmailResult = {
				success: false,
				message: err instanceof Error ? err.message : "Failed to send test email",
			};
		} finally {
			sendingTestEmail = false;
		}
	}

	async function toggleAccountDetails(username: string) {
		if (expandedAccountUsername === username) {
			// Collapse
			expandedAccountUsername = null;
			expandedAccountInfo = null;
			editingEmail = false;
			showDeleteConfirm = false;
			deletionResult = null;
			return;
		}

		// Expand and load details
		expandedAccountUsername = username;
		expandedAccountInfo = null;
		accountError = null;
		editingEmail = false;
		showDeleteConfirm = false;
		deletionResult = null;
		loadingAccountDetails = true;

		try {
			expandedAccountInfo = await getAccount(currentIdentity!.identity!, username);
		} catch (err) {
			accountError = err instanceof Error ? err.message : "Failed to load account details";
		} finally {
			loadingAccountDetails = false;
		}
	}

	async function handleToggleEmailVerified() {
		if (!currentIdentity?.identity || !expandedAccountInfo) return;

		updatingEmailVerified = true;

		try {
			await setEmailVerified(
				currentIdentity.identity,
				expandedAccountInfo.username,
				!expandedAccountInfo.emailVerified
			);
			// Refresh account info
			expandedAccountInfo = await getAccount(currentIdentity.identity, expandedAccountInfo.username);
		} catch (err) {
			accountError = err instanceof Error ? err.message : "Failed to update email verification";
		} finally {
			updatingEmailVerified = false;
		}
	}

	function startEditingEmail() {
		editingEmail = true;
		newEmail = expandedAccountInfo?.email || "";
	}

	function cancelEditingEmail() {
		editingEmail = false;
		newEmail = "";
	}

	async function handleUpdateEmail() {
		if (!currentIdentity?.identity || !expandedAccountInfo) return;

		updatingEmail = true;
		accountError = null;

		try {
			const emailToSet = newEmail.trim() || null;
			await setAccountEmail(currentIdentity.identity, expandedAccountInfo.username, emailToSet);
			// Refresh account info
			expandedAccountInfo = await getAccount(currentIdentity.identity, expandedAccountInfo.username);
			editingEmail = false;
			newEmail = "";
		} catch (err) {
			accountError = err instanceof Error ? err.message : "Failed to update email";
		} finally {
			updatingEmail = false;
		}
	}

	function showDeleteAccountConfirm() {
		showDeleteConfirm = true;
		deleteConfirmUsername = "";
		deletionResult = null;
	}

	function cancelDeleteAccount() {
		showDeleteConfirm = false;
		deleteConfirmUsername = "";
	}

	async function handleDeleteAccount() {
		if (!currentIdentity?.identity || !expandedAccountInfo) return;
		if (deleteConfirmUsername !== expandedAccountInfo.username) return;

		deletingAccount = true;
		accountError = null;

		try {
			const result = await deleteAccount(currentIdentity.identity, expandedAccountInfo.username);
			deletionResult = result;
			expandedAccountInfo = null;
			expandedAccountUsername = null;
			showDeleteConfirm = false;
			// Refresh accounts list
			await loadAccounts();
		} catch (err) {
			accountError = err instanceof Error ? err.message : "Failed to delete account";
		} finally {
			deletingAccount = false;
		}
	}

	async function loadAccounts() {
		if (!currentIdentity?.identity) return;

		loadingAccounts = true;
		accountsError = null;

		try {
			accountsList = await listAccounts(
				currentIdentity.identity,
				ACCOUNTS_PER_PAGE,
				accountsPage * ACCOUNTS_PER_PAGE
			);
		} catch (err) {
			accountsError = err instanceof Error ? err.message : "Failed to load accounts";
		} finally {
			loadingAccounts = false;
		}
	}

	async function handleToggleAdmin(account: AdminAccountInfo, event: Event) {
		event.stopPropagation();
		if (!currentIdentity?.identity) return;

		togglingAdminFor = account.username;
		accountsError = null;

		try {
			await setAdminStatus(currentIdentity.identity, account.username, !account.isAdmin);
			// Refresh the list
			await loadAccounts();
			// If this account is expanded, refresh its details too
			if (expandedAccountUsername === account.username) {
				expandedAccountInfo = await getAccount(currentIdentity.identity, account.username);
			}
		} catch (err) {
			accountsError = err instanceof Error ? err.message : "Failed to update admin status";
		} finally {
			togglingAdminFor = null;
		}
	}

	function goToAccountsPage(page: number) {
		accountsPage = page;
		// Collapse any expanded account when changing pages
		expandedAccountUsername = null;
		expandedAccountInfo = null;
		loadAccounts();
	}

	async function loadRefundRequests() {
		if (!currentIdentity?.identity) return;

		loadingRefunds = true;
		refundsError = null;

		try {
			refundList = await listRefundRequests(
				currentIdentity.identity,
				refundStatusFilter,
				REFUNDS_PER_PAGE,
				refundPage * REFUNDS_PER_PAGE
			);
		} catch (err) {
			refundsError = err instanceof Error ? err.message : "Failed to load refund requests";
			console.error("Failed to load refund requests:", err);
		} finally {
			loadingRefunds = false;
		}
	}

	function onRefundStatusFilterChanged() {
		refundPage = 0;
		reviewTarget = null;
		reviewNote = "";
		loadRefundRequests();
	}

	function goToRefundPage(page: number) {
		refundPage = page;
		reviewTarget = null;
		reviewNote = "";
		loadRefundRequests();
	}

	function startReview(id: number, action: "approve" | "decline") {
		reviewTarget = { id, action };
		reviewNote = "";
	}

	function cancelReview() {
		reviewTarget = null;
		reviewNote = "";
	}

	async function confirmReview() {
		if (!currentIdentity?.identity || !reviewTarget) return;

		const { id, action } = reviewTarget;
		processingRefundId = id;
		refundsError = null;

		try {
			const note = reviewNote.trim() || undefined;
			if (action === "approve") {
				await approveRefundRequest(currentIdentity.identity, id, note);
			} else {
				await declineRefundRequest(currentIdentity.identity, id, note);
			}
			reviewTarget = null;
			reviewNote = "";
			await loadRefundRequests();
		} catch (err) {
			refundsError = err instanceof Error ? err.message : `Failed to ${action} refund request`;
			console.error(`Failed to ${action} refund request:`, err);
		} finally {
			processingRefundId = null;
		}
	}

	function formatNanoTs(ns: number): string {
		return new Date(ns / 1_000_000).toLocaleString();
	}

	function formatRefundAmount(e9s: number, currency: string): string {
		const cents = e9s / 10_000_000;
		try {
			return new Intl.NumberFormat("en-US", {
				style: "currency",
				currency: currency.toUpperCase(),
			}).format(cents / 100);
		} catch {
			return `$${(cents / 100).toFixed(2)}`;
		}
	}

	function formatRefundStatus(status: string): string {
		switch (status) {
			case "pending":
				return "Pending";
			case "auto_issued":
				return "Auto-Issued";
			case "approved":
				return "Approved";
			case "declined":
				return "Declined";
			default:
				return status;
		}
	}

	function refundStatusColor(status: string): string {
		switch (status) {
			case "pending":
				return "text-yellow-400";
			case "auto_issued":
				return "text-blue-400";
			case "approved":
				return "text-green-400";
			case "declined":
				return "text-red-400";
			default:
				return "text-neutral-400";
		}
	}
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-2xl font-bold text-white tracking-tight">Admin Dashboard</h1>
		<p class="text-neutral-500">
			Manage email queue and system administration
		</p>
	</div>

	{#if !isAdmin}
		<div class="bg-red-500/20 backdrop-blur-lg  p-8 border border-red-500/30 text-center">
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">🚫</span>
				<h2 class="text-2xl font-bold text-white">Access Denied</h2>
				<p class="text-neutral-400">
					You do not have admin privileges to access this page.
				</p>
			</div>
		</div>
	{:else}
		{#if error}
			<div class="bg-red-500/20 backdrop-blur-lg  p-4 border border-red-500/30">
				<p class="text-red-200">Error: {error}</p>
			</div>
		{/if}

		{#if loading && !stats}
			<div class="text-neutral-500 text-center py-8">Loading...</div>
		{:else}
			<!-- Email Queue Stats -->
			{#if stats}
				<div class="card p-6 border border-neutral-800">
					<h2 class="text-2xl font-bold text-white mb-4">
						Email Queue Statistics
					</h2>
					<div class="grid grid-cols-2 md:grid-cols-4 gap-4">
						<div class="bg-surface-elevated  p-4">
							<p class="text-neutral-400 text-sm">Total</p>
							<p class="text-white font-bold text-2xl">
								{stats.total}
							</p>
						</div>
						<div class="bg-surface-elevated  p-4">
							<p class="text-neutral-400 text-sm">Pending</p>
							<p class="text-yellow-400 font-bold text-2xl">
								{stats.pending}
							</p>
						</div>
						<div class="bg-surface-elevated  p-4">
							<p class="text-neutral-400 text-sm">Sent</p>
							<p class="text-green-400 font-bold text-2xl">
								{stats.sent}
							</p>
						</div>
						<div class="bg-surface-elevated  p-4">
							<p class="text-neutral-400 text-sm">Failed</p>
							<p class="text-red-400 font-bold text-2xl">
								{stats.failed}
							</p>
						</div>
					</div>
				</div>
			{/if}

			<!-- Test Email -->
			<div class="card p-6 border border-neutral-800">
				<h2 class="text-2xl font-bold text-white mb-4">Send Test Email</h2>
				<p class="text-neutral-500 mb-4">
					Test your email configuration by sending a test email.
				</p>
				<form onsubmit={(e) => { e.preventDefault(); handleSendTestEmail(); }} class="flex gap-4">
					<input
						type="email"
						bind:value={testEmailAddress}
						placeholder="recipient@example.com"
						class="flex-1 px-4 py-2 bg-surface-elevated border border-neutral-800  text-white placeholder-white/40 focus:outline-none focus:border-primary-500"
						required
					/>
					<button
						type="submit"
						disabled={sendingTestEmail || !testEmailAddress.trim()}
						class="px-6 py-2 bg-primary-600 text-white  hover:bg-primary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
					>
						{sendingTestEmail ? "Sending..." : "Send Test"}
					</button>
				</form>
				{#if testEmailResult}
					<div class="mt-4 p-3  {testEmailResult.success ? 'bg-green-500/20 text-green-200' : 'bg-red-500/20 text-red-200'}">
						{testEmailResult.message}
					</div>
				{/if}
			</div>

			<!-- Accounts List -->
			<div class="card p-6 border border-neutral-800">
				<h2 class="text-2xl font-bold text-white mb-4">All Accounts</h2>

				{#if accountsError}
					<div class="p-3  bg-red-500/20 text-red-200 mb-4">
						{accountsError}
					</div>
				{/if}

				{#if deletionResult}
					<div class="p-4  bg-green-500/20 text-green-200 mb-4">
						<p class="font-medium mb-2">Account deleted successfully</p>
						<ul class="text-sm space-y-1">
							<li>Offerings deleted: {deletionResult.offeringsDeleted}</li>
							<li>Contracts as requester: {deletionResult.contractsAsRequester} (nullified)</li>
							<li>Contracts as provider: {deletionResult.contractsAsProvider} (nullified)</li>
							<li>Public keys deleted: {deletionResult.publicKeysDeleted}</li>
							<li>Provider profile deleted: {deletionResult.providerProfileDeleted ? "Yes" : "No"}</li>
						</ul>
					</div>
				{/if}

				{#if loadingAccounts && !accountsList}
					<div class="text-neutral-500 text-center py-8">Loading accounts...</div>
				{:else if accountsList}
					<div class="mb-4 text-neutral-500 text-sm">
						Showing {accountsList.accounts.length} of {accountsList.total} accounts
					</div>

					<div class="overflow-x-auto">
						<table class="w-full text-left text-white/90">
							<thead class="text-neutral-400 border-b border-neutral-800">
								<tr>
									<th class="pb-3 px-2">Username</th>
									<th class="pb-3 px-2">Email</th>
									<th class="pb-3 px-2">Verified</th>
									<th class="pb-3 px-2">Role</th>
									<th class="pb-3 px-2">Created</th>
									<th class="pb-3 px-2">Actions</th>
								</tr>
							</thead>
							<tbody>
								{#each accountsList.accounts as account}
									<tr
										class="border-b border-neutral-800 hover:bg-surface-elevated cursor-pointer transition-colors"
										onclick={() => toggleAccountDetails(account.username)}
									>
										<td class="py-3 px-2 font-medium">
											<span class="inline-flex items-center gap-1">
												{#if expandedAccountUsername === account.username}
													<span class="text-neutral-500">▼</span>
												{:else}
													<span class="text-neutral-500">▶</span>
												{/if}
												@{account.username}
											</span>
										</td>
										<td class="py-3 px-2 text-sm">{account.email || "-"}</td>
										<td class="py-3 px-2">
											<span class="{account.emailVerified ? 'text-green-400' : 'text-neutral-600'}">
												{account.emailVerified ? "Yes" : "No"}
											</span>
										</td>
										<td class="py-3 px-2">
											<span class="{account.isAdmin ? 'text-yellow-400 font-medium' : 'text-neutral-500'}">
												{account.isAdmin ? "Admin" : "User"}
											</span>
										</td>
										<td class="py-3 px-2 text-sm">
											{formatTimestamp(account.createdAt)}
										</td>
										<td class="py-3 px-2">
											<button
												type="button"
												onclick={(e) => handleToggleAdmin(account, e)}
												disabled={togglingAdminFor === account.username}
												class="px-3 py-1 text-sm rounded transition-colors {account.isAdmin
													? 'bg-red-600/20 text-red-400 border border-red-500/30 hover:bg-red-600/30'
													: 'bg-yellow-600/20 text-yellow-400 border border-yellow-500/30 hover:bg-yellow-600/30'} disabled:opacity-50 disabled:cursor-not-allowed"
											>
												{#if togglingAdminFor === account.username}
													...
												{:else}
													{account.isAdmin ? "Revoke Admin" : "Make Admin"}
												{/if}
											</button>
										</td>
									</tr>

									<!-- Expanded account details row -->
									{#if expandedAccountUsername === account.username}
										<tr class="bg-surface-elevated">
											<td colspan="6" class="p-4">
												{#if loadingAccountDetails}
													<div class="text-neutral-500 text-center py-4">Loading details...</div>
												{:else if accountError}
													<div class="p-3  bg-red-500/20 text-red-200">
														{accountError}
													</div>
												{:else if expandedAccountInfo}
													<div class="space-y-4">
														<div class="grid grid-cols-2 md:grid-cols-4 gap-4">
															<div>
																<p class="text-neutral-500 text-sm">Account ID</p>
																<p class="text-white font-mono text-sm">{expandedAccountInfo.id.slice(0, 8)}...{expandedAccountInfo.id.slice(-8)}</p>
															</div>
															<div>
																<p class="text-neutral-500 text-sm">Last Login</p>
																<p class="text-white">
																	{expandedAccountInfo.lastLoginAt ? formatTimestamp(expandedAccountInfo.lastLoginAt) : "Never"}
																</p>
															</div>
															<div>
																<p class="text-neutral-500 text-sm">Active Keys</p>
																<p class="text-white">{expandedAccountInfo.activeKeys} / {expandedAccountInfo.totalKeys}</p>
															</div>
															<div>
																<p class="text-neutral-500 text-sm">Admin Status</p>
																<p class="{expandedAccountInfo.isAdmin ? 'text-yellow-400' : 'text-white'}">
																	{expandedAccountInfo.isAdmin ? "Yes" : "No"}
																</p>
															</div>
														</div>

														<!-- Email Management -->
														<div class="border-t border-neutral-800 pt-4">
															<div class="flex items-center gap-4">
																<div class="flex-1">
																	<p class="text-neutral-500 text-sm mb-1">Email</p>
																	{#if editingEmail}
																		<div class="flex items-center gap-2">
																			<input
																				type="email"
																				bind:value={newEmail}
																				placeholder="email@example.com (leave empty to clear)"
																				class="flex-1 px-3 py-1 bg-surface-elevated border border-neutral-800 rounded text-white placeholder-white/40 focus:outline-none focus:border-primary-500"
																			/>
																			<button
																				type="button"
																				onclick={handleUpdateEmail}
																				disabled={updatingEmail}
																				class="px-3 py-1 text-sm bg-green-600 text-white rounded hover:bg-green-700 disabled:opacity-50 transition-colors"
																			>
																				{updatingEmail ? "..." : "Save"}
																			</button>
																			<button
																				type="button"
																				onclick={cancelEditingEmail}
																				disabled={updatingEmail}
																				class="px-3 py-1 text-sm bg-surface-elevated text-white rounded hover:bg-surface-elevated disabled:opacity-50 transition-colors"
																			>
																				Cancel
																			</button>
																		</div>
																	{:else}
																		<div class="flex items-center gap-2">
																			<span class="text-white">{expandedAccountInfo.email || "Not set"}</span>
																			<button
																				type="button"
																				onclick={startEditingEmail}
																				class="px-2 py-1 text-xs bg-surface-elevated text-white rounded hover:bg-surface-elevated transition-colors"
																			>
																				Edit
																			</button>
																		</div>
																	{/if}
																</div>
																<div>
																	<p class="text-neutral-500 text-sm mb-1">Verified</p>
																	<div class="flex items-center gap-2">
																		<span class="{expandedAccountInfo.emailVerified ? 'text-green-400' : 'text-red-400'}">
																			{expandedAccountInfo.emailVerified ? "Yes" : "No"}
																		</span>
																		<button
																			type="button"
																			onclick={handleToggleEmailVerified}
																			disabled={updatingEmailVerified}
																			class="px-2 py-1 text-xs bg-surface-elevated text-white rounded hover:bg-surface-elevated disabled:opacity-50 transition-colors"
																		>
																			{updatingEmailVerified ? "..." : expandedAccountInfo.emailVerified ? "Unverify" : "Verify"}
																		</button>
																	</div>
																</div>
															</div>
														</div>

														<!-- Delete Account Section -->
														{#if !expandedAccountInfo.isAdmin}
															<div class="border-t border-neutral-800 pt-4">
																{#if showDeleteConfirm}
																	<div class="bg-red-500/10 border border-red-500/30  p-4 space-y-3">
																		<p class="text-red-200 font-medium">Delete Account @{expandedAccountInfo.username}?</p>
																		<p class="text-neutral-500 text-sm">
																			This will permanently delete the account and all associated resources:
																			offerings, provider profile, public keys, and email tokens.
																			Contracts will be preserved but account references will be nullified.
																		</p>
																		<p class="text-neutral-500 text-sm">
																			Type <span class="font-mono text-white">{expandedAccountInfo.username}</span> to confirm:
																		</p>
																		<div class="flex items-center gap-2">
																			<input
																				type="text"
																				bind:value={deleteConfirmUsername}
																				placeholder="username"
																				class="flex-1 px-3 py-1 bg-surface-elevated border border-red-500/30 rounded text-white placeholder-white/40 focus:outline-none focus:border-red-500"
																			/>
																			<button
																				type="button"
																				onclick={handleDeleteAccount}
																				disabled={deletingAccount || deleteConfirmUsername !== expandedAccountInfo.username}
																				class="px-4 py-1 bg-red-600 text-white rounded hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
																			>
																				{deletingAccount ? "Deleting..." : "Delete"}
																			</button>
																			<button
																				type="button"
																				onclick={cancelDeleteAccount}
																				disabled={deletingAccount}
																				class="px-4 py-1 bg-surface-elevated text-white rounded hover:bg-surface-elevated disabled:opacity-50 transition-colors"
																			>
																				Cancel
																			</button>
																		</div>
																	</div>
																{:else}
																	<button
																		type="button"
																		onclick={showDeleteAccountConfirm}
																		class="px-4 py-2 bg-red-600/20 text-red-400 border border-red-500/30  hover:bg-red-600/30 transition-colors"
																	>
																		Delete Account
																	</button>
																{/if}
															</div>
														{/if}
													</div>
												{/if}
											</td>
										</tr>
									{/if}
								{/each}
							</tbody>
						</table>
					</div>

					<!-- Pagination -->
					{#if accountsList.total > ACCOUNTS_PER_PAGE}
						{@const totalPages = Math.ceil(accountsList.total / ACCOUNTS_PER_PAGE)}
						<div class="flex items-center justify-center gap-2 mt-4">
							<button
								type="button"
								onclick={() => goToAccountsPage(accountsPage - 1)}
								disabled={accountsPage === 0 || loadingAccounts}
								class="px-3 py-1 bg-surface-elevated text-white rounded hover:bg-surface-elevated disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
							>
								Previous
							</button>
							<span class="text-neutral-500 text-sm">
								Page {accountsPage + 1} of {totalPages}
							</span>
							<button
								type="button"
								onclick={() => goToAccountsPage(accountsPage + 1)}
								disabled={accountsPage >= totalPages - 1 || loadingAccounts}
								class="px-3 py-1 bg-surface-elevated text-white rounded hover:bg-surface-elevated disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
							>
								Next
							</button>
						</div>
					{/if}
				{/if}
			</div>


			<!-- Refund Requests -->
		<div class="card p-6 border border-neutral-800">
			<div class="flex items-center justify-between mb-4 flex-wrap gap-4">
				<h2 class="text-2xl font-bold text-white">Refund Requests</h2>
				<div class="flex items-center gap-2">
					<label for="refund-status-filter" class="text-neutral-400 text-sm">Status:</label>
					<select
						id="refund-status-filter"
						bind:value={refundStatusFilter}
						onchange={onRefundStatusFilterChanged}
						class="px-3 py-1 bg-surface-elevated border border-neutral-800 rounded text-white focus:outline-none focus:border-primary-500"
					>
						<option value="pending">Pending</option>
						<option value="all">All</option>
						<option value="auto_issued">Auto-Issued</option>
						<option value="approved">Approved</option>
						<option value="declined">Declined</option>
					</select>
				</div>
			</div>

			{#if refundsError}
				<div class="p-3 bg-red-500/20 text-red-200 mb-4">
					{refundsError}
				</div>
			{/if}

			{#if loadingRefunds && !refundList}
				<div class="text-neutral-500 text-center py-8">Loading refund requests...</div>
			{:else if refundList}
				<div class="mb-4 text-neutral-500 text-sm">
					Showing {refundList.requests.length} of {refundList.total} refund requests
				</div>

				{#if refundList.requests.length === 0}
					<p class="text-neutral-500 text-center py-8">
						No refund requests
					</p>
				{:else}
					<div class="overflow-x-auto">
						<table class="w-full text-left text-white/90">
							<thead class="text-neutral-400 border-b border-neutral-800">
								<tr>
									<th class="pb-3 px-2">Contract</th>
									<th class="pb-3 px-2">Reason</th>
									<th class="pb-3 px-2">Refund</th>
									<th class="pb-3 px-2">Last Payment</th>
									<th class="pb-3 px-2">Cap</th>
									<th class="pb-3 px-2">Status</th>
									<th class="pb-3 px-2">Created</th>
									<th class="pb-3 px-2">Actions</th>
								</tr>
							</thead>
							<tbody>
								{#each refundList.requests as req (req.id)}
									<tr class="border-b border-neutral-800 hover:bg-surface-elevated transition-colors">
										<td class="py-3 px-2 font-mono text-xs" title={req.contractId}>
											{req.contractId.slice(0, 10)}…{req.contractId.slice(-6)}
										</td>
										<td class="py-3 px-2 text-sm">{req.reason}</td>
										<td class="py-3 px-2 font-medium">
											{formatRefundAmount(req.refundAmountE9s, req.currency)}
										</td>
										<td class="py-3 px-2 text-sm">
											{formatRefundAmount(req.userLatestPaymentE9s, req.currency)}
										</td>
										<td class="py-3 px-2">
											{#if req.capExceeded}
												<span class="px-2 py-0.5 text-xs bg-red-500/20 text-red-400 border border-red-500/30 rounded">Exceeded</span>
											{:else}
												<span class="text-neutral-600 text-xs">OK</span>
											{/if}
										</td>
										<td class="py-3 px-2">
											<span class="text-sm {refundStatusColor(req.status)}">
												{formatRefundStatus(req.status)}
											</span>
										</td>
										<td class="py-3 px-2 text-sm">
											{formatNanoTs(req.createdAtNs)}
										</td>
										<td class="py-3 px-2">
											{#if req.status === "pending"}
												<div class="flex gap-2">
													<button
														type="button"
														onclick={() => startReview(req.id, "approve")}
														disabled={processingRefundId === req.id}
														class="px-3 py-1 text-sm bg-green-600 text-white rounded hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
													>
														Approve
													</button>
													<button
														type="button"
														onclick={() => startReview(req.id, "decline")}
														disabled={processingRefundId === req.id}
														class="px-3 py-1 text-sm bg-red-600/20 text-red-400 border border-red-500/30 rounded hover:bg-red-600/30 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
													>
														Decline
													</button>
												</div>
											{:else}
												<span class="text-neutral-600 text-xs">—</span>
											{/if}
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>

					<!-- Inline review prompt (note + explicit confirm) -->
					{#if reviewTarget}
						{@const targetReq = refundList.requests.find((r) => r.id === reviewTarget?.id)}
						{#if targetReq}
							<div class="mt-4 bg-surface-elevated border border-neutral-800 p-4 space-y-3">
								<p class="text-white font-medium">
									{reviewTarget.action === "approve" ? "Approve" : "Decline"} refund for
									{formatRefundAmount(targetReq.refundAmountE9s, targetReq.currency)}?
								</p>
								{#if reviewTarget.action === "approve"}
									<p class="text-yellow-300 text-sm">
										⚠ This will issue a refund of
										{formatRefundAmount(targetReq.refundAmountE9s, targetReq.currency)}.
										Are you sure?
									</p>
								{/if}
								<textarea
									bind:value={reviewNote}
									placeholder="Optional note (recorded in audit log)"
									rows="2"
									class="w-full px-3 py-2 bg-surface-elevated border border-neutral-800 rounded text-white placeholder-white/40 focus:outline-none focus:border-primary-500"
								></textarea>
								<div class="flex gap-2">
									<button
										type="button"
										onclick={confirmReview}
										disabled={processingRefundId === reviewTarget.id}
										class="px-4 py-1 rounded text-white disabled:opacity-50 disabled:cursor-not-allowed transition-colors {reviewTarget.action === "approve"
											? "bg-green-600 hover:bg-green-700"
											: "bg-red-600 hover:bg-red-700"}"
									>
										{#if processingRefundId === reviewTarget.id}
											Processing...
										{:else}
											Confirm {reviewTarget.action}
										{/if}
									</button>
									<button
										type="button"
										onclick={cancelReview}
										disabled={processingRefundId === reviewTarget.id}
										class="px-4 py-1 bg-surface-elevated text-white rounded hover:bg-surface-elevated disabled:opacity-50 transition-colors"
									>
										Cancel
									</button>
								</div>
							</div>
						{/if}
					{/if}

					<!-- Pagination -->
					{#if refundList.total > REFUNDS_PER_PAGE}
						{@const totalPages = Math.ceil(refundList.total / REFUNDS_PER_PAGE)}
						<div class="flex items-center justify-center gap-2 mt-4">
							<button
								type="button"
								onclick={() => goToRefundPage(refundPage - 1)}
								disabled={refundPage === 0 || loadingRefunds}
								class="px-3 py-1 bg-surface-elevated text-white rounded hover:bg-surface-elevated disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
							>
								Previous
							</button>
							<span class="text-neutral-500 text-sm">
								Page {refundPage + 1} of {totalPages}
							</span>
							<button
								type="button"
								onclick={() => goToRefundPage(refundPage + 1)}
								disabled={refundPage >= totalPages - 1 || loadingRefunds}
								class="px-3 py-1 bg-surface-elevated text-white rounded hover:bg-surface-elevated disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
							>
								Next
							</button>
						</div>
					{/if}
				{/if}
			{/if}
		</div>

		<!-- Sent Emails -->
			<div class="card p-6 border border-neutral-800">
				<h2 class="text-2xl font-bold text-white mb-4">Sent Emails</h2>

				{#if sentEmails.length === 0}
					<p class="text-neutral-500 text-center py-8">
						No sent emails
					</p>
				{:else}
					<div class="overflow-x-auto">
						<table class="w-full text-left text-white/90">
							<thead class="text-neutral-400 border-b border-neutral-800">
								<tr>
									<th class="pb-3 px-2">To</th>
									<th class="pb-3 px-2">Subject</th>
									<th class="pb-3 px-2">Type</th>
									<th class="pb-3 px-2">Sent</th>
								</tr>
							</thead>
							<tbody>
								{#each sentEmails as email}
									<tr class="border-b border-neutral-800 hover:bg-surface-elevated">
										<td class="py-3 px-2 font-mono text-sm">
											{email.toAddr}
										</td>
										<td class="py-3 px-2">{email.subject}</td>
										<td class="py-3 px-2 text-sm">
											{email.emailType}
										</td>
										<td class="py-3 px-2 text-sm">
											{email.sentAt ? formatTimestamp(email.sentAt) : "N/A"}
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				{/if}
			</div>

			<!-- Failed Emails -->
			<div class="card p-6 border border-neutral-800">
				<div class="flex items-center justify-between mb-4">
					<h2 class="text-2xl font-bold text-white">Failed Emails</h2>
					{#if failedEmails.length > 0}
						<button
							type="button"
							onclick={handleRetryAll}
							disabled={retryingAll}
							class="px-4 py-2 bg-primary-600 text-white  hover:bg-primary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
						>
							{retryingAll ? "Retrying..." : "Retry All"}
						</button>
					{/if}
				</div>

				{#if failedEmails.length === 0}
					<p class="text-neutral-500 text-center py-8">
						No failed emails
					</p>
				{:else}
					<div class="overflow-x-auto">
						<table class="w-full text-left text-white/90">
							<thead class="text-neutral-400 border-b border-neutral-800">
								<tr>
									<th class="pb-3 px-2">To</th>
									<th class="pb-3 px-2">Subject</th>
									<th class="pb-3 px-2">Attempts</th>
									<th class="pb-3 px-2">Created</th>
									<th class="pb-3 px-2">Error</th>
									<th class="pb-3 px-2">Action</th>
								</tr>
							</thead>
							<tbody>
								{#each failedEmails as email}
									{@const emailId = computeEmailId(email)}
									<tr class="border-b border-neutral-800 hover:bg-surface-elevated">
										<td class="py-3 px-2 font-mono text-sm">
											{email.toAddr}
										</td>
										<td class="py-3 px-2">{email.subject}</td>
										<td class="py-3 px-2">
											{email.attempts}/{email.maxAttempts}
										</td>
										<td class="py-3 px-2 text-sm">
											{formatTimestamp(email.createdAt)}
										</td>
									<td class="py-3 px-2 text-xs text-red-400 max-w-xs">
										{#if email.lastError}
											<details>
												<summary class="cursor-pointer whitespace-pre-wrap break-words">{email.lastError}</summary>
												<pre class="mt-1 p-2 bg-base/50 border border-neutral-800 text-xs text-red-300 font-mono overflow-x-auto max-h-40 overflow-y-auto whitespace-pre-wrap">{email.lastError}</pre>
											</details>
										{:else}
											<span title="No error details were recorded">Unknown error</span>
										{/if}
									</td>
										<td class="py-3 px-2">
											<button
												type="button"
												onclick={() => handleRetryEmail(email)}
												disabled={retryingEmailId === emailId}
												class="px-3 py-1 bg-green-600 text-white text-sm rounded hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
											>
												{retryingEmailId === emailId
													? "Retrying..."
													: "Retry"}
											</button>
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				{/if}
			</div>
		{/if}
	{/if}
</div>
