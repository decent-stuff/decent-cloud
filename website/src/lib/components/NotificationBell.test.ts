import { describe, it, expect } from 'vitest';
import { formatNotificationTime } from '$lib/utils/notification-time';

const NOW_MS = 1_700_000_000_000; // fixed reference point
const toNs = (ms: number) => ms * 1_000_000;

describe('formatNotificationTime: seconds range', () => {
	it('returns "0s ago" for a just-created notification', () => {
		expect(formatNotificationTime(toNs(NOW_MS), NOW_MS)).toBe('0s ago');
	});

	it('returns "30s ago" for 30 seconds ago', () => {
		expect(formatNotificationTime(toNs(NOW_MS - 30_000), NOW_MS)).toBe('30s ago');
	});

	it('returns "59s ago" for 59 seconds ago', () => {
		expect(formatNotificationTime(toNs(NOW_MS - 59_000), NOW_MS)).toBe('59s ago');
	});
});

describe('formatNotificationTime: minutes range', () => {
	it('returns "1m ago" for exactly 60 seconds ago', () => {
		expect(formatNotificationTime(toNs(NOW_MS - 60_000), NOW_MS)).toBe('1m ago');
	});

	it('returns "2m ago" for 2 minutes ago', () => {
		expect(formatNotificationTime(toNs(NOW_MS - 2 * 60_000), NOW_MS)).toBe('2m ago');
	});

	it('returns "59m ago" for 59 minutes ago', () => {
		expect(formatNotificationTime(toNs(NOW_MS - 59 * 60_000), NOW_MS)).toBe('59m ago');
	});
});

describe('formatNotificationTime: hours range', () => {
	it('returns "1h ago" for exactly 1 hour ago', () => {
		expect(formatNotificationTime(toNs(NOW_MS - 3600_000), NOW_MS)).toBe('1h ago');
	});

	it('returns "3h ago" for 3 hours ago', () => {
		expect(formatNotificationTime(toNs(NOW_MS - 3 * 3600_000), NOW_MS)).toBe('3h ago');
	});

	it('returns "23h ago" for 23 hours ago', () => {
		expect(formatNotificationTime(toNs(NOW_MS - 23 * 3600_000), NOW_MS)).toBe('23h ago');
	});
});

describe('formatNotificationTime: days range', () => {
	it('returns "1d ago" for exactly 24 hours ago', () => {
		expect(formatNotificationTime(toNs(NOW_MS - 24 * 3600_000), NOW_MS)).toBe('1d ago');
	});

	it('returns "3d ago" for 3 days ago', () => {
		expect(formatNotificationTime(toNs(NOW_MS - 3 * 24 * 3600_000), NOW_MS)).toBe('3d ago');
	});

	it('returns "7d ago" for 7 days ago', () => {
		expect(formatNotificationTime(toNs(NOW_MS - 7 * 24 * 3600_000), NOW_MS)).toBe('7d ago');
	});
});
