<script lang="ts">
	import { onMount } from 'svelte';
	import { fetchDashboardData, type DashboardData } from '$lib/services/dashboard-data';
	import Header from '$lib/components/Header.svelte';
	import HeroSection from '$lib/components/HeroSection.svelte';
	import FeaturesSection from '$lib/components/FeaturesSection.svelte';
	import BenefitsSection from '$lib/components/BenefitsSection.svelte';
	import DashboardSection from '$lib/components/DashboardSection.svelte';
	import InfoSection from '$lib/components/InfoSection.svelte';
	import Footer from '$lib/components/Footer.svelte';

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
			dashboardData = await fetchDashboardData();
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
