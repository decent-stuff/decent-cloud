import { describe, it, expect, vi, beforeEach } from 'vitest';
import { fetchDashboardData } from './dashboard-data';
import { fetchPlatformStats } from './api';
import { fetchDctPrice } from './icp';

vi.mock('./api', () => ({
	fetchPlatformStats: vi.fn()
}));

vi.mock('./icp', () => ({
	fetchDctPrice: vi.fn()
}));

const mockedFetchPlatformStats = vi.mocked(fetchPlatformStats);
const mockedFetchDctPrice = vi.mocked(fetchDctPrice);

const mockStats = {
	total_providers: 12,
	active_providers: 10,
	total_offerings: 7,
	total_contracts: 5,
	total_transfers: 100,
	total_volume_e9s: 1_000_000_000,
	validator_count_24h: 6,
	current_block_validators: 2,
	total_blocks: 256,
	latest_block_timestamp_ns: 123_000,
	blocks_until_next_halving: 10_000,
	current_block_rewards_e9s: 9_000_000_000
};

describe('fetchDashboardData', () => {
	beforeEach(() => {
		vi.resetAllMocks();
	});

	it('combines stats with DCT price', async () => {
		mockedFetchPlatformStats.mockResolvedValue(mockStats);
		mockedFetchDctPrice.mockResolvedValue(1.23);

		const dashboard = await fetchDashboardData();

		expect(dashboard.dctPrice).toBe(1.23);
		expect(dashboard.providerCount).toBe(mockStats.total_providers);
		expect(dashboard.totalBlocks).toBe(mockStats.total_blocks);
		expect(dashboard.validatorCount).toBe(mockStats.current_block_validators);
		expect(dashboard.blockReward).toBeCloseTo(9);
	});

	it('propagates failures from the stats API', async () => {
		mockedFetchPlatformStats.mockRejectedValue(new Error('stats failed'));
		mockedFetchDctPrice.mockResolvedValue(0);

		await expect(fetchDashboardData()).rejects.toThrow('stats failed');
	});
});
