<script lang="ts">
	import { API_BASE_URL } from "$lib/services/api";
	import type { AgentPoolWithStats } from "$lib/types/generated/AgentPoolWithStats";
	import type { SetupToken } from "$lib/types/generated/SetupToken";

	interface Props {
		pool: AgentPoolWithStats;
		isOpen: boolean;
		onCreate: (label: string, expires: number) => Promise<void>;
		onClose: () => void;
		tokens: SetupToken[];
		onDelete: (token: string) => Promise<void>;
	}

	let { pool, isOpen = $bindable(false), onCreate, onClose, tokens = [], onDelete }: Props = $props();

	let label = $state("");
	let expiresHours = $state(24);
	let creating = $state(false);
	let actionMessage = $state("");

	async function handleSubmit(e: Event) {
		e.preventDefault();
		creating = true;
		actionMessage = "";
		try {
			await onCreate(label, expiresHours);
			label = "";
			actionMessage = "Token created successfully!";
		} catch (err) {
			console.error("Failed to create token", err);
			actionMessage = "Error: " + (err instanceof Error ? err.message : "Failed to create token");
		} finally {
			creating = false;
		}
	}

	function copyToClipboard(text: string) {
		navigator.clipboard.writeText(text);
		actionMessage = "Copied to clipboard";
	}

	function formatTimestamp(ns: number): string {
		const date = new Date(ns / 1_000_000);
		return date.toLocaleString();
	}

	function stopPropagation(e: Event) {
		e.stopPropagation();
	}
</script>

{#if isOpen}
	<div
		class="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center"
		onclick={onClose}
		onkeydown={(e) => e.key === 'Escape' && onClose()}
		role="dialog"
		aria-modal="true"
		tabindex="-1"
	>
		<div
			class="bg-gray-900 border border-white/20 rounded-2xl shadow-lg w-full max-w-2xl m-4 text-white"
			onclick={stopPropagation}
			onkeydown={stopPropagation}
			role="dialog"
			aria-labelledby="dialog-title"
			aria-describedby="dialog-description"
			tabindex="-1"
		>
			<header class="p-6 border-b border-white/10">
				<h2 id="dialog-title" class="text-2xl font-bold">Add Agent to {pool.name}</h2>
				<p id="dialog-description" class="text-sm text-white/60 mt-1">
					Generate a one-time setup token for a new agent.
				</p>
			</header>

			<div class="p-6 space-y-6 max-h-[70vh] overflow-y-auto">
				<!-- Create Token Form -->
				<form onsubmit={handleSubmit} class="space-y-3">
					<div class="text-sm font-medium text-white/80">New Token</div>
					<div class="flex flex-wrap gap-3">
						<input
							type="text"
							bind:value={label}
							placeholder="Optional label (e.g., proxmox-node-5)"
							class="flex-1 min-w-48 px-3 py-2 bg-white/5 border border-white/20 rounded-lg text-sm placeholder-white/40 focus:outline-none focus:border-blue-400"
						/>
						<select
							bind:value={expiresHours}
							class="px-3 py-2 bg-white/5 border border-white/20 rounded-lg text-sm focus:outline-none focus:border-blue-400"
						>
							<option value={1}>Expires in 1 hour</option>
							<option value={6}>Expires in 6 hours</option>
							<option value={24}>Expires in 24 hours</option>
							<option value={168}>Expires in 7 days</option>
						</select>
						<button
							type="submit"
							disabled={creating}
							class="px-5 py-2 bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 rounded-lg text-sm font-semibold hover:bg-emerald-500/30 transition-colors disabled:opacity-50"
						>
							{creating ? "Creating..." : "+ Generate"}
						</button>
					</div>
				</form>

				<!-- Pending Tokens List -->
				<div>
					<div class="text-sm font-medium text-white/80 mb-3">Pending Tokens</div>
					{#if tokens.length === 0}
						<div class="text-center py-6 text-white/50 text-sm border-2 border-dashed border-white/10 rounded-lg">
							No pending setup tokens for this pool.
						</div>
					{:else}
						<div class="space-y-3">
							{#each tokens as token}
								<div class="flex items-center gap-3 p-3 bg-white/10 rounded-lg">
									<div class="flex-1 min-w-0">
										<div class="font-mono text-sm text-emerald-300 truncate" title={token.token}>
											{token.token}
										</div>
										<div class="text-xs text-white/50 mt-1">
											{#if token.label}{token.label} &bull; {/if}
											Expires: {formatTimestamp(token.expiresAtNs)}
										</div>
									</div>
									<button
										onclick={() => copyToClipboard(`dc-agent setup token --token ${token.token} --api-url ${API_BASE_URL}`)}
										class="px-3 py-1 rounded text-xs font-medium bg-blue-500/20 text-blue-300 hover:bg-blue-500/30 transition-colors"
										title="Copy setup command"
									>
										Copy Cmd
									</button>
									<button
										onclick={() => onDelete(token.token)}
										class="p-2 rounded-full hover:bg-red-500/20 text-white/60 hover:text-red-300 transition-colors"
										title="Delete token"
									>
										<svg class="w-4 h-4" fill="currentColor" viewBox="0 0 20 20"><path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd"></path></svg>
									</button>
								</div>
							{/each}
						</div>
					{/if}
				</div>

				{#if actionMessage}
					<div class="text-center text-sm text-emerald-400 p-2">{actionMessage}</div>
				{/if}

			</div>

			<footer class="p-4 bg-white/5 text-right">
				<button
					onclick={onClose}
					class="px-6 py-2 rounded-lg text-white/80 hover:text-white hover:bg-white/10 transition-colors font-medium"
				>
					Close
				</button>
			</footer>
		</div>
	</div>
{/if}
