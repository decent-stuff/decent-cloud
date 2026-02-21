<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { authStore } from '$lib/stores/auth';
	import { searchOfferings, getUserContracts, type Offering, type Contract } from '$lib/services/api';
	import { signRequest } from '$lib/services/auth-api';
	import { hexEncode } from '$lib/services/api';
	import { Ed25519KeyIdentity } from '@dfinity/identity';
	import Icon from '$lib/components/Icons.svelte';

	interface NavResult {
		type: 'nav';
		label: string;
		description: string;
		href: string;
		icon: string;
	}

	interface OfferingResult {
		type: 'offering';
		label: string;
		description: string;
		href: string;
		icon: string;
		id: number;
	}

	interface ContractResult {
		type: 'contract';
		label: string;
		description: string;
		href: string;
		icon: string;
		contractId: string;
	}

	type Result = NavResult | OfferingResult | ContractResult;

	interface ResultGroup {
		heading: string;
		items: Result[];
	}

	let isOpen = $state(false);
	let query = $state('');
	let highlightedIndex = $state(0);
	let inputEl = $state<HTMLInputElement | null>(null);
	let isAuthenticated = $state(false);
	let activeIdentity = $state<import('$lib/stores/auth').IdentityInfo | null>(null);
	let offeringResults = $state<OfferingResult[]>([]);
	let contractResults = $state<ContractResult[]>([]);
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;
	let unsubAuth: (() => void) | null = null;
	let unsubIdentity: (() => void) | null = null;

	const NAV_ITEMS: NavResult[] = [
		{ type: 'nav', label: 'Marketplace', description: 'Browse cloud offerings', href: '/dashboard/marketplace', icon: 'cart' },
		{ type: 'nav', label: 'My Rentals', description: 'View your active rentals', href: '/dashboard/rentals', icon: 'file' },
		{ type: 'nav', label: 'Invoices', description: 'View your invoices', href: '/dashboard/invoices', icon: 'download' },
		{ type: 'nav', label: 'Account', description: 'Manage your account', href: '/dashboard/account', icon: 'user' },
	];

	const groups = $derived<ResultGroup[]>(buildGroups(query, offeringResults, contractResults));

	function buildGroups(q: string, offerings: OfferingResult[], contracts: ContractResult[]): ResultGroup[] {
		const result: ResultGroup[] = [];
		const trimmed = q.trim();

		if (!trimmed) {
			result.push({ heading: 'Navigation', items: NAV_ITEMS });
		} else {
			if (offerings.length > 0) result.push({ heading: 'Offerings', items: offerings });
			if (contracts.length > 0) result.push({ heading: 'My Contracts', items: contracts });
			if (offerings.length === 0 && contracts.length === 0) {
				const filtered = NAV_ITEMS.filter(
					(n) =>
						n.label.toLowerCase().includes(trimmed.toLowerCase()) ||
						n.description.toLowerCase().includes(trimmed.toLowerCase())
				);
				if (filtered.length > 0) result.push({ heading: 'Navigation', items: filtered });
			}
		}
		return result;
	}

	const flatItems = $derived<Result[]>(groups.flatMap((g) => g.items));

	function open() {
		isOpen = true;
		highlightedIndex = 0;
		query = '';
		offeringResults = [];
		contractResults = [];
	}

	function close() {
		isOpen = false;
		query = '';
		offeringResults = [];
		contractResults = [];
		if (debounceTimer !== null) {
			clearTimeout(debounceTimer);
			debounceTimer = null;
		}
	}

	function selectItem(item: Result) {
		close();
		goto(item.href);
	}

	function handleKeydown(e: KeyboardEvent) {
		if (!isOpen) return;

		switch (e.key) {
			case 'ArrowDown':
				e.preventDefault();
				highlightedIndex = (highlightedIndex + 1) % Math.max(flatItems.length, 1);
				break;
			case 'ArrowUp':
				e.preventDefault();
				highlightedIndex = (highlightedIndex - 1 + Math.max(flatItems.length, 1)) % Math.max(flatItems.length, 1);
				break;
			case 'Enter':
				e.preventDefault();
				if (flatItems[highlightedIndex]) {
					selectItem(flatItems[highlightedIndex]);
				}
				break;
			case 'Escape':
				e.preventDefault();
				close();
				break;
		}
	}

	function handleGlobalKeydown(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
			e.preventDefault();
			if (isOpen) {
				close();
			} else {
				open();
			}
		}
	}

	async function fetchOfferings(q: string) {
		const results = await searchOfferings({ q, limit: 5 });
		offeringResults = results.map((o: Offering) => ({
			type: 'offering' as const,
			label: o.offer_name ?? `Offering #${o.offering_id}`,
			description: [o.product_type, o.datacenter_country].filter(Boolean).join(' · '),
			href: `/dashboard/marketplace`,
			icon: 'server',
			id: o.id ?? 0,
		}));
	}

	async function fetchContracts(q: string) {
		if (!activeIdentity?.publicKeyBytes || !(activeIdentity.identity instanceof Ed25519KeyIdentity)) return;

		const pubkeyHex = hexEncode(activeIdentity.publicKeyBytes);
		const path = `/api/v1/users/${pubkeyHex}/contracts`;
		const signed = await signRequest(activeIdentity.identity as Ed25519KeyIdentity, 'GET', path);
		const contracts = await getUserContracts(signed.headers, pubkeyHex);

		const lower = q.toLowerCase();
		const filtered = contracts
			.filter((c: Contract) =>
				c.offering_id?.toLowerCase().includes(lower) ||
				c.status?.toLowerCase().includes(lower) ||
				c.contract_id?.toLowerCase().includes(lower)
			)
			.slice(0, 5);

		contractResults = filtered.map((c: Contract) => ({
			type: 'contract' as const,
			label: c.offering_id ?? c.contract_id,
			description: `${c.status} · ${c.contract_id.slice(0, 8)}…`,
			href: `/dashboard/rentals`,
			icon: 'file',
			contractId: c.contract_id,
		}));
	}

	function onQueryInput() {
		highlightedIndex = 0;
		if (debounceTimer !== null) clearTimeout(debounceTimer);

		const q = query.trim();
		if (!q) {
			offeringResults = [];
			contractResults = [];
			return;
		}

		debounceTimer = setTimeout(async () => {
			await Promise.all([
				fetchOfferings(q),
				isAuthenticated ? fetchContracts(q) : Promise.resolve()
			]);
		}, 200);
	}

	// Reset highlighted index when groups change
	$effect(() => {
		// access flatItems to react to changes
		void flatItems;
		highlightedIndex = 0;
	});

	// Focus input when opened
	$effect(() => {
		if (isOpen && inputEl) {
			inputEl.focus();
		}
	});

	onMount(() => {
		window.addEventListener('keydown', handleGlobalKeydown);
		unsubAuth = authStore.isAuthenticated.subscribe((v) => { isAuthenticated = v; });
		unsubIdentity = authStore.activeIdentity.subscribe((v) => { activeIdentity = v; });
	});

	onDestroy(() => {
		window.removeEventListener('keydown', handleGlobalKeydown);
		unsubAuth?.();
		unsubIdentity?.();
		if (debounceTimer !== null) clearTimeout(debounceTimer);
	});

	export function openPalette() {
		open();
	}
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen}
	<!-- Overlay: non-interactive backdrop, keyboard close handled by svelte:window -->
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div
		role="presentation"
		class="fixed inset-0 bg-black/50 z-50 flex items-start justify-center pt-20"
		onclick={close}
		onkeydown={(e) => { if (e.key === 'Escape') close(); }}
	>
		<!-- Modal: stop click propagation so clicking inside doesn't close -->
		<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
		<div
			role="dialog"
			aria-label="Command palette"
			aria-modal="true"
			tabindex="-1"
			class="bg-surface border border-neutral-700 rounded-lg w-full max-w-xl shadow-2xl overflow-hidden"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<!-- Search input -->
			<div class="flex items-center border-b border-neutral-700">
				<div class="pl-4 text-neutral-500 flex-shrink-0">
					<Icon name="search" size={16} />
				</div>
				<input
					bind:this={inputEl}
					bind:value={query}
					oninput={onQueryInput}
					type="text"
					placeholder="Search or navigate..."
					class="w-full px-4 py-3 bg-transparent text-white placeholder-neutral-500 outline-none text-sm"
					aria-label="Command palette search"
					aria-autocomplete="list"
					aria-controls="command-palette-results"
				/>
				<button
					type="button"
					onclick={close}
					class="pr-4 text-neutral-500 hover:text-white transition-colors"
					aria-label="Close"
				>
					<Icon name="x" size={16} />
				</button>
			</div>

			<!-- Results -->
			<div id="command-palette-results" role="listbox" class="max-h-80 overflow-y-auto py-1">
				{#if groups.length === 0}
					<p class="px-4 py-6 text-sm text-neutral-500 text-center">No results</p>
				{:else}
					{#each groups as group}
						<div>
							<div class="px-4 py-1.5 text-xs font-medium text-neutral-500 uppercase tracking-wider">
								{group.heading}
							</div>
							{#each group.items as item}
								{@const idx = flatItems.indexOf(item)}
								{@const isHighlighted = idx === highlightedIndex}
								<button
									type="button"
									role="option"
									aria-selected={isHighlighted}
									onclick={() => selectItem(item)}
									onmouseenter={() => { highlightedIndex = idx; }}
									class="w-full px-4 py-2.5 flex items-center gap-3 cursor-pointer text-sm text-left transition-colors {isHighlighted ? 'bg-surface-hover' : 'hover:bg-surface-hover'}"
								>
									<span class="flex-shrink-0 text-neutral-400">
										<Icon name={item.icon as import('$lib/components/Icons.svelte').IconName} size={16} />
									</span>
									<span class="flex-1 min-w-0">
										<span class="text-white">{item.label}</span>
										{#if item.description}
											<span class="ml-2 text-neutral-500 text-xs truncate">{item.description}</span>
										{/if}
									</span>
									{#if isHighlighted}
										<span class="flex-shrink-0 text-neutral-600 text-xs">↵</span>
									{/if}
								</button>
							{/each}
						</div>
					{/each}
				{/if}
			</div>

			<!-- Footer hint -->
			<div class="px-4 py-2 border-t border-neutral-800 flex items-center gap-4 text-xs text-neutral-600">
				<span><kbd class="font-mono">↑↓</kbd> navigate</span>
				<span><kbd class="font-mono">↵</kbd> select</span>
				<span><kbd class="font-mono">Esc</kbd> close</span>
			</div>
		</div>
	</div>
{/if}
