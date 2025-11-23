import { describe, it, expect, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { dashboardStore } from './dashboard';
import { fetchDashboardData } from '../services/dashboard-data';

vi.mock('../services/dashboard-data', () => ({
	fetchDashboardData: vi.fn()
}));

const mockedFetchDashboardData = vi.mocked(fetchDashboardData);

const mockDashboardData = {
	totalProviders: 12,
	activeProviders: 10,
	totalOfferings: 7,
	totalContracts: 5,
	activeValidators: 6
};

describe('dashboardStore', () => {
	beforeEach(() => {
		vi.resetAllMocks();
	});

	it('initializes with default data', () => {
		const data = get(dashboardStore.data);
		expect(data.totalProviders).toBe(0);
		expect(data.activeProviders).toBe(0);
		expect(data.totalOfferings).toBe(0);
		expect(get(dashboardStore.error)).toBeNull();
		expect(get(dashboardStore.isLoading)).toBe(false);
	});

	it('loads dashboard data successfully', async () => {
		mockedFetchDashboardData.mockResolvedValue(mockDashboardData);

		await dashboardStore.load();

		expect(get(dashboardStore.data)).toEqual(mockDashboardData);
		expect(get(dashboardStore.error)).toBeNull();
		expect(get(dashboardStore.isLoading)).toBe(false);
	});

	it('sets error when load fails', async () => {
		const error = new Error('Failed to fetch');
		mockedFetchDashboardData.mockRejectedValue(error);

		await dashboardStore.load();

		expect(get(dashboardStore.error)).toBe('Failed to fetch');
		expect(get(dashboardStore.isLoading)).toBe(false);
	});

	it('sets isLoading during load', async () => {
		let resolvePromise: (value: typeof mockDashboardData) => void;
		const promise = new Promise<typeof mockDashboardData>((resolve) => {
			resolvePromise = resolve;
		});
		mockedFetchDashboardData.mockReturnValue(promise);

		const loadPromise = dashboardStore.load();
		expect(get(dashboardStore.isLoading)).toBe(true);

		resolvePromise!(mockDashboardData);
		await loadPromise;

		expect(get(dashboardStore.isLoading)).toBe(false);
	});
});
