<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { authStore } from "$lib/stores/auth";
	import type { IdentityInfo } from "$lib/stores/auth";
	import {
		getFailedEmails,
		getEmailStats,
		resetEmail,
		retryAllFailed,
		type EmailQueueEntry,
		type EmailStats,
	} from "$lib/services/admin-api";

	let currentIdentity = $state<IdentityInfo | null>(null);
	let unsubscribe: (() => void) | null = null;

	let stats = $state<EmailStats | null>(null);
	let failedEmails = $state<EmailQueueEntry[]>([]);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let retryingEmailId = $state<string | null>(null);
	let retryingAll = $state(false);

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
			const [statsData, emailsData] = await Promise.all([
				getEmailStats(currentIdentity.identity),
				getFailedEmails(currentIdentity.identity, 50),
			]);
			stats = statsData;
			failedEmails = emailsData;
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
