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
		type EmailQueueEntry,
		type EmailStats,
		type AdminAccountInfo,
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
							<div>
								<p class="text-white/50 text-sm">Email</p>
								<p class="text-white">{accountInfo.email || "Not set"}</p>
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
