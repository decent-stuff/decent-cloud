import { describe, it, expect } from 'vitest';
import {
	hasEnoughTrustData,
	getScoreTier,
	getScoreColor,
	getScoreBgColor,
	getScoreLabel,
	type TrustDataShape
} from './trust-score';

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

// The score-tier helpers are the single source for the 80/60 thresholds shared
// by TrustDashboard and the reputation leaderboard. Pinning the boundaries here
// guards the DRY contract: a single change to the thresholds flows everywhere.
describe('score tier helpers (shared 80/60 thresholds)', () => {
	describe('getScoreTier', () => {
		it('classifies reliable at >=80, caution at 60-79, high-risk below 60', () => {
			expect(getScoreTier(100)).toBe('reliable');
			expect(getScoreTier(80)).toBe('reliable');
			expect(getScoreTier(79)).toBe('caution');
			expect(getScoreTier(60)).toBe('caution');
			expect(getScoreTier(59)).toBe('high-risk');
			expect(getScoreTier(0)).toBe('high-risk');
		});
	});

	describe('getScoreColor', () => {
		it('returns the green/yellow/red text class by tier', () => {
			expect(getScoreColor(95)).toBe('text-green-400');
			expect(getScoreColor(65)).toBe('text-yellow-400');
			expect(getScoreColor(30)).toBe('text-red-400');
		});
	});

	describe('getScoreBgColor', () => {
		it('returns the green/yellow/red bg+border class by tier', () => {
			expect(getScoreColor(95)).toContain('green');
			expect(getScoreBgColor(65)).toContain('yellow');
			expect(getScoreBgColor(30)).toContain('red');
			// background + border both present
			expect(getScoreBgColor(95)).toMatch(/bg-green-500\/20 border-green-500\/50/);
		});
	});

	describe('getScoreLabel', () => {
		it('returns Reliable / Caution / High Risk by tier', () => {
			expect(getScoreLabel(95)).toBe('Reliable');
			expect(getScoreLabel(65)).toBe('Caution');
			expect(getScoreLabel(30)).toBe('High Risk');
		});
	});
});
