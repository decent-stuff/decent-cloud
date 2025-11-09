<script lang="ts">
	import { onMount } from 'svelte';
	import { fetchMetadata, fetchDctPrice } from '$lib/services/icp';
	import { getMetadataValue } from '$lib/utils/metadata.ts';
	import Header from '$lib/components/Header.svelte';
	import HeroSection from '$lib/components/HeroSection.svelte';
	import FeaturesSection from '$lib/components/FeaturesSection.svelte';
	import BenefitsSection from '$lib/components/BenefitsSection.svelte';
	import DashboardSection from '$lib/components/DashboardSection.svelte';
	import InfoSection from '$lib/components/InfoSection.svelte';
	import Footer from '$lib/components/Footer.svelte';

	interface DashboardData {
		dctPrice: number;
		providerCount: number;
		totalBlocks: number;
		blocksUntilHalving: number;
		validatorCount: number;
		blockReward: number;
	}

	let dashboardData: DashboardData = {
		dctPrice: 0,
		providerCount: 0,
		totalBlocks: 0,
		blocksUntilHalving: 0,
		validatorCount: 0,
		blockReward: 0
	};

	async function loadDashboardData() {
		try {
			const [metadata, dctPrice] = await Promise.all([fetchMetadata(), fetchDctPrice()]);

			dashboardData = {
				dctPrice,
				providerCount: getMetadataValue(metadata, 'ledger:total_providers'),
				totalBlocks: getMetadataValue(metadata, 'ledger:num_blocks'),
				blocksUntilHalving: getMetadataValue(metadata, 'ledger:blocks_until_next_halving'),
				validatorCount: getMetadataValue(metadata, 'ledger:current_block_validators'),
				blockReward: getMetadataValue(metadata, 'ledger:current_block_rewards_e9s')
			};
		} catch (err) {
			console.error('Error fetching dashboard data:', err);
		}
	}

	onMount(() => {
		loadDashboardData();
		const interval = setInterval(loadDashboardData, 10000);
		return () => clearInterval(interval);
	});
</script>

<div class="min-h-screen bg-gradient-to-b from-gray-900 via-blue-900 to-purple-900 text-white">
	<Header />
	<HeroSection />
	<FeaturesSection />
	<BenefitsSection />
	<DashboardSection {dashboardData} />
	<InfoSection />
	<Footer />
</div>
