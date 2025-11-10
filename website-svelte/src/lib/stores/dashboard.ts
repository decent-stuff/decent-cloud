import { writable } from 'svelte/store';
import { fetchDashboardData, type DashboardData } from '../services/dashboard-data';

const defaultData: DashboardData = {
	dctPrice: 0,
	providerCount: 0,
	totalBlocks: 0,
	blocksUntilHalving: 0,
	rewardPerBlock: 0,
	accumulatedRewards: 0
};

function createDashboardStore() {
	const data = writable<DashboardData>(defaultData);
	const error = writable<string | null>(null);
	const isLoading = writable(false);

	async function load() {
		isLoading.set(true);
		error.set(null);
		try {
			const result = await fetchDashboardData();
			data.set(result);
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Failed to load dashboard data';
			error.set(errorMessage);
			console.error('Error fetching dashboard data:', err);
		} finally {
			isLoading.set(false);
		}
	}

	return {
		data: { subscribe: data.subscribe },
		error: { subscribe: error.subscribe },
		isLoading: { subscribe: isLoading.subscribe },
		load
	};
}

export const dashboardStore = createDashboardStore();
