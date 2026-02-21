<script lang="ts">
	import type { Ed25519KeyIdentity } from '@dfinity/identity';
	import { getOfferingAllowlist, addToAllowlist, removeFromAllowlist, type AllowlistEntry } from '$lib/services/api';
	import { signRequest } from '$lib/services/auth-api';

	interface Props {
		offeringId: number;
		offeringName: string;
		providerPubkey: string;
		identity: Ed25519KeyIdentity;
		onClose: () => void;
	}

	let { offeringId, offeringName, providerPubkey, identity, onClose }: Props = $props();

	let entries = $state<AllowlistEntry[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let newPubkey = $state('');
	let adding = $state(false);
	let removingId = $state<number | null>(null);

	async function loadEntries() {
		loading = true;
		error = null;
		try {
			const signed = await signRequest(identity, 'GET', `/api/v1/providers/${providerPubkey}/offerings/${offeringId}/allowlist`);
			entries = await getOfferingAllowlist(providerPubkey, offeringId, signed.headers);
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load allowlist';
		} finally {
			loading = false;
		}
	}

	async function handleAdd() {
		const pubkey = newPubkey.trim();
		if (!pubkey) return;

		adding = true;
		error = null;
		try {
			const path = `/api/v1/providers/${providerPubkey}/offerings/${offeringId}/allowlist`;
			const signed = await signRequest(identity, 'POST', path, { allowed_pubkey: pubkey });
			await addToAllowlist(providerPubkey, offeringId, pubkey, signed.headers, signed.body);
			newPubkey = '';
			await loadEntries();
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to add to allowlist';
		} finally {
			adding = false;
		}
	}

	async function handleRemove(entry: AllowlistEntry) {
		removingId = entry.id;
		error = null;
		try {
			const path = `/api/v1/providers/${providerPubkey}/offerings/${offeringId}/allowlist/${entry.allowed_pubkey}`;
			const signed = await signRequest(identity, 'DELETE', path);
			await removeFromAllowlist(providerPubkey, offeringId, entry.allowed_pubkey, signed.headers);
			await loadEntries();
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to remove from allowlist';
		} finally {
			removingId = null;
		}
	}

	function formatPubkey(hex: string): string {
		if (hex.length <= 12) return hex;
		return hex.slice(0, 6) + '...' + hex.slice(-6);
	}

	function formatDate(ns: number): string {
		return new Date(ns / 1_000_000).toLocaleDateString();
	}

	function stopPropagation(e: Event) {
		e.stopPropagation();
	}

	// Load on mount
	$effect(() => {
		loadEntries();
	});
</script>

<div
	class="fixed inset-0 bg-base/80 backdrop-blur-sm z-50 flex items-center justify-center"
	onclick={onClose}
	onkeydown={(e) => e.key === 'Escape' && onClose()}
	role="dialog"
	aria-modal="true"
	tabindex="-1"
>
	<div
		class="bg-surface border border-neutral-800 shadow-lg w-full max-w-xl m-4 text-white"
		onclick={stopPropagation}
		onkeydown={stopPropagation}
		role="dialog"
		aria-labelledby="allowlist-dialog-title"
		tabindex="-1"
	>
		<header class="p-6 border-b border-neutral-800">
			<h2 id="allowlist-dialog-title" class="text-xl font-bold">Allowlist for {offeringName}</h2>
			<p class="text-sm text-neutral-500 mt-1">
				If the allowlist is empty, all users can rent this offering. Add entries to restrict access.
			</p>
		</header>

		<div class="p-6 space-y-5 max-h-[60vh] overflow-y-auto">
			{#if error}
				<div class="p-3 bg-red-500/10 border border-red-500/30 text-red-400 text-sm">
					{error}
				</div>
			{/if}

			<!-- Add new entry -->
			<form
				onsubmit={(e) => { e.preventDefault(); handleAdd(); }}
				class="flex gap-2"
			>
				<input
					type="text"
					bind:value={newPubkey}
					placeholder="Tenant public key (hex)"
					class="flex-1 px-3 py-2 bg-surface-elevated border border-neutral-800 text-sm placeholder-white/40 focus:outline-none focus:border-primary-400"
				/>
				<button
					type="submit"
					disabled={adding || !newPubkey.trim()}
					class="px-4 py-2 bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 text-sm font-semibold hover:bg-emerald-500/30 transition-colors disabled:opacity-50"
				>
					{adding ? 'Adding...' : 'Add'}
				</button>
			</form>

			<!-- Current entries -->
			{#if loading}
				<div class="flex items-center justify-center py-8">
					<div class="animate-spin rounded-full h-6 w-6 border-b-2 border-primary-400"></div>
				</div>
			{:else if entries.length === 0}
				<div class="text-center py-6 text-neutral-500 text-sm border-2 border-dashed border-neutral-800">
					No entries — all users can rent this offering.
				</div>
			{:else}
				<div class="space-y-2">
					{#each entries as entry (entry.id)}
						<div class="flex items-center justify-between gap-3 p-3 bg-surface-elevated">
							<div class="min-w-0">
								<div class="font-mono text-sm text-primary-300 truncate" title={entry.allowed_pubkey}>
									{formatPubkey(entry.allowed_pubkey)}
								</div>
								<div class="text-xs text-neutral-500 mt-0.5">Added {formatDate(entry.created_at)}</div>
							</div>
							<button
								onclick={() => handleRemove(entry)}
								disabled={removingId === entry.id}
								class="p-1.5 rounded hover:bg-red-500/20 text-neutral-500 hover:text-red-300 transition-colors disabled:opacity-50 shrink-0"
								title="Remove from allowlist"
							>
								<svg class="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
									<path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd" />
								</svg>
							</button>
						</div>
					{/each}
				</div>
			{/if}
		</div>

		<footer class="p-4 bg-surface-elevated text-right border-t border-neutral-800">
			<button
				onclick={onClose}
				class="px-6 py-2 text-neutral-300 hover:text-white hover:bg-surface-elevated transition-colors font-medium"
			>
				Close
			</button>
		</footer>
	</div>
</div>
