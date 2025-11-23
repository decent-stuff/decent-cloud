<script lang="ts">
	import type { Contract } from "$lib/services/api";
	import { getContractStatusBadge } from "$lib/utils/contract-status";
	import { truncateContractHash } from "$lib/utils/contract-format";

	export let contract: Contract;
	export let note: string = "";
	export let busy: boolean = false;
	export let onNoteChange: (value: string) => void;
	export let onUpdateStatus: (status: string) => void;

	const badge = getContractStatusBadge(contract.status);
	const normalizedStatus = contract.status.toLowerCase();
</script>

<div class="bg-white/10 border border-white/15 rounded-xl p-6 space-y-4">
	<div class="flex items-start justify-between gap-4">
		<div>
			<div class="flex items-center gap-3 mb-1">
				<h3 class="text-xl font-semibold text-white">
					{contract.offering_id}
				</h3>
				<span
					class="inline-flex items-center gap-1 px-3 py-1 rounded-full text-xs font-medium border {badge.class}"
				>
					<span>{badge.icon}</span>
					{badge.text}
				</span>
			</div>
			<p class="text-white/60 text-sm">
				Contract: {truncateContractHash(contract.contractId)}
			</p>
		</div>
		<div class="text-right text-white/80 text-sm">
			Requester: {truncateContractHash(contract.requester_pubkey)}
		</div>
	</div>

	{#if contract.provisioning_instanceDetails}
		<div
			class="bg-emerald-500/10 border border-emerald-500/20 rounded-lg p-3 text-sm text-white whitespace-pre-wrap"
		>
			{contract.provisioning_instanceDetails}
		</div>
	{/if}

	{#if normalizedStatus === "accepted" || normalizedStatus === "provisioning" || normalizedStatus === "provisioned"}
		<div class="space-y-3">
			{#if normalizedStatus === "provisioning"}
				<textarea
					class="w-full bg-white/10 border border-white/20 rounded-lg text-white p-3"
					rows="3"
					placeholder="Include IP, credentials, and instructions"
					value={note}
					oninput={(event) => onNoteChange(event.currentTarget.value)}
				></textarea>
			{/if}
			<div class="flex flex-wrap gap-3">
				{#if normalizedStatus === "accepted"}
					<button
						class="px-4 py-2 rounded-lg bg-blue-500/80 text-white font-semibold disabled:opacity-60"
						onclick={() => onUpdateStatus("provisioning")}
						disabled={busy}
					>
						Start Provisioning
					</button>
				{:else if normalizedStatus === "provisioning"}
					<button
						class="px-4 py-2 rounded-lg bg-emerald-500/80 text-white font-semibold disabled:opacity-60"
						onclick={() => onUpdateStatus("provisioned")}
						disabled={busy}
					>
						Mark Provisioned
					</button>
				{:else if normalizedStatus === "provisioned"}
					<button
						class="px-4 py-2 rounded-lg bg-indigo-500/80 text-white font-semibold disabled:opacity-60"
						onclick={() => onUpdateStatus("active")}
						disabled={busy}
					>
						Mark Active
					</button>
				{/if}
			</div>
		</div>
	{/if}
</div>
