import { describe, it, expect } from 'vitest';

type OfferingSubset = {
	monthly_price?: number;
	currency: string;
	reseller_commission_percent?: number;
};

function formatSimilarPrice(o: OfferingSubset, icpPriceUsd: number | null): { primary: string; usdEquivalent: string | null } {
	if (!o.monthly_price) {
		return { primary: 'On request', usdEquivalent: null };
	}
	let price = o.monthly_price;
	if (o.reseller_commission_percent) {
		price += price * (o.reseller_commission_percent / 100);
	}
	const currency = o.currency?.toUpperCase();
	if (currency === 'USD') {
		return { primary: `$${price.toFixed(2)}`, usdEquivalent: null };
	}
	if (currency === 'ICP' && icpPriceUsd && icpPriceUsd > 0) {
		const usdAmount = price * icpPriceUsd;
		return { primary: `${price.toFixed(2)} ICP`, usdEquivalent: `≈ $${usdAmount.toFixed(2)}/mo` };
	}
	return { primary: `${price.toFixed(2)} ${o.currency}`, usdEquivalent: null };
}

describe('formatSimilarPrice', () => {
	describe('basic formatting', () => {
		it('normalizes USD offerings to dollar sign format', () => {
			const result = formatSimilarPrice({ monthly_price: 10, currency: 'USD' }, null);
			expect(result.primary).toBe('$10.00');
			expect(result.usdEquivalent).toBeNull();
		});

		it('shows price with currency for ICP offerings without exchange rate', () => {
			const result = formatSimilarPrice({ monthly_price: 5, currency: 'ICP' }, null);
			expect(result.primary).toBe('5.00 ICP');
			expect(result.usdEquivalent).toBeNull();
		});

		it('shows "On request" when no monthly_price', () => {
			const result = formatSimilarPrice({ currency: 'USD' }, null);
			expect(result.primary).toBe('On request');
			expect(result.usdEquivalent).toBeNull();
		});
	});

	describe('ICP to USD conversion', () => {
		it('shows USD equivalent for ICP offerings when exchange rate available', () => {
			const result = formatSimilarPrice({ monthly_price: 10, currency: 'ICP' }, 5);
			expect(result.primary).toBe('10.00 ICP');
			expect(result.usdEquivalent).toBe('≈ $50.00/mo');
		});

		it('correctly calculates USD equivalent with different exchange rates', () => {
			const result = formatSimilarPrice({ monthly_price: 5, currency: 'ICP' }, 8.5);
			expect(result.primary).toBe('5.00 ICP');
			expect(result.usdEquivalent).toBe('≈ $42.50/mo');
		});

		it('does not show USD equivalent for non-ICP currencies even with exchange rate', () => {
			const result = formatSimilarPrice({ monthly_price: 10, currency: 'EUR' }, 5);
			expect(result.primary).toBe('10.00 EUR');
			expect(result.usdEquivalent).toBeNull();
		});
	});

	describe('reseller commission handling', () => {
		it('includes commission in ICP price and USD equivalent', () => {
			const result = formatSimilarPrice(
				{ monthly_price: 10, currency: 'ICP', reseller_commission_percent: 10 },
				5
			);
			expect(result.primary).toBe('11.00 ICP');
			expect(result.usdEquivalent).toBe('≈ $55.00/mo');
		});

		it('includes commission in USD price (normalized to dollar sign)', () => {
			const result = formatSimilarPrice(
				{ monthly_price: 100, currency: 'USD', reseller_commission_percent: 15 },
				null
			);
			expect(result.primary).toBe('$115.00');
			expect(result.usdEquivalent).toBeNull();
		});
	});

	describe('currency case handling', () => {
		it('handles lowercase USD currency', () => {
			const result = formatSimilarPrice({ monthly_price: 10, currency: 'usd' }, 5);
			expect(result.primary).toBe('$10.00');
			expect(result.usdEquivalent).toBeNull();
		});

		it('handles lowercase ICP currency', () => {
			const result = formatSimilarPrice({ monthly_price: 10, currency: 'icp' }, 5);
			expect(result.primary).toBe('10.00 ICP');
			expect(result.usdEquivalent).toBe('≈ $50.00/mo');
		});

		it('handles mixed case ICP currency', () => {
			const result = formatSimilarPrice({ monthly_price: 10, currency: 'Icp' }, 5);
			expect(result.primary).toBe('10.00 ICP');
			expect(result.usdEquivalent).toBe('≈ $50.00/mo');
		});
	});

	describe('currency normalization for consistent display', () => {
		it('USD and ICP offerings can be compared via USD equivalent', () => {
			const usdResult = formatSimilarPrice({ monthly_price: 50, currency: 'USD' }, 5);
			const icpResult = formatSimilarPrice({ monthly_price: 10, currency: 'ICP' }, 5);

			expect(usdResult.primary).toBe('$50.00');
			expect(icpResult.primary).toBe('10.00 ICP');
			expect(icpResult.usdEquivalent).toBe('≈ $50.00/mo');
		});

		it('non-USD, non-ICP currencies show currency code', () => {
			const result = formatSimilarPrice({ monthly_price: 100, currency: 'EUR' }, null);
			expect(result.primary).toBe('100.00 EUR');
			expect(result.usdEquivalent).toBeNull();
		});
	});

	describe('edge cases', () => {
		it('handles zero exchange rate as unavailable (no USD equivalent shown)', () => {
			const result = formatSimilarPrice({ monthly_price: 10, currency: 'ICP' }, 0);
			expect(result.primary).toBe('10.00 ICP');
			expect(result.usdEquivalent).toBeNull();
		});

		it('handles very small prices', () => {
			const result = formatSimilarPrice({ monthly_price: 0.001, currency: 'ICP' }, 5);
			expect(result.primary).toBe('0.00 ICP');
			expect(result.usdEquivalent).toBe('≈ $0.01/mo');
		});

		it('handles very large prices', () => {
			const result = formatSimilarPrice({ monthly_price: 10000, currency: 'ICP' }, 5);
			expect(result.primary).toBe('10000.00 ICP');
			expect(result.usdEquivalent).toBe('≈ $50000.00/mo');
		});
	});
});
