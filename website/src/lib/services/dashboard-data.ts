import { fetchPlatformStats } from './api';

export interface DashboardData {
	totalProviders: number;
	activeProviders: number;
	totalOfferings: number;
	totalContracts: number;
	activeValidators: number;
	totalTransfers: number;
	totalVolumeE9s: number;
}

export async function fetchDashboardData(): Promise<DashboardData> {
	const platformStats = await fetchPlatformStats();

	return {
		totalProviders: platformStats.total_providers,
		activeProviders: platformStats.active_providers,
		totalOfferings: platformStats.total_offerings,
		totalContracts: platformStats.total_contracts,
		activeValidators: platformStats.validator_count_24h,
		totalTransfers: platformStats.total_transfers,
		totalVolumeE9s: platformStats.total_volume_e9s
	};
}
