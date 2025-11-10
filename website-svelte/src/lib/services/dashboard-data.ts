import { fetchPlatformStats } from './api';
import { fetchDctPrice } from './icp';

export interface DashboardData {
	dctPrice: number;
	providerCount: number;
	totalBlocks: number;
	blocksUntilHalving: number;
	rewardPerBlock: number;
	accumulatedRewards: number;
}

const E9S_TO_DCT = 1_000_000_000;

export async function fetchDashboardData(): Promise<DashboardData> {
	const [platformStats, dctPrice] = await Promise.all([fetchPlatformStats(), fetchDctPrice()]);

	return {
		dctPrice,
		providerCount: platformStats.total_providers,
		totalBlocks: platformStats.total_blocks,
		blocksUntilHalving: platformStats.blocks_until_next_halving,
		rewardPerBlock: platformStats.reward_per_block_e9s / E9S_TO_DCT,
		accumulatedRewards: platformStats.current_block_rewards_e9s / E9S_TO_DCT
	};
}
