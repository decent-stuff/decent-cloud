import { fetchPlatformStats } from './api';

export interface DashboardData {
	totalProviders: number;
	activeProviders: number;
	totalOfferings: number;
	totalContracts: number;
	activeValidators: number;
}

export async function fetchDashboardData(): Promise<DashboardData> {
	const platformStats = await fetchPlatformStats();

	return {
		totalProviders: platformStats.total_providers,
		activeProviders: platformStats.active_providers,
		totalOfferings: platformStats.total_offerings,
		totalContracts: platformStats.total_contracts,
		activeValidators: platformStats.validator_count_24h
	};
}
