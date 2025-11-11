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

function getMetadataNumber(metadata: Record<string, unknown>, key: string): number {
	const value = metadata[key];
	if (typeof value === 'number') {
		return value;
	}
	return 0;
}

export async function fetchDashboardData(): Promise<DashboardData> {
	const [platformStats, dctPrice] = await Promise.all([fetchPlatformStats(), fetchDctPrice()]);

	const numBlocks = getMetadataNumber(platformStats.metadata, 'ledger:num_blocks');
	const blocksUntilHalving = getMetadataNumber(platformStats.metadata, 'ledger:blocks_until_next_halving');
	const rewardPerBlockE9 = getMetadataNumber(platformStats.metadata, 'ledger:reward_per_block_e9s');
	const accumulatedRewardsE9 = getMetadataNumber(platformStats.metadata, 'ledger:current_block_rewards_e9s');

	return {
		dctPrice,
		providerCount: platformStats.total_providers,
		totalBlocks: numBlocks,
		blocksUntilHalving,
		rewardPerBlock: rewardPerBlockE9 / E9S_TO_DCT,
		accumulatedRewards: accumulatedRewardsE9 / E9S_TO_DCT
	};
}
