<script lang="ts">
	import { onMount } from "svelte";
	import { getActiveValidators, type Validator } from "$lib/services/api";

	let validators = $state<Validator[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	onMount(async () => {
		try {
			loading = true;
			error = null;
			// Get validators active in the last 30 days
			validators = await getActiveValidators(30);
		} catch (e) {
			error =
				e instanceof Error ? e.message : "Failed to load validators";
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

	// Calculate active validators by different time periods
	const activeIn24h = $derived(
		validators.filter((v) => v.checkIns24h > 0).length,
	);
	const activeIn7d = $derived(
		validators.filter((v) => v.checkIns7d > 0).length,
	);
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">Validators</h1>
		<p class="text-white/60">Network validators and their performance</p>
	</div>

	{#if error}
		<div
			class="bg-red-500/20 border border-red-500/30 rounded-lg p-4 text-red-400"
		>
			<p class="font-semibold">Error loading validators</p>
			<p class="text-sm mt-1">{error}</p>
		</div>
	{/if}

	{#if loading}
		<div class="flex justify-center items-center p-8">
			<div
				class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-400"
			></div>
		</div>
	{:else}
		<!-- Stats Summary -->
		<div class="grid grid-cols-1 md:grid-cols-3 gap-6">
			<div
				class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20"
			>
				<div class="flex items-center justify-between mb-2">
					<h3 class="text-white/70 text-sm font-medium">
						Active (24h)
					</h3>
					<span class="text-2xl">✓</span>
				</div>
				<p class="text-3xl font-bold text-white">{activeIn24h}</p>
				<p class="text-white/50 text-sm mt-1">
					Validators checked in today
				</p>
			</div>

			<div
				class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20"
			>
				<div class="flex items-center justify-between mb-2">
					<h3 class="text-white/70 text-sm font-medium">
						Active (7d)
					</h3>
					<span class="text-2xl">🔗</span>
				</div>
				<p class="text-3xl font-bold text-white">{activeIn7d}</p>
				<p class="text-white/50 text-sm mt-1">
					Validators active this week
				</p>
			</div>

			<div
				class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20"
			>
				<div class="flex items-center justify-between mb-2">
					<h3 class="text-white/70 text-sm font-medium">
						Total (30d)
					</h3>
					<span class="text-2xl">🌐</span>
				</div>
				<p class="text-3xl font-bold text-white">{validators.length}</p>
				<p class="text-white/50 text-sm mt-1">
					Validators active this month
				</p>
			</div>
		</div>

		<!-- Validators Table -->
		<div
			class="bg-white/10 backdrop-blur-lg rounded-xl border border-white/20 overflow-hidden"
		>
			<div class="overflow-x-auto">
				<table class="w-full">
					<thead class="bg-white/5 border-b border-white/10">
						<tr>
							<th
								class="px-6 py-4 text-left text-sm font-semibold text-white"
								>Validator</th
							>
							<th
								class="px-6 py-4 text-left text-sm font-semibold text-white"
								>Check-ins</th
							>
							<th
								class="px-6 py-4 text-left text-sm font-semibold text-white"
								>Last Seen</th
							>
							<th
								class="px-6 py-4 text-left text-sm font-semibold text-white"
								>Status</th
							>
						</tr>
					</thead>
					<tbody class="divide-y divide-white/10">
						{#if validators.length === 0}
							<tr>
								<td
									colspan="4"
									class="px-6 py-8 text-center text-white/60"
								>
									No active validators found
								</td>
							</tr>
						{:else}
							{#each validators as validator}
								<tr class="hover:bg-white/5 transition-colors">
									<td class="px-6 py-4">
										<div class="flex items-center gap-3">
											<div
												class="w-10 h-10 rounded-full bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center text-white font-bold"
											>
												{validator.name
													? validator.name
															.charAt(0)
															.toUpperCase()
													: "V"}
											</div>
											<div>
												<p
													class="text-white font-medium"
												>
													{validator.name ||
														"Unnamed Validator"}
												</p>
												<a
													href="/dashboard/reputation/{getPubkeyString(
														validator.pubkey,
													)}"
													class="text-white/50 hover:text-blue-400 text-sm font-mono transition-colors"
												>
													{formatPubkey(
														validator.pubkey,
													)}
												</a>
												{#if validator.description}
													<p
														class="text-white/50 text-xs mt-0.5 max-w-xs truncate"
													>
														{validator.description}
													</p>
												{/if}
											</div>
										</div>
									</td>
									<td class="px-6 py-4">
										<div class="text-sm">
											<div class="text-white font-medium">
												{validator.totalCheckIns} total
											</div>
											<div
												class="text-white/50 space-x-2 mt-1"
											>
												<span
													>24h: {validator.checkIns24h}</span
												>
												<span
													>7d: {validator.checkIns7d}</span
												>
												<span
													>30d: {validator.checkIns30d}</span
												>
											</div>
										</div>
									</td>
									<td class="px-6 py-4">
										<div class="text-sm text-white">
											{formatTimestamp(
												validator.lastCheckInNs,
											)}
										</div>
									</td>
									<td class="px-6 py-4">
										<span
											class="inline-flex items-center gap-1 px-3 py-1 rounded-full text-sm font-medium {validator.checkIns24h >
											0
												? 'bg-green-500/20 text-green-400 border-green-500/30'
												: validator.checkIns7d > 0
													? 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30'
													: 'bg-gray-500/20 text-gray-400 border-gray-500/30'} border"
										>
											<span
												class="w-2 h-2 rounded-full bg-current"
											></span>
											{validator.checkIns24h > 0
												? "active"
												: validator.checkIns7d > 0
													? "recent"
													: "idle"}
										</span>
										{#if validator.websiteUrl}
											<a
												href={validator.websiteUrl}
												target="_blank"
												rel="noopener noreferrer"
												class="text-blue-400 hover:text-blue-300 text-xs ml-2"
											>
												website →
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
		<div
			class="bg-gradient-to-r from-blue-500/20 to-purple-600/20 rounded-xl p-8 border border-blue-500/30"
		>
			<h2 class="text-2xl font-bold text-white mb-2">
				Become a Validator
			</h2>
			<p class="text-white/70 mb-4">
				Help secure the network and earn rewards by becoming a validator
			</p>
			<a
				href="https://decent-stuff.github.io/decent-cloud/mining-and-validation.html"
				target="_blank"
				rel="noopener noreferrer"
				class="inline-block px-6 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold hover:brightness-110 hover:scale-105 transition-all"
			>
				Learn More →
			</a>
		</div>
	{/if}
</div>
