import { describe, it, expect } from 'vitest';
import { marketplaceIsEmpty, type MarketplaceStats } from './marketplace-empty';

// U2: an all-zero Marketplace Statistics grid reads as a dead marketplace. The
// landing page must detect genuine emptiness (no providers AND no contracts)
// so it can show an honest early-access reframe instead of fabricated-looking
// zeros. See DashboardSection.svelte for the call site.
describe('marketplaceIsEmpty', () => {
	it('returns true when every signal is zero (genuinely empty)', () => {
		const allZero: MarketplaceStats = { totalProviders: 0, totalContracts: 0 };
		expect(marketplaceIsEmpty(allZero)).toBe(true);
	});

	it('returns false when at least one provider exists', () => {
		const withProviders: MarketplaceStats = { totalProviders: 1, totalContracts: 0 };
		expect(marketplaceIsEmpty(withProviders)).toBe(false);
	});

	it('returns false when contracts exist even with zero providers', () => {
		// A contract implies real marketplace activity regardless of the
		// provider count (e.g. a transient count gap or historical contracts).
		const withContracts: MarketplaceStats = { totalProviders: 0, totalContracts: 3 };
		expect(marketplaceIsEmpty(withContracts)).toBe(false);
	});

	it('returns false for a typically populated marketplace', () => {
		const populated: MarketplaceStats = { totalProviders: 12, totalContracts: 48 };
		expect(marketplaceIsEmpty(populated)).toBe(false);
	});
});
