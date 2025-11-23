import { describe, it, expect, vi, beforeEach } from 'vitest';
import { fetchDashboardData } from './dashboard-data';
import { fetchPlatformStats } from './api';

vi.mock('./api', () => ({
	fetchPlatformStats: vi.fn()
}));

const mockedFetchPlatformStats = vi.mocked(fetchPlatformStats);

const mockStats = {
	total_providers: 12,
	active_providers: 10,
	total_offerings: 7,
	total_contracts: 5,
	total_transfers: 100,
	total_volume_e9s: 1_000_000_000,
	validator_count_24h: 6,
	latest_block_timestamp_ns: 123_000,
	metadata: {
		'ledger:num_blocks': 256,
		'ledger:blocks_until_next_halving': 10_000,
		'ledger:reward_per_block_e9s': 50_000_000_000,
		'ledger:current_block_rewards_e9s': 12_800_000_000_000,
		'ledger:current_block_validators': 2,
		'ledger:token_value_in_usd_e6': 1_000_000
	}
};

describe('fetchDashboardData', () => {
	beforeEach(() => {
		vi.resetAllMocks();
	});

	it('returns marketplace statistics from platform stats', async () => {
		mockedFetchPlatformStats.mockResolvedValue(mockStats);

		const dashboard = await fetchDashboardData();

		expect(dashboard.totalProviders).toBe(mockStats.total_providers);
		expect(dashboard.activeProviders).toBe(mockStats.active_providers);
		expect(dashboard.totalOfferings).toBe(mockStats.total_offerings);
		expect(dashboard.totalContracts).toBe(mockStats.total_contracts);
		expect(dashboard.activeValidators).toBe(mockStats.validator_count_24h);
	});

	it('propagates failures from the stats API', async () => {
		mockedFetchPlatformStats.mockRejectedValue(new Error('stats failed'));

		await expect(fetchDashboardData()).rejects.toThrow('stats failed');
	});
});
