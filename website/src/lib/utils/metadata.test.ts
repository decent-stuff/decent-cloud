import { describe, it, expect } from 'vitest';
import { getMetadataValue, type Metadata } from './metadata.ts';

describe('metadata utils', () => {
	it('extracts numeric values and handles reward scaling', () => {
		const metadata: Metadata = [
			['ledger:total_providers', { Nat: 123n }],
			['ledger:current_block_rewards_e9s', { Nat: 5_000_000_000n }],
			['ledger:current_block_validators', { Int: 12n }]
		];

		expect(getMetadataValue(metadata, 'ledger:total_providers')).toBe(123);
		expect(getMetadataValue(metadata, 'ledger:current_block_rewards_e9s')).toBe(5);
		expect(getMetadataValue(metadata, 'ledger:current_block_validators')).toBe(12);
	});

	it('returns 0 when the key is missing', () => {
		const metadata: Metadata = [['ledger:num_blocks', { Nat: 42n }]];

		expect(getMetadataValue(metadata, 'ledger:unknown')).toBe(0);
	});
});
