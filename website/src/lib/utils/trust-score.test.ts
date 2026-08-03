import { describe, it, expect } from 'vitest';
import { hasEnoughTrustData, type TrustDataShape } from './trust-score';

// F5: a brand-new provider (zero contracts, zero reputation) was shown a green
// "Reliable" badge on a near-100 trust score. The backend scoring starts at 100
// and only deducts for observed negative signals, so absence of history reads as
// reliability — the opposite of honest. The dashboard TrustDashboard card must
// detect "no behavioural track record" and refuse to present the numeric score
// + verdict; it renders an N/A + neutral "Not enough data" state instead.
// See TrustDashboard.svelte for the call site.
describe('hasEnoughTrustData', () => {
	it('returns false when the provider has zero contracts (no track record)', () => {
		const fresh: TrustDataShape = { total_contracts: 0 };
		expect(hasEnoughTrustData(fresh)).toBe(false);
	});

	it('returns true as soon as the provider has at least one contract', () => {
		const tracked: TrustDataShape = { total_contracts: 1 };
		expect(hasEnoughTrustData(tracked)).toBe(true);
	});

	it('returns true for an established provider with many contracts', () => {
		const established: TrustDataShape = { total_contracts: 42 };
		expect(hasEnoughTrustData(established)).toBe(true);
	});
});
