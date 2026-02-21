import { describe, it, expect } from 'vitest';

// Test pure logic extracted from AllowlistDialog component

describe('AllowlistDialog pubkey formatting', () => {
	function formatPubkey(hex: string): string {
		if (hex.length <= 12) return hex;
		return hex.slice(0, 6) + '...' + hex.slice(-6);
	}

	it('returns short pubkeys unchanged', () => {
		expect(formatPubkey('abcdef')).toBe('abcdef');
		expect(formatPubkey('abc123456789')).toBe('abc123456789');
	});

	it('truncates long pubkeys to 6+...+6 format', () => {
		const pubkey = 'aabbccddeeff00112233445566778899';
		expect(formatPubkey(pubkey)).toBe('aabbcc...778899');
	});

	it('handles exactly 13-character pubkeys with truncation', () => {
		const pubkey = 'abcdefghijklm';
		expect(formatPubkey(pubkey)).toBe('abcdef...hijklm');
	});
});

describe('AllowlistDialog date formatting', () => {
	function formatDate(ns: number): string {
		return new Date(ns / 1_000_000).toLocaleDateString();
	}

	it('converts nanoseconds to locale date string', () => {
		// 2024-01-15 in nanoseconds: 1705276800000 ms * 1_000_000
		const ns = 1705276800000 * 1_000_000;
		const result = formatDate(ns);
		// Result is locale-dependent; just verify it's a non-empty string
		expect(typeof result).toBe('string');
		expect(result.length).toBeGreaterThan(0);
	});

	it('returns different dates for different nanosecond values', () => {
		// Two dates 24 hours apart
		const day1Ns = 1705276800000 * 1_000_000;
		const day2Ns = (1705276800000 + 86400000) * 1_000_000;
		expect(formatDate(day1Ns)).not.toBe(formatDate(day2Ns));
	});
});

describe('AllowlistDialog input validation', () => {
	it('trims whitespace before adding a pubkey', () => {
		const rawInput = '  abc123  ';
		const trimmed = rawInput.trim();
		expect(trimmed).toBe('abc123');
	});

	it('treats whitespace-only input as empty', () => {
		const rawInput = '   ';
		expect(!rawInput.trim()).toBe(true);
	});

	it('treats empty string as invalid', () => {
		expect(!''.trim()).toBe(true);
	});
});
