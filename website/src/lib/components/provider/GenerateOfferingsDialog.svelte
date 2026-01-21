<script lang="ts">
	import type { AgentPoolWithStats } from "$lib/types/generated/AgentPoolWithStats";
	import type {
		OfferingSuggestionsResponse,
		TierPricing,
		GenerateOfferingsRequest,
		GenerateOfferingsResponse
	} from "$lib/services/api";

	interface Props {
		pool: AgentPoolWithStats;
		isOpen: boolean;
		onClose: () => void;
		onLoad: () => Promise<OfferingSuggestionsResponse>;
		onGenerate: (request: GenerateOfferingsRequest) => Promise<GenerateOfferingsResponse>;
	}

	let { pool, isOpen = $bindable(false), onClose, onLoad, onGenerate }: Props = $props();

	// State
	let loading = $state(true);
	let generating = $state(false);
	let error = $state<string | null>(null);
	let suggestions = $state<OfferingSuggestionsResponse | null>(null);
	let selectedTiers = $state<Set<string>>(new Set());
	let pricing = $state<Record<string, { price: string; currency: string }>>({});
	let visibility = $state("public");
	let dryRun = $state(false);
	let result = $state<GenerateOfferingsResponse | null>(null);

	// Load suggestions when dialog opens
	$effect(() => {
		if (isOpen && !suggestions) {
			loadSuggestions();
		}
	});

	async function loadSuggestions() {
		loading = true;
		error = null;
		try {
			suggestions = await onLoad();
			// Pre-select all available tiers
			selectedTiers = new Set(suggestions.suggestedOfferings.map(s => s.tierName));
			// Initialize pricing with empty values
			pricing = {};
			for (const suggestion of suggestions.suggestedOfferings) {
				pricing[suggestion.tierName] = { price: "", currency: "USD" };
			}
		} catch (err) {
			console.error("Failed to load suggestions", err);
			error = err instanceof Error ? err.message : "Failed to load suggestions";
		} finally {
			loading = false;
		}
	}

	function toggleTier(tierName: string) {
		const newSet = new Set(selectedTiers);
		if (newSet.has(tierName)) {
			newSet.delete(tierName);
		} else {
			newSet.add(tierName);
		}
		selectedTiers = newSet;
	}

	async function handleGenerate() {
		generating = true;
		error = null;
		result = null;

		try {
			// Build pricing map with only selected tiers that have valid prices
			const pricingMap: Record<string, TierPricing> = {};
			for (const tierName of selectedTiers) {
				const p = pricing[tierName];
				if (p && p.price) {
					const priceNum = parseFloat(p.price);
					if (!isNaN(priceNum) && priceNum > 0) {
						pricingMap[tierName] = {
							monthlyPrice: priceNum,
							currency: p.currency
						};
					}
				}
			}

			if (Object.keys(pricingMap).length === 0) {
				throw new Error("Please set prices for at least one selected tier");
			}

			const request: GenerateOfferingsRequest = {
				tiers: Array.from(selectedTiers),
				pricing: pricingMap,
				visibility,
				dryRun
			};

			result = await onGenerate(request);

			if (!dryRun && result.createdOfferings.length > 0) {
				// Success! Clear form for next use
				setTimeout(() => {
					onClose();
					// Reset state
					suggestions = null;
					result = null;
					selectedTiers = new Set();
					pricing = {};
				}, 2000);
			}
		} catch (err) {
			console.error("Failed to generate offerings", err);
			error = err instanceof Error ? err.message : "Failed to generate offerings";
		} finally {
			generating = false;
		}
	}

	function formatBytes(mb: number): string {
		if (mb >= 1024) {
			return `${(mb / 1024).toFixed(1)} GB`;
		}
		return `${mb} MB`;
	}

	function stopPropagation(e: Event) {
		e.stopPropagation();
	}
</script>

{#if isOpen}
	<div
		class="fixed inset-0 bg-base/80 backdrop-blur-sm z-50 flex items-center justify-center"
		onclick={onClose}
		onkeydown={(e) => e.key === 'Escape' && onClose()}
		role="dialog"
		aria-modal="true"
		tabindex="-1"
	>
		<div
			class="bg-surface border border-neutral-800 shadow-lg w-full max-w-3xl m-4 text-white"
			onclick={stopPropagation}
			onkeydown={stopPropagation}
			role="dialog"
			aria-labelledby="dialog-title"
			aria-describedby="dialog-description"
			tabindex="-1"
		>
			<header class="p-6 border-b border-neutral-800">
				<h2 id="dialog-title" class="text-2xl font-bold">Generate Offerings for {pool.name}</h2>
				<p id="dialog-description" class="text-sm text-neutral-500 mt-1">
					Auto-generate VPS offerings based on pool hardware capabilities
				</p>
			</header>

			<div class="p-6 space-y-6 max-h-[70vh] overflow-y-auto">
				{#if loading}
					<div class="flex items-center justify-center py-12">
						<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-400"></div>
						<span class="ml-3 text-neutral-400">Loading pool capabilities...</span>
					</div>
				{:else if error && !suggestions}
					<div class="text-center py-12">
						<div class="text-red-400 mb-4">{error}</div>
						<button
							onclick={loadSuggestions}
							class="px-4 py-2 bg-primary-500/20 text-primary-300 border border-primary-500/30 text-sm hover:bg-primary-500/30"
						>
							Retry
						</button>
					</div>
				{:else if suggestions}
					<!-- Pool Capabilities Summary -->
					<div class="bg-surface-elevated p-4 space-y-2">
						<div class="text-sm font-medium text-neutral-300 mb-2">Pool Capabilities</div>
						<div class="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
							<div>
								<div class="text-neutral-500">Online Agents</div>
								<div class="font-mono text-emerald-400">{suggestions.poolCapabilities.onlineAgents}</div>
							</div>
							<div>
								<div class="text-neutral-500">Total CPU</div>
								<div class="font-mono text-primary-300">{suggestions.poolCapabilities.totalCpuCores} cores</div>
							</div>
							<div>
								<div class="text-neutral-500">Total Memory</div>
								<div class="font-mono text-primary-300">{formatBytes(suggestions.poolCapabilities.totalMemoryMb)}</div>
							</div>
							<div>
								<div class="text-neutral-500">Total Storage</div>
								<div class="font-mono text-primary-300">{suggestions.poolCapabilities.totalStorageGb} GB</div>
							</div>
						</div>
						{#if suggestions.poolCapabilities.cpuModels.length > 0}
							<div class="text-xs text-neutral-500 mt-2">
								CPU: {suggestions.poolCapabilities.cpuModels.join(", ")}
							</div>
						{/if}
						{#if suggestions.poolCapabilities.hasGpu}
							<div class="text-xs text-emerald-400 mt-1">
								GPU: {suggestions.poolCapabilities.gpuModels.join(", ")}
							</div>
						{/if}
					</div>

					<!-- Available Tiers -->
					{#if suggestions.suggestedOfferings.length > 0}
						<div>
							<div class="text-sm font-medium text-neutral-300 mb-3">Available Tiers</div>
							<div class="space-y-2">
								{#each suggestions.suggestedOfferings as suggestion}
									<div class="flex items-center gap-4 p-3 bg-surface-elevated {selectedTiers.has(suggestion.tierName) ? 'border border-primary-500/50' : 'border border-transparent'}">
										<label class="flex items-center gap-3 flex-1 cursor-pointer">
											<input
												type="checkbox"
												checked={selectedTiers.has(suggestion.tierName)}
												onchange={() => toggleTier(suggestion.tierName)}
												class="w-4 h-4 rounded bg-surface-elevated border-neutral-700 text-primary-500 focus:ring-primary-500"
											/>
											<div class="flex-1">
												<div class="font-medium">{suggestion.offerName}</div>
												<div class="text-xs text-neutral-500">
													{suggestion.processorCores} core{suggestion.processorCores > 1 ? 's' : ''} &bull;
													{suggestion.memoryAmount} RAM &bull;
													{suggestion.totalSsdCapacity} SSD
													{#if suggestion.gpuCount}
														&bull; {suggestion.gpuCount}x GPU
													{/if}
												</div>
											</div>
										</label>
										<div class="flex items-center gap-2">
											<input
												type="number"
												step="0.01"
												min="0"
												placeholder="Price"
												bind:value={pricing[suggestion.tierName].price}
												disabled={!selectedTiers.has(suggestion.tierName)}
												class="w-24 px-2 py-1 bg-black/30 border border-neutral-700 text-sm text-right disabled:opacity-50"
											/>
											<select
												bind:value={pricing[suggestion.tierName].currency}
												disabled={!selectedTiers.has(suggestion.tierName)}
												class="px-2 py-1 bg-black/30 border border-neutral-700 text-sm disabled:opacity-50"
											>
												<option value="USD">USD</option>
												<option value="EUR">EUR</option>
												<option value="GBP">GBP</option>
											</select>
											<span class="text-xs text-neutral-500">/mo</span>
										</div>
									</div>
								{/each}
							</div>
						</div>
					{/if}

					<!-- Unavailable Tiers -->
					{#if suggestions.unavailableTiers.length > 0}
						<div>
							<div class="text-sm font-medium text-neutral-500 mb-2">Unavailable Tiers</div>
							<div class="space-y-1">
								{#each suggestions.unavailableTiers as tier}
									<div class="flex items-center gap-2 text-sm text-neutral-500 p-2 bg-black/20">
										<span class="line-through">{tier.tier}</span>
										<span class="text-xs">- {tier.reason}</span>
									</div>
								{/each}
							</div>
						</div>
					{/if}

					<!-- Options -->
					<div class="flex items-center gap-6">
						<label class="flex items-center gap-2 text-sm">
							<span class="text-neutral-400">Visibility:</span>
							<select
								bind:value={visibility}
								class="px-2 py-1 bg-surface-elevated border border-neutral-700 text-sm"
							>
								<option value="public">Public</option>
								<option value="private">Private</option>
							</select>
						</label>
						<label class="flex items-center gap-2 text-sm cursor-pointer">
							<input
								type="checkbox"
								bind:checked={dryRun}
								class="w-4 h-4 rounded bg-surface-elevated border-neutral-700 text-primary-500"
							/>
							<span class="text-neutral-400">Preview only (don't create)</span>
						</label>
					</div>

					<!-- Result -->
					{#if result}
						<div class="p-4 bg-emerald-500/10 border border-emerald-500/30">
							{#if dryRun}
								<div class="text-emerald-400 font-medium mb-2">Preview - Would create {result.createdOfferings.length} offering(s)</div>
							{:else}
								<div class="text-emerald-400 font-medium mb-2">Successfully created {result.createdOfferings.length} offering(s)!</div>
							{/if}
							{#if result.skippedTiers.length > 0}
								<div class="text-sm text-yellow-400">
									Skipped: {result.skippedTiers.map(t => `${t.tier} (${t.reason})`).join(", ")}
								</div>
							{/if}
						</div>
					{/if}

					<!-- Error -->
					{#if error}
						<div class="p-4 bg-red-500/10 border border-red-500/30 text-red-400">
							{error}
						</div>
					{/if}
				{/if}
			</div>

			<footer class="p-4 bg-surface-elevated flex justify-between">
				<button
					onclick={onClose}
					class="px-6 py-2 text-neutral-300 hover:text-white hover:bg-surface-elevated transition-colors font-medium"
				>
					Cancel
				</button>
				{#if suggestions && suggestions.suggestedOfferings.length > 0}
					<button
						onclick={handleGenerate}
						disabled={generating || selectedTiers.size === 0}
						class="px-6 py-2 bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 font-semibold hover:bg-emerald-500/30 transition-colors disabled:opacity-50"
					>
						{#if generating}
							Generating...
						{:else if dryRun}
							Preview
						{:else}
							Generate Offerings
						{/if}
					</button>
				{/if}
			</footer>
		</div>
	</div>
{/if}
