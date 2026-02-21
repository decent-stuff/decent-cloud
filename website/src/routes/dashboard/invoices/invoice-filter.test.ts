import { describe, it, expect } from 'vitest';

// Pure filter functions for invoice page - tested here before use in +page.svelte
// Note: Contract uses provider_pubkey (not provider_name), payment_amount_e9s (not amount_e9s)

type InvoiceItem = {
	contract_id: string;
	provider_pubkey: string;
	created_at_ns: number;
	payment_status: string;
	payment_amount_e9s: number;
};

const DAY_NS = 24 * 60 * 60 * 1_000_000_000;

function filterByDateRange(contracts: InvoiceItem[], days: number | null): InvoiceItem[] {
	if (!days) return contracts;
	const cutoff = Date.now() * 1_000_000 - days * DAY_NS;
	return contracts.filter(c => c.created_at_ns >= cutoff);
}

function filterBySearch(contracts: InvoiceItem[], query: string): InvoiceItem[] {
	if (!query.trim()) return contracts;
	const q = query.toLowerCase();
	return contracts.filter(c =>
		c.contract_id.toLowerCase().includes(q) ||
		c.provider_pubkey.toLowerCase().includes(q)
	);
}

function filterByStatus(contracts: InvoiceItem[], status: 'all' | 'paid' | 'pending'): InvoiceItem[] {
	if (status === 'all') return contracts;
	if (status === 'paid') return contracts.filter(c => c.payment_status === 'succeeded' || c.payment_status === 'refunded');
	// pending = not yet paid
	return contracts.filter(c => c.payment_status !== 'succeeded' && c.payment_status !== 'refunded');
}

function sumAmounts(contracts: InvoiceItem[]): number {
	return contracts.reduce((sum, c) => sum + (c.payment_amount_e9s || 0), 0) / 1e9;
}

describe('invoice filters', () => {
	const now = Date.now() * 1_000_000;

	const sampleContracts = [
		{ contract_id: 'abc123def456', provider_pubkey: 'pubkey-aaa', created_at_ns: now - 5 * DAY_NS, payment_status: 'succeeded', payment_amount_e9s: 100_000_000 },
		{ contract_id: 'def456ghi789', provider_pubkey: 'pubkey-bbb', created_at_ns: now - 15 * DAY_NS, payment_status: 'pending', payment_amount_e9s: 200_000_000 },
		{ contract_id: 'ghi789jkl012', provider_pubkey: 'pubkey-aaa', created_at_ns: now - 45 * DAY_NS, payment_status: 'refunded', payment_amount_e9s: 50_000_000 },
	];

	describe('filterByDateRange', () => {
		it('null days returns all contracts', () => {
			expect(filterByDateRange(sampleContracts, null)).toHaveLength(3);
		});

		it('7 days returns only the contract from 5 days ago', () => {
			expect(filterByDateRange(sampleContracts, 7)).toHaveLength(1);
		});

		it('30 days returns contracts from 5 and 15 days ago', () => {
			expect(filterByDateRange(sampleContracts, 30)).toHaveLength(2);
		});

		it('90 days returns all three contracts', () => {
			expect(filterByDateRange(sampleContracts, 90)).toHaveLength(3);
		});

		it('1 day returns no contracts when none are that recent', () => {
			expect(filterByDateRange(sampleContracts, 1)).toHaveLength(0);
		});
	});

	describe('filterBySearch', () => {
		it('empty query returns all contracts', () => {
			expect(filterBySearch(sampleContracts, '')).toHaveLength(3);
		});

		it('whitespace-only query returns all contracts', () => {
			expect(filterBySearch(sampleContracts, '   ')).toHaveLength(3);
		});

		it('partial contract_id match returns matching contracts', () => {
			expect(filterBySearch(sampleContracts, 'abc')).toHaveLength(1);
			expect(filterBySearch(sampleContracts, 'abc')[0].contract_id).toBe('abc123def456');
		});

		it('provider_pubkey match returns all contracts with that pubkey', () => {
			expect(filterBySearch(sampleContracts, 'pubkey-aaa')).toHaveLength(2);
		});

		it('case-insensitive matching on contract_id', () => {
			expect(filterBySearch(sampleContracts, 'ABC')).toHaveLength(1);
		});

		it('no match returns empty array', () => {
			expect(filterBySearch(sampleContracts, 'zzz')).toHaveLength(0);
		});
	});

	describe('filterByStatus', () => {
		it('"all" returns all contracts regardless of payment_status', () => {
			expect(filterByStatus(sampleContracts, 'all')).toHaveLength(3);
		});

		it('"paid" returns only succeeded and refunded contracts', () => {
			const result = filterByStatus(sampleContracts, 'paid');
			expect(result).toHaveLength(2);
			expect(result.every(c => c.payment_status === 'succeeded' || c.payment_status === 'refunded')).toBe(true);
		});

		it('"pending" returns only non-paid contracts', () => {
			const result = filterByStatus(sampleContracts, 'pending');
			expect(result).toHaveLength(1);
			expect(result[0].payment_status).toBe('pending');
		});
	});

	describe('sumAmounts', () => {
		it('sums payment_amount_e9s and converts to decimal ICP', () => {
			// 100_000_000 + 200_000_000 + 50_000_000 = 350_000_000 e9s = 0.35 ICP
			expect(sumAmounts(sampleContracts)).toBeCloseTo(0.35, 5);
		});

		it('returns 0 for empty array', () => {
			expect(sumAmounts([])).toBe(0);
		});

		it('handles single contract correctly', () => {
			expect(sumAmounts([sampleContracts[0]])).toBeCloseTo(0.1, 5);
		});
	});

	describe('combined filters (AND logic)', () => {
		it('date + status filters compose correctly', () => {
			// last 30 days: contracts 0 and 1; paid from those: only contract 0
			const byDate = filterByDateRange(sampleContracts, 30);
			const byStatus = filterByStatus(byDate, 'paid');
			expect(byStatus).toHaveLength(1);
			expect(byStatus[0].contract_id).toBe('abc123def456');
		});

		it('search + date filters compose correctly', () => {
			// all time, search pubkey-aaa: contracts 0 and 2
			const bySearch = filterBySearch(sampleContracts, 'pubkey-aaa');
			expect(bySearch).toHaveLength(2);
		});
	});
});
