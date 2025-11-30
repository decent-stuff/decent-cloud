import { describe, it, expect } from 'vitest';
import { formatContractDate, formatContractPrice, truncateContractHash, computePubkey, formatDuration } from './contract-format';

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
});
