import { describe, it, expect } from 'vitest';
import { filterSimilarOfferings } from './similar-offerings';

type OfferingWithCurrency = {
	id: number;
	product_type: string;
	currency: string;
	monthly_price?: number;
	provider_online?: boolean;
	has_critical_flags?: boolean;
};

describe('filterSimilarOfferings', () => {
	const mainOfferingUsd: OfferingWithCurrency = {
		id: 1,
		product_type: 'vps',
		currency: 'USD',
		monthly_price: 10
	};

	const mainOfferingIcp: OfferingWithCurrency = {
		id: 2,
		product_type: 'vps',
		currency: 'ICP',
		monthly_price: 5
	};

	describe('currency consistency', () => {
		it('excludes ICP offerings when main offering is USD', () => {
			const allOfferings: OfferingWithCurrency[] = [
				mainOfferingUsd,
				{ id: 10, product_type: 'vps', currency: 'ICP', monthly_price: 5 },
				{ id: 11, product_type: 'vps', currency: 'USD', monthly_price: 12 },
			];

			const result = filterSimilarOfferings(allOfferings, mainOfferingUsd);

			expect(result).toHaveLength(1);
			expect(result[0].currency).toBe('USD');
			expect(result[0].id).toBe(11);
		});

		it('excludes USD offerings when main offering is ICP', () => {
			const allOfferings: OfferingWithCurrency[] = [
				mainOfferingIcp,
				{ id: 10, product_type: 'vps', currency: 'USD', monthly_price: 10 },
				{ id: 11, product_type: 'vps', currency: 'ICP', monthly_price: 6 },
			];

			const result = filterSimilarOfferings(allOfferings, mainOfferingIcp);

			expect(result).toHaveLength(1);
			expect(result[0].currency).toBe('ICP');
			expect(result[0].id).toBe(11);
		});

		it('only includes offerings with matching currency (case-insensitive)', () => {
			const allOfferings: OfferingWithCurrency[] = [
				{ id: 1, product_type: 'vps', currency: 'usd', monthly_price: 10 },
				{ id: 2, product_type: 'vps', currency: 'USD', monthly_price: 12 },
				{ id: 3, product_type: 'vps', currency: 'Usd', monthly_price: 15 },
				{ id: 4, product_type: 'vps', currency: 'ICP', monthly_price: 5 },
				{ id: 5, product_type: 'vps', currency: 'EUR', monthly_price: 20 },
			];

			const result = filterSimilarOfferings(allOfferings, { id: 99, product_type: 'vps', currency: 'USD' });

			expect(result).toHaveLength(3);
			result.forEach(o => expect(o.currency.toUpperCase()).toBe('USD'));
		});

		it('returns empty array when no matching currency offerings exist', () => {
			const allOfferings: OfferingWithCurrency[] = [
				{ id: 1, product_type: 'vps', currency: 'ICP', monthly_price: 5 },
				{ id: 2, product_type: 'vps', currency: 'EUR', monthly_price: 20 },
			];

			const result = filterSimilarOfferings(allOfferings, { id: 99, product_type: 'vps', currency: 'USD' });

			expect(result).toHaveLength(0);
		});
	});

	describe('product type filtering', () => {
		it('only includes offerings with matching product type', () => {
			const allOfferings: OfferingWithCurrency[] = [
				{ id: 1, product_type: 'vps', currency: 'USD', monthly_price: 10 },
				{ id: 2, product_type: 'storage', currency: 'USD', monthly_price: 5 },
				{ id: 3, product_type: 'vps', currency: 'USD', monthly_price: 12 },
			];

			const result = filterSimilarOfferings(allOfferings, { id: 99, product_type: 'vps', currency: 'USD' });

			expect(result).toHaveLength(2);
			result.forEach(o => expect(o.product_type).toBe('vps'));
		});
	});

	describe('exclusion of current offering', () => {
		it('excludes the main offering from results', () => {
			const allOfferings: OfferingWithCurrency[] = [
				{ id: 1, product_type: 'vps', currency: 'USD', monthly_price: 10 },
				{ id: 2, product_type: 'vps', currency: 'USD', monthly_price: 12 },
			];

			const result = filterSimilarOfferings(allOfferings, { id: 1, product_type: 'vps', currency: 'USD' });

			expect(result).toHaveLength(1);
			expect(result[0].id).toBe(2);
		});
	});

	describe('result limiting', () => {
		it('limits results to maxResults parameter', () => {
			const allOfferings: OfferingWithCurrency[] = Array.from({ length: 10 }, (_, i) => ({
				id: i + 1,
				product_type: 'vps',
				currency: 'USD',
				monthly_price: 10 + i
			}));

			const result = filterSimilarOfferings(allOfferings, { id: 99, product_type: 'vps', currency: 'USD' }, 3);

			expect(result).toHaveLength(3);
		});

		it('defaults to 4 results when maxResults not specified', () => {
			const allOfferings: OfferingWithCurrency[] = Array.from({ length: 10 }, (_, i) => ({
				id: i + 1,
				product_type: 'vps',
				currency: 'USD',
				monthly_price: 10 + i
			}));

			const result = filterSimilarOfferings(allOfferings, { id: 99, product_type: 'vps', currency: 'USD' });

			expect(result).toHaveLength(4);
		});
	});

	// Liveness guard: a "Similar Offerings" link must never route a buyer to a
	// dead/offline detail page. Mirrors the marketplace list's own offline-hiding
	// (provider_online === false) and additionally drops offerings carrying
	// critical flags — these are not "genuinely rentable alternatives".
	describe('liveness filtering (no dead-end links)', () => {
		it('excludes offerings where provider_online is false', () => {
			const allOfferings: OfferingWithCurrency[] = [
				{ id: 10, product_type: 'vps', currency: 'USD', provider_online: true },
				{ id: 11, product_type: 'vps', currency: 'USD', provider_online: false },
			];

			const result = filterSimilarOfferings(allOfferings, { id: 1, product_type: 'vps', currency: 'USD' });

			expect(result.map((o) => o.id)).toEqual([10]);
		});

		it('excludes offerings with has_critical_flags true', () => {
			const allOfferings: OfferingWithCurrency[] = [
				{ id: 10, product_type: 'vps', currency: 'USD', has_critical_flags: false },
				{ id: 11, product_type: 'vps', currency: 'USD', has_critical_flags: true },
			];

			const result = filterSimilarOfferings(allOfferings, { id: 1, product_type: 'vps', currency: 'USD' });

			expect(result.map((o) => o.id)).toEqual([10]);
		});

		it('excludes offline offerings even when they carry has_critical_flags', () => {
			// Real-world catalog state: the demo/seeded offline alternatives are
			// BOTH offline AND flagged. Regression for the 1628 detail page, which
			// previously surfaced 956/1206/957/1207 as "similar" — all dead ends.
			const allOfferings: OfferingWithCurrency[] = [
				{ id: 956, product_type: 'vps', currency: 'USD', provider_online: false, has_critical_flags: true },
				{ id: 1206, product_type: 'vps', currency: 'USD', provider_online: false, has_critical_flags: true },
				{ id: 957, product_type: 'vps', currency: 'USD', provider_online: false, has_critical_flags: true },
				{ id: 1207, product_type: 'vps', currency: 'USD', provider_online: false, has_critical_flags: true },
			];

			const result = filterSimilarOfferings(allOfferings, { id: 1628, product_type: 'vps', currency: 'USD' });

			expect(result).toHaveLength(0);
		});

		it('includes offerings with unknown online status (undefined) — unknown is not offline', () => {
			const allOfferings: OfferingWithCurrency[] = [
				{ id: 10, product_type: 'vps', currency: 'USD' /* provider_online undefined */ },
			];

			const result = filterSimilarOfferings(allOfferings, { id: 1, product_type: 'vps', currency: 'USD' });

			expect(result.map((o) => o.id)).toEqual([10]);
		});
	});
});
