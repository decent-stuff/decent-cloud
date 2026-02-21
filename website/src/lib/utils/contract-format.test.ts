import { describe, it, expect, vi, afterEach } from 'vitest';
import { formatContractDate, formatContractPrice, truncateContractHash, computePubkey, formatDuration, formatRelativeTime, formatTimeRemaining, getProvisioningElapsed, isProvisioningStuck } from './contract-format';

describe('contract formatting helpers', () => {
	it('formats timestamps into readable strings', () => {
		const human = formatContractDate(1_700_000_000_000_000_000);
		expect(human).toMatch(/\d{1,2}\/\d{1,2}\/\d{4}/);
	});

	it('returns N/A when timestamp missing', () => {
		expect(formatContractDate(undefined)).toBe('N/A');
	});

	it('formats e9s amounts to ICP string', () => {
		expect(formatContractPrice(123_000_000_000, "ICP")).toBe('123.00 ICP');
	});

	it('truncates hashes preserving start and end', () => {
		expect(truncateContractHash('abcdef123456')).toBe('abcdef123456');
		expect(truncateContractHash('abcdef1234567890', 4)).toBe('abcd...7890');
	});

	describe('computePubkey', () => {
		it('converts public key bytes to hex string', () => {
			const testPubkey = new Uint8Array([1, 2, 3, 4, 5]);
			const hex = computePubkey(testPubkey);

			// Should produce 10 hex characters (5 bytes * 2 hex chars per byte)
			expect(hex).toBe('0102030405');
			expect(hex).toHaveLength(10);
			expect(hex).toMatch(/^[0-9a-f]+$/);
		});

		it('produces consistent result for same input', () => {
			const testPubkey = new Uint8Array([10, 20, 30, 40]);
			const hex1 = computePubkey(testPubkey);
			const hex2 = computePubkey(testPubkey);

			expect(hex1).toBe(hex2);
			expect(hex1).toBe('0a141e28');
		});

		it('handles Ed25519 public key (32 bytes)', () => {
			const ed25519Pubkey = new Uint8Array(32).fill(0xff);
			const hex = computePubkey(ed25519Pubkey);

			// Ed25519 public key is 32 bytes = 64 hex characters
			expect(hex).toHaveLength(64);
			expect(hex).toBe('f'.repeat(64));
		});
	});

	describe('formatRelativeTime', () => {
		const NOW_MS = 1_700_000_000_000;
		const NOW_NS = NOW_MS * 1_000_000;

		afterEach(() => {
			vi.useRealTimers();
		});

		it('returns "never" for null', () => {
			expect(formatRelativeTime(null)).toBe('never');
		});

		it('returns "just now" for future timestamps', () => {
			vi.setSystemTime(NOW_MS);
			expect(formatRelativeTime(NOW_NS + 5_000_000_000)).toBe('just now');
		});

		it('returns "just now" for 30 seconds ago', () => {
			vi.setSystemTime(NOW_MS);
			const thirtySecondsAgoNs = NOW_NS - 30 * 1_000_000_000;
			expect(formatRelativeTime(thirtySecondsAgoNs)).toBe('just now');
		});

		it('returns minutes for 5 minutes ago', () => {
			vi.setSystemTime(NOW_MS);
			const fiveMinAgoNs = NOW_NS - 5 * 60 * 1_000_000_000;
			expect(formatRelativeTime(fiveMinAgoNs)).toBe('5m ago');
		});

		it('returns hours for 2 hours ago', () => {
			vi.setSystemTime(NOW_MS);
			const twoHoursAgoNs = NOW_NS - 2 * 60 * 60 * 1_000_000_000;
			expect(formatRelativeTime(twoHoursAgoNs)).toBe('2h ago');
		});

		it('returns days for 3 days ago', () => {
			vi.setSystemTime(NOW_MS);
			const threeDaysAgoNs = NOW_NS - 3 * 24 * 60 * 60 * 1_000_000_000;
			expect(formatRelativeTime(threeDaysAgoNs)).toBe('3d ago');
		});

		it('returns months for 2 months ago', () => {
			vi.setSystemTime(NOW_MS);
			const twoMonthsAgoNs = NOW_NS - 60 * 24 * 60 * 60 * 1_000_000_000;
			expect(formatRelativeTime(twoMonthsAgoNs)).toBe('2mo ago');
		});
	});

	describe('formatDuration', () => {
		it('formats sub-hour durations in minutes', () => {
			const thirtyMinNs = 30 * 60 * 1_000_000_000;
			expect(formatDuration(thirtyMinNs)).toBe('30.0min');
		});

		it('formats hour durations', () => {
			const twoHoursNs = 2 * 60 * 60 * 1_000_000_000;
			expect(formatDuration(twoHoursNs)).toBe('2.0h');
		});

		it('formats day durations', () => {
			const threeDaysNs = 3 * 24 * 60 * 60 * 1_000_000_000;
			expect(formatDuration(threeDaysNs)).toBe('3.0d');
		});

		it('formats fractional minutes clearly', () => {
			const ninetySecondsNs = 90 * 1_000_000_000;
			expect(formatDuration(ninetySecondsNs)).toBe('1.5min');
		});
	});

	describe('getProvisioningElapsed', () => {
		const NOW_MS = 1_700_000_000_000;
		const NOW_NS = NOW_MS * 1_000_000;

		afterEach(() => {
			vi.useRealTimers();
		});

		it('returns null for non-provisioning statuses', () => {
			vi.setSystemTime(NOW_MS);
			expect(getProvisioningElapsed('active', NOW_NS - 5 * 60 * 1_000_000_000)).toBeNull();
			expect(getProvisioningElapsed('cancelled', NOW_NS)).toBeNull();
			expect(getProvisioningElapsed('requested', NOW_NS)).toBeNull();
		});

		it('returns elapsed string for provisioning status using status_updated_at_ns', () => {
			vi.setSystemTime(NOW_MS);
			const twoMinAgoNs = NOW_NS - 2 * 60 * 1_000_000_000;
			const result = getProvisioningElapsed('provisioning', NOW_NS - 10 * 60 * 1_000_000_000, twoMinAgoNs);
			expect(result).toBe('2m ago');
		});

		it('falls back to created_at_ns when status_updated_at_ns is absent', () => {
			vi.setSystemTime(NOW_MS);
			const fiveMinAgoNs = NOW_NS - 5 * 60 * 1_000_000_000;
			const result = getProvisioningElapsed('pending', fiveMinAgoNs);
			expect(result).toBe('5m ago');
		});

		it('returns elapsed string for accepted status', () => {
			vi.setSystemTime(NOW_MS);
			const oneHourAgoNs = NOW_NS - 60 * 60 * 1_000_000_000;
			const result = getProvisioningElapsed('accepted', NOW_NS, oneHourAgoNs);
			expect(result).toBe('1h ago');
		});
	});

	describe('isProvisioningStuck', () => {
		const NOW_MS = 1_700_000_000_000;
		const NOW_NS = NOW_MS * 1_000_000;

		afterEach(() => {
			vi.useRealTimers();
		});

		it('returns false for non-provisioning statuses regardless of age', () => {
			vi.setSystemTime(NOW_MS);
			const oldNs = NOW_NS - 60 * 60 * 1_000_000_000; // 1 hour ago
			expect(isProvisioningStuck('active', oldNs)).toBe(false);
			expect(isProvisioningStuck('requested', oldNs)).toBe(false);
		});

		it('returns false for provisioning contract under 30 minutes old', () => {
			vi.setSystemTime(NOW_MS);
			const tenMinAgoNs = NOW_NS - 10 * 60 * 1_000_000_000;
			expect(isProvisioningStuck('provisioning', tenMinAgoNs)).toBe(false);
		});

		it('returns true for provisioning contract over 30 minutes old', () => {
			vi.setSystemTime(NOW_MS);
			const fortyMinAgoNs = NOW_NS - 40 * 60 * 1_000_000_000;
			expect(isProvisioningStuck('provisioning', NOW_NS, fortyMinAgoNs)).toBe(true);
		});

		it('uses status_updated_at_ns over created_at_ns for stuck detection', () => {
			vi.setSystemTime(NOW_MS);
			// created_at_ns is old but status_updated_at_ns is recent (just transitioned)
			const oldCreatedNs = NOW_NS - 2 * 60 * 60 * 1_000_000_000;
			const recentStatusNs = NOW_NS - 5 * 60 * 1_000_000_000;
			expect(isProvisioningStuck('provisioning', oldCreatedNs, recentStatusNs)).toBe(false);
		});
	});

	describe('formatTimeRemaining', () => {
		afterEach(() => {
			vi.useRealTimers();
		});

		it('returns null when no end timestamp', () => {
			expect(formatTimeRemaining(undefined)).toBeNull();
		});

		it('returns null for past timestamps', () => {
			const pastNs = (Date.now() - 1000) * 1_000_000;
			expect(formatTimeRemaining(pastNs)).toBeNull();
		});

		it('shows hours for < 24 hours remaining with critical urgency', () => {
			vi.setSystemTime(1_700_000_000_000);
			const futureNs = (1_700_000_000_000 + 5 * 60 * 60 * 1000) * 1_000_000;
			const result = formatTimeRemaining(futureNs);
			expect(result).not.toBeNull();
			expect(result!.text).toContain('5h');
			expect(result!.urgency).toBe('critical');
		});

		it('shows days for 3 days remaining with warning urgency', () => {
			vi.setSystemTime(1_700_000_000_000);
			const futureNs = (1_700_000_000_000 + 3 * 24 * 60 * 60 * 1000) * 1_000_000;
			const result = formatTimeRemaining(futureNs);
			expect(result).not.toBeNull();
			expect(result!.text).toContain('3d');
			expect(result!.urgency).toBe('warning');
		});

		it('shows normal urgency for > 7 days remaining', () => {
			vi.setSystemTime(1_700_000_000_000);
			const futureNs = (1_700_000_000_000 + 10 * 24 * 60 * 60 * 1000) * 1_000_000;
			const result = formatTimeRemaining(futureNs);
			expect(result).not.toBeNull();
			expect(result!.urgency).toBe('normal');
		});
	});
});
