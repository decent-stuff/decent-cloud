import { describe, it, expect } from 'vitest';
import { calculateSpendingByCurrency, type ContractForSpending } from '$lib/utils/contract-format';

describe('Provider Earnings Currency Display', () => {
	describe('calculateSpendingByCurrency (used for revenue totals)', () => {
		it('groups revenue by currency from contracts', () => {
			const contracts: ContractForSpending[] = [
				{ payment_amount_e9s: 10_000_000_000, currency: 'USD' },
				{ payment_amount_e9s: 5_000_000_000, currency: 'USD' },
				{ payment_amount_e9s: 2_000_000_000, currency: 'ICP' },
			];
			const result = calculateSpendingByCurrency(contracts);
			expect(result.get('USD')).toBe(15);
			expect(result.get('ICP')).toBe(2);
			expect(result.size).toBe(2);
		});

		it('handles mixed case currency codes', () => {
			const contracts: ContractForSpending[] = [
				{ payment_amount_e9s: 10_000_000_000, currency: 'usd' },
				{ payment_amount_e9s: 5_000_000_000, currency: 'USD' },
				{ payment_amount_e9s: 2_000_000_000, currency: 'Usd' },
			];
			const result = calculateSpendingByCurrency(contracts);
			expect(result.get('USD')).toBe(17);
			expect(result.size).toBe(1);
		});

		it('defaults to USD when currency is missing', () => {
			const contracts: ContractForSpending[] = [
				{ payment_amount_e9s: 10_000_000_000 },
				{ payment_amount_e9s: 5_000_000_000, currency: 'USD' },
			];
			const result = calculateSpendingByCurrency(contracts);
			expect(result.get('USD')).toBe(15);
			expect(result.size).toBe(1);
		});

		it('returns empty map for no contracts', () => {
			const result = calculateSpendingByCurrency([]);
			expect(result.size).toBe(0);
		});

		it('supports multiple currencies in same dataset', () => {
			const contracts: ContractForSpending[] = [
				{ payment_amount_e9s: 100_000_000_000, currency: 'USD' },
				{ payment_amount_e9s: 50_000_000_000, currency: 'EUR' },
				{ payment_amount_e9s: 25_000_000_000, currency: 'ICP' },
				{ payment_amount_e9s: 10_000_000_000, currency: 'USD' },
			];
			const result = calculateSpendingByCurrency(contracts);
			expect(result.get('USD')).toBe(110);
			expect(result.get('EUR')).toBe(50);
			expect(result.get('ICP')).toBe(25);
			expect(result.size).toBe(3);
		});
	});
});
