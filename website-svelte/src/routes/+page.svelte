<script lang="ts">
	import { onMount } from 'svelte';
	import { fetchMetadata } from '$lib/services/icp';
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
			const metadata = await fetchMetadata();

			const getValue = (key: string): number => {
				const entry = metadata.find(([k]: [string, any]) => k === key);
				if (!entry) return 0;
				const value = entry[1];
				if ('Nat' in value) {
					const num = Number(value.Nat);
					if (key === 'ledger:token_value_in_usd_e6') return num / 1_000_000;
					if (key === 'ledger:current_block_rewards_e9s') return num / 1_000_000_000;
					return num;
				}
				if ('Int' in value) return Number(value.Int);
				return 0;
			};

			dashboardData = {
				dctPrice: getValue('ledger:token_value_in_usd_e6'),
				providerCount: getValue('ledger:total_providers'),
				totalBlocks: getValue('ledger:num_blocks'),
				blocksUntilHalving: getValue('ledger:blocks_until_next_halving'),
				validatorCount: getValue('ledger:current_block_validators'),
				blockReward: getValue('ledger:current_block_rewards_e9s')
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
	<HeroSection />
	<FeaturesSection />
	<BenefitsSection />
	<DashboardSection {dashboardData} />
	<InfoSection />
	<Footer />
</div>
