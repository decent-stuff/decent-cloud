<script lang="ts">
	import { onMount } from "svelte";
	import { getActiveValidators, type Validator } from "$lib/services/api";
	import Icon from "$lib/components/Icons.svelte";

	let validators = $state<Validator[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	onMount(async () => {
		try {
			loading = true;
			error = null;
			validators = await getActiveValidators(30);
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load validators";
			console.error("Error loading validators:", e);
		} finally {
			loading = false;
		}
	});

	function formatPubkey(hash: string | number[]): string {
		const hashStr = typeof hash === "string" ? hash : hash.join("");
		return `${hashStr.substring(0, 8)}...${hashStr.substring(hashStr.length - 8)}`;
	}

	function getPubkeyString(hash: string | number[]): string {
		return typeof hash === "string" ? hash : hash.join("");
	}

	function formatTimestamp(timestampNs: number): string {
		const date = new Date(timestampNs / 1_000_000);
		const now = new Date();
		const diffMs = now.getTime() - date.getTime();
		const diffMins = Math.floor(diffMs / 60000);
		const diffHours = Math.floor(diffMs / 3600000);
		const diffDays = Math.floor(diffMs / 86400000);

		if (diffMins < 60) return `${diffMins}m ago`;
		if (diffHours < 24) return `${diffHours}h ago`;
		return `${diffDays}d ago`;
	}

	const activeIn24h = $derived(validators.filter((v) => v.checkIns24h > 0).length);
	const activeIn7d = $derived(validators.filter((v) => v.checkIns7d > 0).length);
</script>

<div class="space-y-6">
	<div>
		<h1 class="text-2xl font-bold text-white tracking-tight">Validators</h1>
		<p class="text-neutral-500 text-sm mt-1">Network validators and their performance</p>
	</div>

	{#if error}
		<div class="bg-danger/10 border border-danger/20 p-4">
			<p class="font-semibold text-danger text-sm">Error loading validators</p>
			<p class="text-xs text-neutral-400 mt-1">{error}</p>
		</div>
	{/if}

	{#if loading}
		<div class="flex justify-center items-center p-8">
			<div class="w-8 h-8 border-2 border-primary-500/30 border-t-primary-500 animate-spin"></div>
		</div>
	{:else}
		<!-- Stats Summary -->
		<div class="grid grid-cols-1 md:grid-cols-3 gap-3">
			<div class="metric-card">
				<div class="flex items-center gap-2 mb-3">
					<Icon name="check" size={20} class="text-success" />
					<span class="metric-label mb-0">Active (24h)</span>
				</div>
				<div class="metric-value">{activeIn24h}</div>
				<div class="metric-subtext">Validators checked in today</div>
			</div>

			<div class="metric-card">
				<div class="flex items-center gap-2 mb-3">
					<Icon name="activity" size={20} class="text-neutral-600" />
					<span class="metric-label mb-0">Active (7d)</span>
				</div>
				<div class="metric-value">{activeIn7d}</div>
				<div class="metric-subtext">Validators active this week</div>
			</div>

			<div class="metric-card">
				<div class="flex items-center gap-2 mb-3">
					<Icon name="globe" size={20} class="text-neutral-600" />
					<span class="metric-label mb-0">Total (30d)</span>
				</div>
				<div class="metric-value">{validators.length}</div>
				<div class="metric-subtext">Validators active this month</div>
			</div>
		</div>

		<!-- Validators Table -->
		<div class="card overflow-hidden">
			<div class="overflow-x-auto">
				<table class="w-full">
					<thead class="bg-surface-elevated border-b border-neutral-800">
						<tr>
							<th class="px-5 py-3 text-left text-[10px] font-semibold text-neutral-500 uppercase tracking-label">Validator</th>
							<th class="px-5 py-3 text-left text-[10px] font-semibold text-neutral-500 uppercase tracking-label">Check-ins</th>
							<th class="px-5 py-3 text-left text-[10px] font-semibold text-neutral-500 uppercase tracking-label">Last Seen</th>
							<th class="px-5 py-3 text-left text-[10px] font-semibold text-neutral-500 uppercase tracking-label">Status</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-neutral-800/60">
						{#if validators.length === 0}
							<tr>
								<td colspan="4" class="px-5 py-8 text-center text-neutral-500">
									No active validators found
								</td>
							</tr>
						{:else}
							{#each validators as validator}
								<tr class="hover:bg-surface-hover transition-colors">
									<td class="px-5 py-4">
										<div class="flex items-center gap-3">
											<div class="w-9 h-9 bg-primary-500/10 border border-primary-500/20 flex items-center justify-center text-primary-400 font-semibold text-sm">
												{validator.name ? validator.name.charAt(0).toUpperCase() : "V"}
											</div>
											<div>
												<p class="text-white font-medium text-sm">
													{validator.name || "Unnamed Validator"}
												</p>
												<a
													href="/dashboard/reputation/{getPubkeyString(validator.pubkey)}"
													class="text-neutral-600 hover:text-primary-400 text-xs font-mono transition-colors"
												>
													{formatPubkey(validator.pubkey)}
												</a>
												{#if validator.description}
													<p class="text-neutral-600 text-xs mt-0.5 max-w-xs truncate">
														{validator.description}
													</p>
												{/if}
											</div>
										</div>
									</td>
									<td class="px-5 py-4">
										<div class="text-sm">
											<div class="text-white font-medium">{validator.totalCheckIns} total</div>
											<div class="text-neutral-600 space-x-2 mt-1 text-xs">
												<span>24h: {validator.checkIns24h}</span>
												<span>7d: {validator.checkIns7d}</span>
												<span>30d: {validator.checkIns30d}</span>
											</div>
										</div>
									</td>
									<td class="px-5 py-4">
										<div class="text-sm text-white">{formatTimestamp(validator.lastCheckInNs)}</div>
									</td>
									<td class="px-5 py-4">
										<span class="badge {validator.checkIns24h > 0 ? 'badge-success' : validator.checkIns7d > 0 ? 'badge-warning' : 'badge-neutral'}">
											<span class="w-1.5 h-1.5 bg-current"></span>
											{validator.checkIns24h > 0 ? "active" : validator.checkIns7d > 0 ? "recent" : "idle"}
										</span>
										{#if validator.websiteUrl}
											<a
												href={validator.websiteUrl}
												target="_blank"
												rel="noopener noreferrer"
												class="text-primary-400 hover:text-primary-300 text-xs ml-2 inline-flex items-center gap-1"
											>
												website
												<Icon name="external" size={20} />
											</a>
										{/if}
									</td>
								</tr>
							{/each}
						{/if}
					</tbody>
				</table>
			</div>
		</div>

		<!-- Become a Validator CTA -->
		<div class="card-accent p-6">
			<div class="flex flex-col md:flex-row items-start md:items-center gap-4">
				<div class="icon-box-accent shrink-0">
					<Icon name="shield" size={20} />
				</div>
				<div class="flex-1">
					<h2 class="text-lg font-semibold text-white mb-1">Become a Validator</h2>
					<p class="text-neutral-400 text-sm">
						Help secure the network and earn rewards by becoming a validator
					</p>
				</div>
				<a
					href="https://decent-stuff.github.io/decent-cloud/mining-and-validation.html"
					target="_blank"
					rel="noopener noreferrer"
					class="btn-primary inline-flex items-center gap-2"
				>
					<span>Learn More</span>
					<Icon name="external" size={20} />
				</a>
			</div>
		</div>
	{/if}
</div>
