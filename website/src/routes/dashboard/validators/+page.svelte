<script lang="ts">
	import { onMount } from 'svelte';
	import { getActiveProviders, type ProviderProfile } from '$lib/services/api';

	let validators = $state<ProviderProfile[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	onMount(async () => {
		try {
			loading = true;
			error = null;
			// Get providers active in the last 24 hours (validators)
			validators = await getActiveProviders(1);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load validators';
			console.error('Error loading validators:', e);
		} finally {
			loading = false;
		}
	});

	function formatPubkeyHash(hash: string): string {
		return `${hash.substring(0, 8)}...${hash.substring(hash.length - 8)}`;
	}
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">Validators</h1>
		<p class="text-white/60">Network validators and their performance</p>
	</div>

	{#if error}
		<div class="bg-red-500/20 border border-red-500/30 rounded-lg p-4 text-red-400">
			<p class="font-semibold">Error loading validators</p>
			<p class="text-sm mt-1">{error}</p>
		</div>
	{/if}

	{#if loading}
		<div class="flex justify-center items-center p-8">
			<div class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-400"></div>
		</div>
	{:else}
		<!-- Stats Summary -->
		<div class="grid grid-cols-1 md:grid-cols-3 gap-6">
			<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
				<div class="flex items-center justify-between mb-2">
					<h3 class="text-white/70 text-sm font-medium">Active Validators</h3>
					<span class="text-2xl">✓</span>
				</div>
				<p class="text-3xl font-bold text-white">{validators.length}</p>
			</div>

			<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
				<div class="flex items-center justify-between mb-2">
					<h3 class="text-white/70 text-sm font-medium">Total Validators</h3>
					<span class="text-2xl">🔗</span>
				</div>
				<p class="text-3xl font-bold text-white">{validators.length}</p>
			</div>

			<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
				<div class="flex items-center justify-between mb-2">
					<h3 class="text-white/70 text-sm font-medium">Network Status</h3>
					<span class="text-2xl">🌐</span>
				</div>
				<p class="text-3xl font-bold text-white">{validators.length > 0 ? 'Active' : 'Inactive'}</p>
			</div>
		</div>

		<!-- Validators Table -->
		<div class="bg-white/10 backdrop-blur-lg rounded-xl border border-white/20 overflow-hidden">
			<div class="overflow-x-auto">
				<table class="w-full">
					<thead class="bg-white/5 border-b border-white/10">
						<tr>
							<th class="px-6 py-4 text-left text-sm font-semibold text-white">Validator</th>
							<th class="px-6 py-4 text-left text-sm font-semibold text-white">Status</th>
							<th class="px-6 py-4 text-left text-sm font-semibold text-white">Details</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-white/10">
						{#if validators.length === 0}
							<tr>
								<td colspan="3" class="px-6 py-8 text-center text-white/60">
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
												{validator.name.charAt(0).toUpperCase()}
											</div>
											<div>
												<p class="text-white font-medium">{validator.name || 'Unnamed Validator'}</p>
												<p class="text-white/50 text-sm font-mono">{formatPubkeyHash(validator.pubkey_hash)}</p>
											</div>
										</div>
									</td>
									<td class="px-6 py-4">
										<span
											class="inline-flex items-center gap-1 px-3 py-1 rounded-full text-sm font-medium bg-green-500/20 text-green-400 border border-green-500/30"
										>
											<span class="w-2 h-2 rounded-full bg-current"></span>
											active
										</span>
									</td>
									<td class="px-6 py-4">
										<div class="text-sm text-white/70">
											{#if validator.description}
												<p class="truncate max-w-md">{validator.description}</p>
											{/if}
											{#if validator.website_url}
												<a
													href={validator.website_url}
													target="_blank"
													rel="noopener noreferrer"
													class="text-blue-400 hover:text-blue-300 mt-1 block"
												>
													Visit Website →
												</a>
											{/if}
										</div>
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
			<h2 class="text-2xl font-bold text-white mb-2">Become a Validator</h2>
			<p class="text-white/70 mb-4">
				Help secure the network and earn rewards by becoming a validator
			</p>
			<button
				class="px-6 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold hover:brightness-110 hover:scale-105 transition-all"
			>
				Learn More →
			</button>
		</div>
	{/if}
</div>
