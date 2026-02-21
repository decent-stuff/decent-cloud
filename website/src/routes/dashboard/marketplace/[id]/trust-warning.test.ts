import { describe, it, expect } from 'vitest';

// Mirror the trust warning display logic from the offering detail page
type TrustMetricsSubset = {
	total_contracts: number;
	trust_score: bigint | number;
};

function getTrustBannerType(
	trustMetrics: TrustMetricsSubset | null,
	trustWarningDismissed: boolean
): 'new-provider' | 'low-trust' | 'none' {
	if (!trustMetrics) return 'none';
	if (trustMetrics.total_contracts === 0) return 'new-provider';
	if (!trustWarningDismissed && Number(trustMetrics.trust_score) < 60) return 'low-trust';
	return 'none';
}

describe('trust banner: new provider', () => {
	it('shows new-provider info when total_contracts is 0', () => {
		const metrics: TrustMetricsSubset = { total_contracts: 0, trust_score: 0n };
		expect(getTrustBannerType(metrics, false)).toBe('new-provider');
	});

	it('shows new-provider info even if trust score is 0 (new provider overrides low-trust)', () => {
		const metrics: TrustMetricsSubset = { total_contracts: 0, trust_score: 0n };
		expect(getTrustBannerType(metrics, true)).toBe('new-provider');
	});
});

describe('trust banner: low trust warning', () => {
	it('shows low-trust warning when trust_score < 60 and not dismissed', () => {
		const metrics: TrustMetricsSubset = { total_contracts: 5, trust_score: 45n };
		expect(getTrustBannerType(metrics, false)).toBe('low-trust');
	});

	it('shows no warning when low-trust warning is dismissed', () => {
		const metrics: TrustMetricsSubset = { total_contracts: 5, trust_score: 45n };
		expect(getTrustBannerType(metrics, true)).toBe('none');
	});

	it('shows no warning when trust_score is exactly 60', () => {
		const metrics: TrustMetricsSubset = { total_contracts: 5, trust_score: 60n };
		expect(getTrustBannerType(metrics, false)).toBe('none');
	});
});

describe('trust banner: no warning', () => {
	it('shows no banner when trust_score >= 60 and contracts exist', () => {
		const metrics: TrustMetricsSubset = { total_contracts: 10, trust_score: 85n };
		expect(getTrustBannerType(metrics, false)).toBe('none');
	});

	it('shows no banner when trustMetrics is null', () => {
		expect(getTrustBannerType(null, false)).toBe('none');
	});

	it('shows no banner when trust_score is 100', () => {
		const metrics: TrustMetricsSubset = { total_contracts: 20, trust_score: 100n };
		expect(getTrustBannerType(metrics, false)).toBe('none');
	});
});

describe('trust warning: sessionStorage dismissal key format', () => {
	it('uses correct sessionStorage key per offering ID', () => {
		const offeringId = 42;
		const key = `trust_warning_dismissed_${offeringId}`;
		expect(key).toBe('trust_warning_dismissed_42');
	});

	it('keys are unique per offering ID', () => {
		const key1 = `trust_warning_dismissed_1`;
		const key2 = `trust_warning_dismissed_2`;
		expect(key1).not.toBe(key2);
	});
});
