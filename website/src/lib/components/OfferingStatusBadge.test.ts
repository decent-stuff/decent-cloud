import { describe, it, expect } from 'vitest';

function getSubscriptionLabel(isSubscription: boolean, days?: number): string | null {
	if (!isSubscription) return null;
	if (!days) return "Recurring";
	if (days <= 31) return "Monthly";
	if (days <= 93) return "Quarterly";
	if (days <= 366) return "Yearly";
	return `${days}d`;
}

function getPrimaryStatus(
	providerOnline: boolean,
	isDemo: boolean,
	isReseller: boolean,
	resellerName?: string
): { label: string; color: string } | null {
	if (!providerOnline) {
		return { label: "Offline", color: "bg-red-500/20 text-red-400 border-red-500/30" };
	}
	if (isDemo) {
		return { label: "Demo", color: "bg-amber-500/20 text-amber-400 border-amber-500/30" };
	}
	if (isReseller && resellerName) {
		return { label: `Via ${resellerName}`, color: "bg-primary-500/20 text-primary-400 border-primary-500/30" };
	}
	return null;
}

function getTrustColor(score: number, hasCriticalFlags: boolean): string {
	if (hasCriticalFlags) return 'bg-red-500/20 text-red-400 border-red-500/30';
	if (score >= 80) return 'bg-green-500/20 text-green-400 border-green-500/30';
	if (score >= 60) return 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30';
	return 'bg-red-500/20 text-red-400 border-red-500/30';
}

describe('OfferingStatusBadge: primary status priority', () => {
	it('returns Offline when provider is offline', () => {
		const status = getPrimaryStatus(false, false, false);
		expect(status).toEqual({ label: "Offline", color: expect.stringContaining("red") });
	});

	it('returns Demo when offering is a demo and provider is online', () => {
		const status = getPrimaryStatus(true, true, false);
		expect(status).toEqual({ label: "Demo", color: expect.stringContaining("amber") });
	});

	it('returns reseller badge when via reseller and no higher priority status', () => {
		const status = getPrimaryStatus(true, false, true, "TestReseller");
		expect(status).toEqual({ label: "Via TestReseller", color: expect.stringContaining("primary") });
	});

	it('returns null when no special status', () => {
		const status = getPrimaryStatus(true, false, false);
		expect(status).toBeNull();
	});

	it('prioritizes offline over demo', () => {
		const status = getPrimaryStatus(false, true, false);
		expect(status?.label).toBe("Offline");
	});

	it('prioritizes demo over reseller', () => {
		const status = getPrimaryStatus(true, true, true, "Reseller");
		expect(status?.label).toBe("Demo");
	});

	it('returns null for reseller without name', () => {
		const status = getPrimaryStatus(true, false, true);
		expect(status).toBeNull();
	});
});

describe('OfferingStatusBadge: subscription label', () => {
	it('returns null when not a subscription', () => {
		expect(getSubscriptionLabel(false)).toBeNull();
	});

	it('returns Recurring when no interval days', () => {
		expect(getSubscriptionLabel(true)).toBe("Recurring");
		expect(getSubscriptionLabel(true, undefined)).toBe("Recurring");
	});

	it('returns Monthly for <= 31 days', () => {
		expect(getSubscriptionLabel(true, 1)).toBe("Monthly");
		expect(getSubscriptionLabel(true, 31)).toBe("Monthly");
	});

	it('returns Quarterly for 32-93 days', () => {
		expect(getSubscriptionLabel(true, 32)).toBe("Quarterly");
		expect(getSubscriptionLabel(true, 93)).toBe("Quarterly");
	});

	it('returns Yearly for 94-366 days', () => {
		expect(getSubscriptionLabel(true, 94)).toBe("Yearly");
		expect(getSubscriptionLabel(true, 365)).toBe("Yearly");
	});

	it('returns custom days for > 366', () => {
		expect(getSubscriptionLabel(true, 400)).toBe("400d");
	});
});

describe('OfferingStatusBadge: trust score colors', () => {
	it('returns green for score >= 80 without flags', () => {
		expect(getTrustColor(80, false)).toContain("green");
		expect(getTrustColor(100, false)).toContain("green");
	});

	it('returns yellow for score 60-79 without flags', () => {
		expect(getTrustColor(60, false)).toContain("yellow");
		expect(getTrustColor(79, false)).toContain("yellow");
	});

	it('returns red for score < 60 without flags', () => {
		expect(getTrustColor(59, false)).toContain("red");
		expect(getTrustColor(0, false)).toContain("red");
	});

	it('returns red for any score with critical flags', () => {
		expect(getTrustColor(100, true)).toContain("red");
		expect(getTrustColor(80, true)).toContain("red");
	});
});
