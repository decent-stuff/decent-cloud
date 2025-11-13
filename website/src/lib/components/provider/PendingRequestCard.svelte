<script lang="ts">
	import type { Contract } from "$lib/services/api";
	import { getContractStatusBadge } from "$lib/utils/contract-status";
	import {
		formatContractPrice,
		truncateContractHash,
	} from "$lib/utils/contract-format";

	export let contract: Contract;
	export let memo: string = "";
	export let busy: boolean = false;
	export let onMemoChange: (value: string) => void;
	export let onRespond: (accept: boolean) => void;

	const statusBadge = getContractStatusBadge(contract.status);
	let memoFieldId = "";
	$: memoFieldId = `pending-response-${contract.contract_id}`;
</script>

<div class="bg-white/10 border border-white/15 rounded-xl p-6 space-y-4">
	<div class="flex flex-wrap items-start justify-between gap-4">
		<div>
			<div class="flex items-center gap-3 mb-1">
				<h3 class="text-xl font-semibold text-white">
					{contract.offering_id}
				</h3>
				<span class="inline-flex items-center gap-1 px-3 py-1 rounded-full text-xs font-medium border {statusBadge.class}">
					<span>{statusBadge.icon}</span>
					{statusBadge.text}
				</span>
			</div>
			<a
				href="/dashboard/user/{contract.requester_pubkey_hash}"
				class="text-white/60 text-sm hover:text-blue-400 transition-colors inline-flex items-center gap-1"
			>
				<span class="text-xs">👤</span>
				Requester: {truncateContractHash(contract.requester_pubkey_hash)}
				<span class="text-xs">→</span>
			</a>
		</div>
		<div class="text-right">
			<div class="text-2xl font-bold text-white">
				{formatContractPrice(contract.payment_amount_e9s)}
			</div>
		</div>
	</div>

	<div class="grid md:grid-cols-2 gap-4">
		<div class="bg-white/5 rounded-lg p-3 border border-white/10">
			<div class="text-white/60 text-xs mb-1">SSH Key</div>
			<div class="font-mono text-sm text-white truncate">
				{truncateContractHash(contract.requester_ssh_pubkey)}
			</div>
		</div>
		<div class="bg-white/5 rounded-lg p-3 border border-white/10">
			<div class="text-white/60 text-xs mb-1">Contact</div>
			<div class="text-white text-sm">{contract.requester_contact}</div>
		</div>
	</div>

	{#if contract.request_memo}
		<div class="bg-white/5 rounded-lg p-3 border border-white/10">
			<div class="text-white/60 text-xs mb-1">Request Memo</div>
			<p class="text-white text-sm whitespace-pre-wrap">
				{contract.request_memo}
			</p>
		</div>
	{/if}

	<div class="space-y-3">
		<label class="text-white/80 text-sm font-medium" for={memoFieldId}>
			Optional response memo
		</label>
		<textarea
			class="w-full bg-white/10 border border-white/20 rounded-lg text-white p-3"
			id={memoFieldId}
			rows="2"
			placeholder="Add provisioning notes or reasons"
			value={memo}
			oninput={(event) => onMemoChange(event.currentTarget.value)}
		></textarea>

		<div class="flex flex-wrap gap-3">
			<button
				class="px-4 py-2 rounded-lg bg-emerald-500/80 text-white font-semibold disabled:opacity-60"
				onclick={() => onRespond(true)}
				disabled={busy}
			>
				Accept
			</button>
			<button
				class="px-4 py-2 rounded-lg bg-red-500/80 text-white font-semibold disabled:opacity-60"
				onclick={() => onRespond(false)}
				disabled={busy}
			>
				Reject
			</button>
		</div>
	</div>
</div>
