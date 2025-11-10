import { describe, it, expect, vi, afterEach } from 'vitest';
import { fetchPlatformStats } from './api';

const sampleStats = {
	total_providers: 10,
	active_providers: 8,
	total_offerings: 5,
	total_contracts: 3,
	total_transfers: 12,
	total_volume_e9s: 1_500_000_000,
	validator_count_24h: 4,
	current_block_validators: 3,
	total_blocks: 42,
	latest_block_timestamp_ns: 123_456_789,
	blocks_until_next_halving: 210_000,
	current_block_rewards_e9s: 5_000_000_000
};

describe('fetchPlatformStats', () => {
	afterEach(() => {
		vi.restoreAllMocks();
	});

	it('returns stats payload when API succeeds', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({ success: true, data: sampleStats })
		});

		const stats = await fetchPlatformStats();

		expect(stats).toEqual(sampleStats);
		expect(global.fetch).toHaveBeenCalledWith(expect.stringContaining('/api/v1/stats'));
	});

	it('throws when response is not ok', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: false,
			status: 500,
			statusText: 'error'
		});

		await expect(fetchPlatformStats()).rejects.toThrow('Failed to fetch platform stats');
	});

	it('throws when API reports failure', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({ success: false, error: 'boom' })
		});

		await expect(fetchPlatformStats()).rejects.toThrow('boom');
	});

	it('throws when data is missing', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({ success: true })
		});

		await expect(fetchPlatformStats()).rejects.toThrow('did not include data');
	});
});
