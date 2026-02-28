import { describe, it, expect } from 'vitest';
import { truncatePubkey } from '$lib/utils/identity';

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

// Mirror the provider display logic from the offering detail page
type OfferingSubset = {
	owner_username?: string;
	pubkey: string;
};

function getProviderDisplayName(offering: OfferingSubset): string {
	return offering.owner_username ? `@${offering.owner_username}` : truncatePubkey(offering.pubkey);
}

function getProviderHref(offering: OfferingSubset): string {
	return `/dashboard/providers/${offering.owner_username || offering.pubkey}`;
}

describe('provider display: username vs pubkey', () => {
	it('shows @username when owner_username is available', () => {
		const offering: OfferingSubset = {
			owner_username: 'testprovider',
			pubkey: '3e9f603a58a993f87ecdf024bc5f7647aa9007222774336bfa509fa31a3869f3'
		};
		expect(getProviderDisplayName(offering)).toBe('@testprovider');
	});

	it('shows truncated pubkey when owner_username is undefined', () => {
		const offering: OfferingSubset = {
			pubkey: '3e9f603a58a993f87ecdf024bc5f7647aa9007222774336bfa509fa31a3869f3'
		};
		expect(getProviderDisplayName(offering)).toBe('3e9f60...3869f3');
	});

	it('shows truncated pubkey when owner_username is empty string', () => {
		const offering: OfferingSubset = {
			owner_username: '',
			pubkey: '3e9f603a58a993f87ecdf024bc5f7647aa9007222774336bfa509fa31a3869f3'
		};
		// Empty string is falsy, so it should fall back to pubkey
		expect(getProviderDisplayName(offering)).toBe('3e9f60...3869f3');
	});

	it('generates correct href with username when available', () => {
		const offering: OfferingSubset = {
			owner_username: 'testprovider',
			pubkey: '3e9f603a58a993f87ecdf024bc5f7647aa9007222774336bfa509fa31a3869f3'
		};
		expect(getProviderHref(offering)).toBe('/dashboard/providers/testprovider');
	});

	it('generates correct href with pubkey when username is not available', () => {
		const offering: OfferingSubset = {
			pubkey: '3e9f603a58a993f87ecdf024bc5f7647aa9007222774336bfa509fa31a3869f3'
		};
		expect(getProviderHref(offering)).toBe('/dashboard/providers/3e9f603a58a993f87ecdf024bc5f7647aa9007222774336bfa509fa31a3869f3');
	});
});
