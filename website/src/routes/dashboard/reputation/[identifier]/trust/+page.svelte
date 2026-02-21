<script lang="ts">
	import { onMount } from "svelte";
	import { page } from "$app/stores";
	import {
		getProviderTrustMetrics,
		getProviderResponseMetrics,
		getProviderHealthSummary,
		type ProviderTrustMetrics,
		type ProviderResponseMetrics,
		type ProviderHealthSummary,
	} from "$lib/services/api";
	import TrustDashboard from "$lib/components/TrustDashboard.svelte";
	import Icon from "$lib/components/Icons.svelte";
	import { resolveIdentifierToPubkey } from "$lib/utils/identity";

	const identifier = $page.params.identifier ?? "";

	let pubkey = $state<string>("");
	let displayName = $state<string>(identifier);
	let trustMetrics = $state<ProviderTrustMetrics | null>(null);
	let responseMetrics = $state<ProviderResponseMetrics | null>(null);
	let healthSummary = $state<ProviderHealthSummary | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let copied = $state(false);

	async function copyUrl() {
		await navigator.clipboard.writeText(window.location.href);
		copied = true;
		setTimeout(() => { copied = false; }, 2000);
	}

	onMount(async () => {
		try {
			loading = true;
			error = null;

			const resolved = await resolveIdentifierToPubkey(identifier);
			if (!resolved) {
				error = "Account not found";
				loading = false;
				return;
			}

			pubkey = resolved;

			// Resolve display name: prefer username over raw identifier
			const { getAccountByPublicKey, getAccount } = await import('$lib/services/account-api');
			const account = await (
				identifier === resolved
					? getAccountByPublicKey(resolved)
					: getAccount(identifier)
			).catch(() => null);
			if (account?.username) {
				displayName = account.username;
			}

			const [trustData, responseData, healthData] = await Promise.all([
				getProviderTrustMetrics(pubkey).catch(() => null),
				getProviderResponseMetrics(pubkey).catch(() => null),
				getProviderHealthSummary(pubkey, 30).catch(() => null),
			]);

			trustMetrics = trustData;
			responseMetrics = responseData;
			healthSummary = healthData;
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load trust data";
			console.error("Error loading trust data:", e);
		} finally {
			loading = false;
		}
	});
</script>

<div class="space-y-6 max-w-4xl">
	<!-- Breadcrumb -->
	<nav class="text-sm text-neutral-500">
		<a href="/dashboard/reputation" class="hover:text-white transition-colors">Reputation</a>
		<span class="mx-2">/</span>
		<a href="/dashboard/reputation/{identifier}" class="hover:text-white transition-colors">{displayName}</a>
		<span class="mx-2">/</span>
		<span class="text-white">Trust Report</span>
	</nav>

	{#if loading}
		<div class="flex justify-center items-center p-8">
			<div class="w-8 h-8 border-2 border-primary-500/30 border-t-primary-500 animate-spin"></div>
		</div>
	{:else if error}
		<div class="bg-warning/10 border border-warning/20 p-6">
			<div class="text-center">
				<div class="icon-box mx-auto mb-4">
					<Icon name="search" size={20} />
				</div>
				<h2 class="text-lg font-semibold text-white mb-2">Account Not Found</h2>
				<p class="text-neutral-400 text-sm mb-4">
					The identifier <span class="font-mono text-neutral-300">{identifier}</span> is not registered in the system.
				</p>
				<a href="/dashboard/reputation" class="btn-secondary inline-flex items-center gap-2">
					<Icon name="arrow-left" size={20} />
					<span>Back to Reputation</span>
				</a>
			</div>
		</div>
	{:else}
		<!-- Page Header -->
		<div class="card p-5">
			<div class="flex items-start justify-between gap-4">
				<div>
					<h1 class="text-2xl font-bold text-white tracking-tight mb-1">
						Trust Report
					</h1>
					<p class="text-neutral-400 text-sm font-mono">{displayName}</p>
				</div>
				<div class="flex items-center gap-3">
					<button
						type="button"
						onclick={copyUrl}
						class="btn-secondary inline-flex items-center gap-2 text-sm"
					>
						{#if copied}
							<Icon name="check" size={16} />
							<span>Copied!</span>
						{:else}
							<Icon name="copy" size={16} />
							<span>Copy Link</span>
						{/if}
					</button>
					<a
						href="/dashboard/reputation/{identifier}"
						class="text-sm text-primary-400 hover:text-primary-300 transition-colors"
					>
						View Full Profile →
					</a>
				</div>
			</div>
		</div>

		<!-- Trust Dashboard or empty state -->
		{#if trustMetrics}
			<TrustDashboard metrics={trustMetrics} {responseMetrics} {healthSummary} />
		{:else}
			<div class="card p-8 text-center">
				<div class="icon-box mx-auto mb-4">
					<Icon name="shield" size={20} />
				</div>
				<h2 class="text-lg font-semibold text-white mb-2">No Trust Data Available</h2>
				<p class="text-neutral-400 text-sm mb-6">
					This account has not yet provided any services, so no trust metrics have been recorded.
				</p>
				<a href="/dashboard/reputation/{identifier}" class="btn-secondary inline-flex items-center gap-2">
					<Icon name="arrow-left" size={20} />
					<span>View Full Profile</span>
				</a>
			</div>
		{/if}
	{/if}
</div>
