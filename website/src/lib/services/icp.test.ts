import { beforeEach, describe, expect, it, vi } from 'vitest';

const {
	mockFetchRootKey,
	mockMetadata,
	mockCreateActor,
	mockCreateSync
} = vi.hoisted(() => ({
	mockFetchRootKey: vi.fn(),
	mockMetadata: vi.fn(),
	mockCreateActor: vi.fn(),
	mockCreateSync: vi.fn()
}));

vi.mock('@dfinity/agent', () => {
	mockCreateSync.mockImplementation(() => ({
		fetchRootKey: mockFetchRootKey
	}));
	mockCreateActor.mockImplementation(() => ({
		metadata: mockMetadata
	}));

	return {
		HttpAgent: {
			createSync: mockCreateSync
		},
		Actor: {
			createActor: mockCreateActor
		}
	};
});

import { fetchDctPrice } from './icp';

describe('fetchDctPrice', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		mockFetchRootKey.mockResolvedValue(undefined);
	});

	it('reads DCT USD price from canister metadata e6 nat value', async () => {
		mockMetadata.mockResolvedValue([
			['ledger:token_value_in_usd_e6', { Nat: 20_385n }]
		]);

		const price = await fetchDctPrice();

		expect(price).toBe(0.020385);
		expect(mockCreateSync).toHaveBeenCalled();
		expect(mockCreateActor).toHaveBeenCalled();
		expect(mockMetadata).toHaveBeenCalledTimes(1);
	});

	it('returns 0 when metadata is missing token value key', async () => {
		mockMetadata.mockResolvedValue([['icrc1:name', { Text: 'Decent Cloud' }]]);

		const price = await fetchDctPrice();

		expect(price).toBe(0);
	});

	it('returns 0 when token value is non-positive', async () => {
		mockMetadata.mockResolvedValue([
			['ledger:token_value_in_usd_e6', { Nat: 0n }]
		]);

		const price = await fetchDctPrice();

		expect(price).toBe(0);
	});

});
