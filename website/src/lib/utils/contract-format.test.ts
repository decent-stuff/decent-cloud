import { describe, it, expect } from 'vitest';
import { formatContractDate, formatContractPrice, truncateContractHash } from './contract-format';

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
});
