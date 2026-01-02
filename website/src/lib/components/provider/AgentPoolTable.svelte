<script lang="ts">
	import { goto } from "$app/navigation";
	import type { AgentPoolWithStats } from "$lib/types/generated/AgentPoolWithStats";

	interface Props {
		pools: AgentPoolWithStats[];
		onEdit?: (pool: AgentPoolWithStats) => void;
	}

	let { pools = [], onEdit }: Props = $props();

	function handleRowClick(pool: AgentPoolWithStats) {
		goto(`/dashboard/provider/agents/${pool.poolId}`);
	}

	function handleEdit(e: Event, pool: AgentPoolWithStats) {
		e.stopPropagation();
		onEdit?.(pool);
	}

	function handleAddAgent(e: Event, pool: AgentPoolWithStats) {
		e.stopPropagation();
		goto(`/dashboard/provider/agents/${pool.poolId}`);
	}
</script>

<div class="bg-glass/5 border border-glass/10 rounded-xl overflow-hidden">
	<table class="w-full text-sm text-left">
		<thead class="bg-glass/5 text-xs text-white/60 uppercase">
			<tr>
				<th scope="col" class="px-6 py-3">Pool</th>
				<th scope="col" class="px-6 py-3">Pool ID</th>
				<th scope="col" class="px-6 py-3">Region</th>
				<th scope="col" class="px-6 py-3">Type</th>
				<th scope="col" class="px-6 py-3">Agents</th>
				<th scope="col" class="px-6 py-3">Online</th>
				<th scope="col" class="px-6 py-3">Active Contracts</th>
				<th scope="col" class="px-6 py-3">Offerings</th>
				<th scope="col" class="px-6 py-3 text-right hidden sm:table-cell">Actions</th>
			</tr>
		</thead>
		<tbody>
			{#if pools.length === 0}
				<tr>
					<td colspan="9" class="text-center py-8 text-white/50">
						No agent pools configured yet.
					</td>
				</tr>
			{/if}
			{#each pools as pool (pool.poolId)}
				<tr
					onclick={() => handleRowClick(pool)}
					onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') handleRowClick(pool); }}
					class="border-b border-glass/10 last:border-b-0 hover:bg-glass/5 transition-colors cursor-pointer"
					role="button"
					tabindex="0"
				>
					<th scope="row" class="px-6 py-4 font-medium text-white whitespace-nowrap">
						<span class="text-primary-400">
							{pool.name}
						</span>
					</th>
					<td class="px-6 py-4 font-mono text-white/60 text-xs">
						{pool.poolId}
					</td>
					<td class="px-6 py-4">
						<span class="px-2 py-0.5 rounded text-xs bg-primary-500/20 text-primary-300 border border-primary-500/30">
							{pool.location}
						</span>
					</td>
					<td class="px-6 py-4">
						<span class="px-2 py-0.5 rounded text-xs bg-purple-500/20 text-primary-300 border border-purple-500/30">
							{pool.provisionerType}
						</span>
					</td>
					<td class="px-6 py-4 text-white/80">{pool.agentCount}</td>
					<td class="px-6 py-4 text-white/80">
						<span class="{pool.onlineCount > 0 ? 'text-green-400' : 'text-red-400'}">
							{pool.onlineCount} / {pool.agentCount}
						</span>
					</td>
					<td class="px-6 py-4 text-white/80">{pool.activeContracts}</td>
					<td class="px-6 py-4 text-white/80">{pool.offeringsCount}</td>
					<td class="px-6 py-4 text-right space-x-2 hidden sm:table-cell">
						<button
							onclick={(e) => handleAddAgent(e, pool)}
							class="px-3 py-1.5 rounded-lg text-sm font-medium bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 hover:bg-emerald-500/30 transition-colors"
							title="Add Agent / Setup Tokens"
						>
							+
						</button>
						<button
							onclick={(e) => handleEdit(e, pool)}
							class="px-3 py-1.5 rounded-lg text-sm font-medium bg-glass/10 text-white/80 hover:bg-glass/15 transition-colors"
							title="Edit Pool"
						>
							Edit
						</button>
					</td>
				</tr>
			{/each}
		</tbody>
	</table>
</div>
