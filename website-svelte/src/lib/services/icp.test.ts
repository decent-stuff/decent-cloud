import { describe, it, expect, vi, beforeEach } from 'vitest';
import { fetchDctPrice, fetchMetadata } from './icp';

describe('fetchDctPrice', () => {
	beforeEach(() => {
		vi.resetAllMocks();
	});

	it('should fetch and parse DCT price from KongSwap API', async () => {
		const mockResponse = {
			items: [
				{
					metrics: {
						price: 1.23456789
					}
				}
			]
		};

		global.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => mockResponse
		});

		const price = await fetchDctPrice();

		expect(price).toBe(1.23456789);
		expect(global.fetch).toHaveBeenCalledWith(
			'https://api.kongswap.io/api/tokens/by_canister',
			expect.objectContaining({
				method: 'POST',
				headers: {
					'Content-Type': 'application/json'
				},
				body: expect.stringContaining('ggi4a-wyaaa-aaaai-actqq-cai')
			})
		);
	});

	it('should parse string price values correctly', async () => {
		const mockResponse = {
			items: [
				{
					metrics: {
						price: '2.5'
					}
				}
			]
		};

		global.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => mockResponse
		});

		const price = await fetchDctPrice();

		expect(price).toBe(2.5);
	});

	it('should handle price with commas', async () => {
		const mockResponse = {
			items: [
				{
					metrics: {
						price: '1,234.56'
					}
				}
			]
		};

		global.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => mockResponse
		});

		const price = await fetchDctPrice();

		expect(price).toBe(1234.56);
	});

	it('should return 0 when API returns HTTP error', async () => {
		global.fetch = vi.fn().mockResolvedValue({
			ok: false,
			status: 500
		});

		const price = await fetchDctPrice();

		expect(price).toBe(0);
	});

	it('should return 0 when API returns empty items', async () => {
		global.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({ items: [] })
		});

		const price = await fetchDctPrice();

		expect(price).toBe(0);
	});

	it('should return 0 when fetch throws network error', async () => {
		global.fetch = vi.fn().mockRejectedValue(new Error('Network error'));

		const price = await fetchDctPrice();

		expect(price).toBe(0);
	});

	it('should return 0 when price is null', async () => {
		const mockResponse = {
			items: [
				{
					metrics: {
						price: null
					}
				}
			]
		};

		global.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => mockResponse
		});

		const price = await fetchDctPrice();

		expect(price).toBe(0);
	});

	it('should return 0 when price is undefined', async () => {
		const mockResponse = {
			items: [
				{
					metrics: {}
				}
			]
		};

		global.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => mockResponse
		});

		const price = await fetchDctPrice();

		expect(price).toBe(0);
	});
});
