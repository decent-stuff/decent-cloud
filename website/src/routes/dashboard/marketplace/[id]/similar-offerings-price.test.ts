import { describe, it, expect } from 'vitest';

type OfferingSubset = {
	monthly_price?: number;
	currency: string;
};

// Local mirror of formatSimilarPrice from marketplace/[id]/+page.svelte.
// Kept in sync manually so the pure logic is unit-testable without a DOM.
function formatSimilarPrice(o: OfferingSubset): string {
	if (!o.monthly_price) {
		return 'On request';
	}
	const price = o.monthly_price;
	const currency = o.currency?.toUpperCase();
	if (currency === 'USD') {
		return `$${price.toFixed(2)}`;
	}
	return `${price.toFixed(2)} ${o.currency}`;
}

describe('formatSimilarPrice', () => {
	describe('basic formatting', () => {
		it('normalizes USD offerings to dollar sign format', () => {
			expect(formatSimilarPrice({ monthly_price: 10, currency: 'USD' })).toBe('$10.00');
		});

		it('shows "On request" when no monthly_price', () => {
			expect(formatSimilarPrice({ currency: 'USD' })).toBe('On request');
		});

		it('shows price with currency code for non-USD fiat', () => {
			expect(formatSimilarPrice({ monthly_price: 10, currency: 'EUR' })).toBe('10.00 EUR');
		});
	});

	describe('currency case handling', () => {
		it('handles lowercase USD currency', () => {
			expect(formatSimilarPrice({ monthly_price: 10, currency: 'usd' })).toBe('$10.00');
		});

		it('handles mixed-case USD currency', () => {
			expect(formatSimilarPrice({ monthly_price: 10, currency: 'UsD' })).toBe('$10.00');
		});

		it('preserves original case for non-USD currency codes', () => {
			expect(formatSimilarPrice({ monthly_price: 10, currency: 'eur' })).toBe('10.00 eur');
		});
	});
});
