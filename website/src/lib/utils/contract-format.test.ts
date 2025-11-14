import { describe, it, expect } from 'vitest';
import { formatContractDate, formatContractPrice, truncateContractHash, computePubkey } from './contract-format';

describe('contract formatting helpers', () => {
	it('formats timestamps into readable strings', () => {
		const human = formatContractDate(1_700_000_000_000_000_000);
		expect(human).toMatch(/\d{1,2}\/\d{1,2}\/\d{4}/);
	});

	it('returns N/A when timestamp missing', () => {
		expect(formatContractDate(undefined)).toBe('N/A');
	});

	it('formats e9s amounts to ICP string', () => {
		expect(formatContractPrice(123_000_000_000)).toBe('123.00 ICP');
	});

	it('truncates hashes preserving start and end', () => {
		expect(truncateContractHash('abcdef123456')).toBe('abcdef123456');
		expect(truncateContractHash('abcdef1234567890', 4)).toBe('abcd...7890');
	});

	describe('computePubkey', () => {
		it('computes SHA256 hash of public key bytes', () => {
			const testPubkey = new Uint8Array([1, 2, 3, 4, 5]);
			const hash = computePubkey(testPubkey);

			// SHA256 should produce 64 hex characters (32 bytes)
			expect(hash).toHaveLength(64);
			expect(hash).toMatch(/^[0-9a-f]+$/);
		});

		it('produces consistent hash for same input', () => {
			const testPubkey = new Uint8Array([10, 20, 30, 40]);
			const hash1 = computePubkey(testPubkey);
			const hash2 = computePubkey(testPubkey);

			expect(hash1).toBe(hash2);
		});
	});
});
