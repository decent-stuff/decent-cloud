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
		type EmailQueueEntry,
		type EmailStats,
		type AdminAccountInfo,
		type AccountDeletionSummary,
		type AdminAccountListResponse,
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

	// Account lookup state
	let lookupUsername = $state("");
	let lookingUpAccount = $state(false);
	let accountInfo = $state<AdminAccountInfo | null>(null);
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

	const isAdmin = $derived(currentIdentity?.account?.isAdmin ?? false);

	onMount(() => {
		unsubscribe = authStore.currentIdentity.subscribe((value) => {
			currentIdentity = value;
			if (value?.account?.isAdmin) {
				loadData();
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

	async function handleLookupAccount() {
		if (!currentIdentity?.identity || !lookupUsername.trim()) return;

		lookingUpAccount = true;
		accountInfo = null;
		accountError = null;

		try {
			accountInfo = await getAccount(currentIdentity.identity, lookupUsername.trim());
		} catch (err) {
			accountError = err instanceof Error ? err.message : "Failed to lookup account";
		} finally {
			lookingUpAccount = false;
		}
	}

	async function handleToggleEmailVerified() {
		if (!currentIdentity?.identity || !accountInfo) return;

		updatingEmailVerified = true;

		try {
			await setEmailVerified(
				currentIdentity.identity,
				accountInfo.username,
				!accountInfo.emailVerified
			);
			// Refresh account info
			accountInfo = await getAccount(currentIdentity.identity, accountInfo.username);
		} catch (err) {
			accountError = err instanceof Error ? err.message : "Failed to update email verification";
		} finally {
			updatingEmailVerified = false;
		}
	}

	function startEditingEmail() {
		editingEmail = true;
		newEmail = accountInfo?.email || "";
	}

	function cancelEditingEmail() {
		editingEmail = false;
		newEmail = "";
	}

	async function handleUpdateEmail() {
		if (!currentIdentity?.identity || !accountInfo) return;

		updatingEmail = true;
		accountError = null;

		try {
			const emailToSet = newEmail.trim() || null;
			await setAccountEmail(currentIdentity.identity, accountInfo.username, emailToSet);
			// Refresh account info
			accountInfo = await getAccount(currentIdentity.identity, accountInfo.username);
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
		if (!currentIdentity?.identity || !accountInfo) return;
		if (deleteConfirmUsername !== accountInfo.username) return;

		deletingAccount = true;
		accountError = null;

		try {
			const result = await deleteAccount(currentIdentity.identity, accountInfo.username);
			deletionResult = result;
			accountInfo = null;
			showDeleteConfirm = false;
			// Refresh accounts list if loaded
			if (accountsList) {
				await loadAccounts();
			}
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

	async function handleToggleAdmin(account: AdminAccountInfo) {
		if (!currentIdentity?.identity) return;

		togglingAdminFor = account.username;
		accountsError = null;

		try {
			await setAdminStatus(currentIdentity.identity, account.username, !account.isAdmin);
			// Refresh the list
			await loadAccounts();
		} catch (err) {
			accountsError = err instanceof Error ? err.message : "Failed to update admin status";
		} finally {
			togglingAdminFor = null;
		}
	}

	function goToAccountsPage(page: number) {
		accountsPage = page;
		loadAccounts();
	}
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">Admin Dashboard</h1>
		<p class="text-white/60">
			Manage email queue and system administration
		</p>
	</div>

	{#if !isAdmin}
		<div class="bg-red-500/20 backdrop-blur-lg rounded-xl p-8 border border-red-500/30 text-center">
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">🚫</span>
				<h2 class="text-2xl font-bold text-white">Access Denied</h2>
				<p class="text-white/70">
					You do not have admin privileges to access this page.
				</p>
			</div>
		</div>
	{:else}
		{#if error}
			<div class="bg-red-500/20 backdrop-blur-lg rounded-xl p-4 border border-red-500/30">
				<p class="text-red-200">Error: {error}</p>
			</div>
		{/if}

		{#if loading && !stats}
			<div class="text-white/60 text-center py-8">Loading...</div>
		{:else}
			<!-- Email Queue Stats -->
			{#if stats}
				<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
					<h2 class="text-2xl font-bold text-white mb-4">
						Email Queue Statistics
					</h2>
					<div class="grid grid-cols-2 md:grid-cols-4 gap-4">
						<div class="bg-white/5 rounded-lg p-4">
							<p class="text-white/70 text-sm">Total</p>
							<p class="text-white font-bold text-2xl">
								{stats.total}
							</p>
						</div>
						<div class="bg-white/5 rounded-lg p-4">
							<p class="text-white/70 text-sm">Pending</p>
							<p class="text-yellow-400 font-bold text-2xl">
								{stats.pending}
							</p>
						</div>
						<div class="bg-white/5 rounded-lg p-4">
							<p class="text-white/70 text-sm">Sent</p>
							<p class="text-green-400 font-bold text-2xl">
								{stats.sent}
							</p>
						</div>
						<div class="bg-white/5 rounded-lg p-4">
							<p class="text-white/70 text-sm">Failed</p>
							<p class="text-red-400 font-bold text-2xl">
								{stats.failed}
							</p>
						</div>
					</div>
				</div>
			{/if}

			<!-- Test Email -->
			<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
				<h2 class="text-2xl font-bold text-white mb-4">Send Test Email</h2>
				<p class="text-white/60 mb-4">
					Test your email configuration by sending a test email.
				</p>
				<form onsubmit={(e) => { e.preventDefault(); handleSendTestEmail(); }} class="flex gap-4">
					<input
						type="email"
						bind:value={testEmailAddress}
						placeholder="recipient@example.com"
						class="flex-1 px-4 py-2 bg-white/5 border border-white/20 rounded-lg text-white placeholder-white/40 focus:outline-none focus:border-blue-500"
						required
					/>
					<button
						type="submit"
						disabled={sendingTestEmail || !testEmailAddress.trim()}
						class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
					>
						{sendingTestEmail ? "Sending..." : "Send Test"}
					</button>
				</form>
				{#if testEmailResult}
					<div class="mt-4 p-3 rounded-lg {testEmailResult.success ? 'bg-green-500/20 text-green-200' : 'bg-red-500/20 text-red-200'}">
						{testEmailResult.message}
					</div>
				{/if}
			</div>

			<!-- Accounts List -->
			<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
				<div class="flex items-center justify-between mb-4">
					<h2 class="text-2xl font-bold text-white">All Accounts</h2>
					<button
						type="button"
						onclick={loadAccounts}
						disabled={loadingAccounts}
						class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
					>
						{loadingAccounts ? "Loading..." : accountsList ? "Refresh" : "Load Accounts"}
					</button>
				</div>

				{#if accountsError}
					<div class="p-3 rounded-lg bg-red-500/20 text-red-200 mb-4">
						{accountsError}
					</div>
				{/if}

				{#if accountsList}
					<div class="mb-4 text-white/60 text-sm">
						Showing {accountsList.accounts.length} of {accountsList.total} accounts
					</div>

					<div class="overflow-x-auto">
						<table class="w-full text-left text-white/90">
							<thead class="text-white/70 border-b border-white/20">
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
									<tr class="border-b border-white/10 hover:bg-white/5">
										<td class="py-3 px-2 font-medium">@{account.username}</td>
										<td class="py-3 px-2 text-sm">{account.email || "-"}</td>
										<td class="py-3 px-2">
											<span class="{account.emailVerified ? 'text-green-400' : 'text-white/40'}">
												{account.emailVerified ? "Yes" : "No"}
											</span>
										</td>
										<td class="py-3 px-2">
											<span class="{account.isAdmin ? 'text-yellow-400 font-medium' : 'text-white/60'}">
												{account.isAdmin ? "Admin" : "User"}
											</span>
										</td>
										<td class="py-3 px-2 text-sm">
											{formatTimestamp(account.createdAt)}
										</td>
										<td class="py-3 px-2">
											<button
												type="button"
												onclick={() => handleToggleAdmin(account)}
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
								class="px-3 py-1 bg-white/10 text-white rounded hover:bg-white/20 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
							>
								Previous
							</button>
							<span class="text-white/60 text-sm">
								Page {accountsPage + 1} of {totalPages}
							</span>
							<button
								type="button"
								onclick={() => goToAccountsPage(accountsPage + 1)}
								disabled={accountsPage >= totalPages - 1 || loadingAccounts}
								class="px-3 py-1 bg-white/10 text-white rounded hover:bg-white/20 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
							>
								Next
							</button>
						</div>
					{/if}
				{:else if !loadingAccounts}
					<p class="text-white/60 text-center py-8">
						Click "Load Accounts" to view all accounts
					</p>
				{/if}
			</div>

			<!-- Account Lookup -->
			<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
				<h2 class="text-2xl font-bold text-white mb-4">Account Lookup</h2>
				<p class="text-white/60 mb-4">
					Search for an account by username to view details and manage settings.
				</p>
				<form onsubmit={(e) => { e.preventDefault(); handleLookupAccount(); }} class="flex gap-4 mb-4">
					<input
						type="text"
						bind:value={lookupUsername}
						placeholder="username"
						class="flex-1 px-4 py-2 bg-white/5 border border-white/20 rounded-lg text-white placeholder-white/40 focus:outline-none focus:border-blue-500"
						required
					/>
					<button
						type="submit"
						disabled={lookingUpAccount || !lookupUsername.trim()}
						class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
					>
						{lookingUpAccount ? "Searching..." : "Lookup"}
					</button>
				</form>

				{#if accountError}
					<div class="p-3 rounded-lg bg-red-500/20 text-red-200 mb-4">
						{accountError}
					</div>
				{/if}

				{#if deletionResult}
					<div class="p-4 rounded-lg bg-green-500/20 text-green-200 mb-4">
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

				{#if accountInfo}
					<div class="bg-white/5 rounded-lg p-4 space-y-4">
						<div class="grid grid-cols-2 gap-4">
							<div>
								<p class="text-white/50 text-sm">Username</p>
								<p class="text-white font-medium">@{accountInfo.username}</p>
							</div>
							<div>
								<p class="text-white/50 text-sm">Account ID</p>
								<p class="text-white font-mono text-sm">{accountInfo.id.slice(0, 8)}...{accountInfo.id.slice(-8)}</p>
							</div>
							<div class="col-span-2">
								<p class="text-white/50 text-sm">Email</p>
								{#if editingEmail}
									<div class="flex items-center gap-2 mt-1">
										<input
											type="email"
											bind:value={newEmail}
											placeholder="email@example.com (leave empty to clear)"
											class="flex-1 px-3 py-1 bg-white/5 border border-white/20 rounded text-white placeholder-white/40 focus:outline-none focus:border-blue-500"
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
											class="px-3 py-1 text-sm bg-white/10 text-white rounded hover:bg-white/20 disabled:opacity-50 transition-colors"
										>
											Cancel
										</button>
									</div>
								{:else}
									<div class="flex items-center gap-2">
										<span class="text-white">{accountInfo.email || "Not set"}</span>
										<button
											type="button"
											onclick={startEditingEmail}
											class="px-2 py-1 text-xs bg-white/10 text-white rounded hover:bg-white/20 transition-colors"
										>
											Edit
										</button>
									</div>
								{/if}
							</div>
							<div>
								<p class="text-white/50 text-sm">Email Verified</p>
								<div class="flex items-center gap-2">
									<span class="{accountInfo.emailVerified ? 'text-green-400' : 'text-red-400'}">
										{accountInfo.emailVerified ? "Yes" : "No"}
									</span>
									<button
										type="button"
										onclick={handleToggleEmailVerified}
										disabled={updatingEmailVerified}
										class="px-2 py-1 text-xs bg-white/10 text-white rounded hover:bg-white/20 disabled:opacity-50 transition-colors"
									>
										{updatingEmailVerified ? "..." : accountInfo.emailVerified ? "Unverify" : "Verify"}
									</button>
								</div>
							</div>
							<div>
								<p class="text-white/50 text-sm">Created</p>
								<p class="text-white">{formatTimestamp(accountInfo.createdAt)}</p>
							</div>
							<div>
								<p class="text-white/50 text-sm">Last Login</p>
								<p class="text-white">
									{accountInfo.lastLoginAt ? formatTimestamp(accountInfo.lastLoginAt) : "Never"}
								</p>
							</div>
							<div>
								<p class="text-white/50 text-sm">Active Keys</p>
								<p class="text-white">{accountInfo.activeKeys} / {accountInfo.totalKeys}</p>
							</div>
							<div>
								<p class="text-white/50 text-sm">Admin</p>
								<p class="{accountInfo.isAdmin ? 'text-yellow-400' : 'text-white'}">
									{accountInfo.isAdmin ? "Yes" : "No"}
								</p>
							</div>
						</div>

						<!-- Delete Account Section -->
						{#if !accountInfo.isAdmin}
							<div class="border-t border-white/10 pt-4 mt-4">
								{#if showDeleteConfirm}
									<div class="bg-red-500/10 border border-red-500/30 rounded-lg p-4 space-y-3">
										<p class="text-red-200 font-medium">Delete Account @{accountInfo.username}?</p>
										<p class="text-white/60 text-sm">
											This will permanently delete the account and all associated resources:
											offerings, provider profile, public keys, and email tokens.
											Contracts will be preserved but account references will be nullified.
										</p>
										<p class="text-white/60 text-sm">
											Type <span class="font-mono text-white">{accountInfo.username}</span> to confirm:
										</p>
										<div class="flex items-center gap-2">
											<input
												type="text"
												bind:value={deleteConfirmUsername}
												placeholder="username"
												class="flex-1 px-3 py-1 bg-white/5 border border-red-500/30 rounded text-white placeholder-white/40 focus:outline-none focus:border-red-500"
											/>
											<button
												type="button"
												onclick={handleDeleteAccount}
												disabled={deletingAccount || deleteConfirmUsername !== accountInfo.username}
												class="px-4 py-1 bg-red-600 text-white rounded hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
											>
												{deletingAccount ? "Deleting..." : "Delete"}
											</button>
											<button
												type="button"
												onclick={cancelDeleteAccount}
												disabled={deletingAccount}
												class="px-4 py-1 bg-white/10 text-white rounded hover:bg-white/20 disabled:opacity-50 transition-colors"
											>
												Cancel
											</button>
										</div>
									</div>
								{:else}
									<button
										type="button"
										onclick={showDeleteAccountConfirm}
										class="px-4 py-2 bg-red-600/20 text-red-400 border border-red-500/30 rounded-lg hover:bg-red-600/30 transition-colors"
									>
										Delete Account
									</button>
								{/if}
							</div>
						{/if}
					</div>
				{/if}
			</div>

			<!-- Sent Emails -->
			<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
				<h2 class="text-2xl font-bold text-white mb-4">Sent Emails</h2>

				{#if sentEmails.length === 0}
					<p class="text-white/60 text-center py-8">
						No sent emails
					</p>
				{:else}
					<div class="overflow-x-auto">
						<table class="w-full text-left text-white/90">
							<thead class="text-white/70 border-b border-white/20">
								<tr>
									<th class="pb-3 px-2">To</th>
									<th class="pb-3 px-2">Subject</th>
									<th class="pb-3 px-2">Type</th>
									<th class="pb-3 px-2">Sent</th>
								</tr>
							</thead>
							<tbody>
								{#each sentEmails as email}
									<tr class="border-b border-white/10 hover:bg-white/5">
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
			<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
				<div class="flex items-center justify-between mb-4">
					<h2 class="text-2xl font-bold text-white">Failed Emails</h2>
					{#if failedEmails.length > 0}
						<button
							type="button"
							onclick={handleRetryAll}
							disabled={retryingAll}
							class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
						>
							{retryingAll ? "Retrying..." : "Retry All"}
						</button>
					{/if}
				</div>

				{#if failedEmails.length === 0}
					<p class="text-white/60 text-center py-8">
						No failed emails
					</p>
				{:else}
					<div class="overflow-x-auto">
						<table class="w-full text-left text-white/90">
							<thead class="text-white/70 border-b border-white/20">
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
									<tr class="border-b border-white/10 hover:bg-white/5">
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
										<td class="py-3 px-2 text-xs text-red-400 max-w-xs truncate">
											{email.lastError || "Unknown error"}
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
