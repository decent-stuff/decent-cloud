<script lang="ts">
	type Agent = {
		label: string;
		status: "Online" | "Offline";
		version: string;
		activeContracts: number;
		lastSeen: string;
	};

	interface Props {
		agents: Agent[];
	}

	let { agents = [] }: Props = $props();

	function handleRevoke(agent: Agent) {
		console.log("Revoke agent:", agent.label);
	}
</script>

<div class="bg-surface-elevated border border-neutral-800  overflow-hidden">
	<h3 class="px-6 py-4 text-lg font-medium text-white border-b border-neutral-800">
		Agents
	</h3>
	<table class="w-full text-sm text-left">
		<thead class="bg-surface-elevated text-xs text-neutral-500 uppercase">
			<tr>
				<th scope="col" class="px-6 py-3">Label</th>
				<th scope="col" class="px-6 py-3">Status</th>
				<th scope="col" class="px-6 py-3">Version</th>
				<th scope="col" class="px-6 py-3">Active</th>
				<th scope="col" class="px-6 py-3">Last Seen</th>
				<th scope="col" class="px-6 py-3 text-right">Actions</th>
			</tr>
		</thead>
		<tbody>
			{#if agents.length === 0}
				<tr>
					<td colspan="6" class="text-center py-8 text-neutral-500">
						No agents in this pool.
					</td>
				</tr>
			{/if}
			{#each agents as agent (agent.label)}
				<tr class="border-b border-neutral-800 last:border-b-0 hover:bg-surface-elevated transition-colors">
					<th scope="row" class="px-6 py-4 font-medium text-white whitespace-nowrap">
						{agent.label}
					</th>
					<td class="px-6 py-4">
						{#if agent.status === "Online"}
							<span class="flex items-center gap-2 text-green-400">
								<span class="h-2 w-2 rounded-full bg-green-400"></span>
								Online
							</span>
						{:else}
							<span class="flex items-center gap-2 text-red-400">
								<span class="h-2 w-2 rounded-full bg-red-400"></span>
								Offline
							</span>
						{/if}
					</td>
					<td class="px-6 py-4 text-neutral-300">{agent.version}</td>
					<td class="px-6 py-4 text-neutral-300">{agent.activeContracts}</td>
					<td class="px-6 py-4 text-neutral-300">{agent.lastSeen}</td>
					<td class="px-6 py-4 text-right">
						<button
							onclick={() => handleRevoke(agent)}
							class="px-3 py-1.5  text-sm font-medium bg-red-500/20 text-red-300 border border-red-500/30 hover:bg-red-500/30 transition-colors"
							title="Revoke Agent Delegation"
						>
							Revoke
						</button>
					</td>
				</tr>
			{/each}
		</tbody>
	</table>
</div>
