import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// ---- Pure logic extracted from CommandPalette ----

const NAV_ITEMS = [
	{ type: 'nav' as const, label: 'Marketplace', description: 'Browse cloud offerings', href: '/dashboard/marketplace', icon: 'cart' },
	{ type: 'nav' as const, label: 'My Rentals', description: 'View your active rentals', href: '/dashboard/rentals', icon: 'file' },
	{ type: 'nav' as const, label: 'Invoices', description: 'View your invoices', href: '/dashboard/invoices', icon: 'download' },
	{ type: 'nav' as const, label: 'Account', description: 'Manage your account', href: '/dashboard/account', icon: 'user' },
];

function buildGroups(
	query: string,
	offeringResults: { type: 'offering'; label: string; description: string; href: string; icon: string; id: number }[],
	contractResults: { type: 'contract'; label: string; description: string; href: string; icon: string; contractId: string }[],
	_isAuthenticated: boolean
) {
	const q = query.trim();
	const result: { heading: string; items: { type: string; label: string }[] }[] = [];

	if (!q) {
		result.push({ heading: 'Navigation', items: NAV_ITEMS });
	} else {
		if (offeringResults.length > 0) result.push({ heading: 'Offerings', items: offeringResults });
		if (contractResults.length > 0) result.push({ heading: 'My Contracts', items: contractResults });
		if (offeringResults.length === 0 && contractResults.length === 0) {
			const filtered = NAV_ITEMS.filter(
				(n) =>
					n.label.toLowerCase().includes(q.toLowerCase()) ||
					n.description.toLowerCase().includes(q.toLowerCase())
			);
			if (filtered.length > 0) result.push({ heading: 'Navigation', items: filtered });
		}
	}
	return result;
}

function navigateHighlight(current: number, total: number, direction: 'up' | 'down'): number {
	if (total === 0) return 0;
	if (direction === 'down') return (current + 1) % total;
	return (current - 1 + total) % total;
}

// ---- Tests ----

describe('CommandPalette: empty query shows navigation items', () => {
	it('shows all 4 nav items when query is empty', () => {
		const groups = buildGroups('', [], [], false);
		expect(groups).toHaveLength(1);
		expect(groups[0].heading).toBe('Navigation');
		expect(groups[0].items).toHaveLength(4);
	});

	it('includes Marketplace, My Rentals, Invoices, Account links', () => {
		const groups = buildGroups('', [], [], false);
		const labels = groups[0].items.map((i) => i.label);
		expect(labels).toContain('Marketplace');
		expect(labels).toContain('My Rentals');
		expect(labels).toContain('Invoices');
		expect(labels).toContain('Account');
	});

	it('shows nav items for whitespace-only query', () => {
		const groups = buildGroups('   ', [], [], false);
		expect(groups).toHaveLength(1);
		expect(groups[0].heading).toBe('Navigation');
	});
});

describe('CommandPalette: query filters navigation when no API results', () => {
	it('filters nav items by label (case-insensitive)', () => {
		const groups = buildGroups('market', [], [], false);
		expect(groups).toHaveLength(1);
		expect(groups[0].heading).toBe('Navigation');
		expect(groups[0].items).toHaveLength(1);
		expect(groups[0].items[0].label).toBe('Marketplace');
	});

	it('filters nav items by description', () => {
		const groups = buildGroups('active rentals', [], [], false);
		expect(groups).toHaveLength(1);
		expect(groups[0].items[0].label).toBe('My Rentals');
	});

	it('returns empty groups when query matches nothing', () => {
		const groups = buildGroups('xyzzy-nonexistent', [], [], false);
		expect(groups).toHaveLength(0);
	});
});

describe('CommandPalette: shows offering and contract results when present', () => {
	it('shows offering group when offering results are provided', () => {
		const offerings = [{ type: 'offering' as const, label: 'GPU Server', description: 'gpu · US', href: '/dashboard/marketplace', icon: 'server', id: 1 }];
		const groups = buildGroups('gpu', offerings, [], false);
		expect(groups).toHaveLength(1);
		expect(groups[0].heading).toBe('Offerings');
		expect(groups[0].items[0].label).toBe('GPU Server');
	});

	it('shows contract group when contract results are provided', () => {
		const contracts = [{ type: 'contract' as const, label: 'abc-001', description: 'active · abc-001…', href: '/dashboard/rentals', icon: 'file', contractId: 'abc-001' }];
		const groups = buildGroups('abc', [], contracts, true);
		expect(groups).toHaveLength(1);
		expect(groups[0].heading).toBe('My Contracts');
	});

	it('shows both offering and contract groups together', () => {
		const offerings = [{ type: 'offering' as const, label: 'VPS', description: '', href: '/dashboard/marketplace', icon: 'server', id: 2 }];
		const contracts = [{ type: 'contract' as const, label: 'active-001', description: 'active · …', href: '/dashboard/rentals', icon: 'file', contractId: 'active-001' }];
		const groups = buildGroups('active', offerings, contracts, true);
		expect(groups).toHaveLength(2);
		expect(groups[0].heading).toBe('Offerings');
		expect(groups[1].heading).toBe('My Contracts');
	});

	it('offering/contract results take precedence over nav filtering when query has results', () => {
		// Even if query would match nav, if offering results exist we show offerings only (not nav)
		const offerings = [{ type: 'offering' as const, label: 'Market GPU', description: '', href: '/dashboard/marketplace', icon: 'server', id: 3 }];
		const groups = buildGroups('market', offerings, [], false);
		const headings = groups.map((g) => g.heading);
		expect(headings).not.toContain('Navigation');
		expect(headings).toContain('Offerings');
	});
});

describe('CommandPalette: keyboard navigation', () => {
	it('moves to next item on ArrowDown', () => {
		expect(navigateHighlight(0, 4, 'down')).toBe(1);
		expect(navigateHighlight(2, 4, 'down')).toBe(3);
	});

	it('wraps from last to first on ArrowDown', () => {
		expect(navigateHighlight(3, 4, 'down')).toBe(0);
	});

	it('moves to previous item on ArrowUp', () => {
		expect(navigateHighlight(2, 4, 'up')).toBe(1);
		expect(navigateHighlight(1, 4, 'up')).toBe(0);
	});

	it('wraps from first to last on ArrowUp', () => {
		expect(navigateHighlight(0, 4, 'up')).toBe(3);
	});

	it('handles empty list without error', () => {
		expect(navigateHighlight(0, 0, 'down')).toBe(0);
		expect(navigateHighlight(0, 0, 'up')).toBe(0);
	});
});

describe('CommandPalette: toggle open/close via keyboard shortcut', () => {
	let isOpen = false;

	function handleGlobalKeydown(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
			isOpen = !isOpen;
		}
	}

	beforeEach(() => {
		isOpen = false;
		window.addEventListener('keydown', handleGlobalKeydown);
	});

	afterEach(() => {
		window.removeEventListener('keydown', handleGlobalKeydown);
	});

	it('opens on Ctrl+K', () => {
		window.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true }));
		expect(isOpen).toBe(true);
	});

	it('opens on Cmd+K (metaKey)', () => {
		window.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', metaKey: true, bubbles: true }));
		expect(isOpen).toBe(true);
	});

	it('toggles closed when already open', () => {
		isOpen = true;
		window.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true }));
		expect(isOpen).toBe(false);
	});

	it('does not open on plain K press (no modifier)', () => {
		window.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', bubbles: true }));
		expect(isOpen).toBe(false);
	});
});

describe('CommandPalette: Escape closes the palette', () => {
	it('returns closed state when Escape is pressed', () => {
		// Simulate escape key handling
		let isOpen = true;
		function handleKeydown(e: KeyboardEvent) {
			if (e.key === 'Escape') isOpen = false;
		}
		const event = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true });
		handleKeydown(event);
		expect(isOpen).toBe(false);
	});
});

describe('CommandPalette: contract filtering', () => {
	interface Contract {
		contract_id: string;
		offering_id?: string;
		status: string;
	}

	function filterContracts(contracts: Contract[], q: string): Contract[] {
		const lower = q.toLowerCase();
		return contracts.filter((c) =>
			c.offering_id?.toLowerCase().includes(lower) ||
			c.status?.toLowerCase().includes(lower) ||
			c.contract_id?.toLowerCase().includes(lower)
		).slice(0, 5);
	}

	const sampleContracts: Contract[] = [
		{ contract_id: 'abc123', offering_id: 'gpu-server-v1', status: 'active' },
		{ contract_id: 'def456', offering_id: 'vps-small', status: 'pending' },
		{ contract_id: 'ghi789', offering_id: 'bare-metal', status: 'active' },
	];

	it('filters by offering_id', () => {
		const result = filterContracts(sampleContracts, 'gpu');
		expect(result).toHaveLength(1);
		expect(result[0].contract_id).toBe('abc123');
	});

	it('filters by status', () => {
		const result = filterContracts(sampleContracts, 'active');
		expect(result).toHaveLength(2);
	});

	it('filters by contract_id', () => {
		const result = filterContracts(sampleContracts, 'def456');
		expect(result).toHaveLength(1);
		expect(result[0].offering_id).toBe('vps-small');
	});

	it('caps results at 5', () => {
		const manyContracts = Array.from({ length: 10 }, (_, i) => ({
			contract_id: `id-${i}`,
			offering_id: 'same-offering',
			status: 'active',
		}));
		const result = filterContracts(manyContracts, 'same');
		expect(result).toHaveLength(5);
	});

	it('returns empty array when no match', () => {
		const result = filterContracts(sampleContracts, 'xyzzy');
		expect(result).toHaveLength(0);
	});
});
