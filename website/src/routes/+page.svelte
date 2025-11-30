<script lang="ts">
	import { onMount } from 'svelte';
	import { browser } from '$app/environment';
	import { dashboardStore } from '$lib/stores/dashboard';
	import Header from '$lib/components/Header.svelte';
	import HeroSection from '$lib/components/HeroSection.svelte';
	import FeaturesSection from '$lib/components/FeaturesSection.svelte';
	import TrustGuaranteesSection from '$lib/components/TrustGuaranteesSection.svelte';
	import BenefitsSection from '$lib/components/BenefitsSection.svelte';
	import AIFeaturesSection from '$lib/components/AIFeaturesSection.svelte';
	import DashboardSection from '$lib/components/DashboardSection.svelte';
	import InfoSection from '$lib/components/InfoSection.svelte';
	import Footer from '$lib/components/Footer.svelte';
	import type { DashboardData } from '$lib/services/dashboard-data';

	let dashboardData = $state<DashboardData>({
		totalProviders: 0,
		activeProviders: 0,
		totalOfferings: 0,
		totalContracts: 0,
		activeValidators: 0
	});
	let error = $state<string | null>(null);

	onMount(() => {
		if (!browser) return;

		const unsubscribeData = dashboardStore.data.subscribe((value) => {
			dashboardData = value;
		});
		const unsubscribeError = dashboardStore.error.subscribe((value) => {
			error = value;
		});

		dashboardStore.load();
		const interval = setInterval(() => dashboardStore.load(), 10000);

		return () => {
			unsubscribeData();
			unsubscribeError();
			clearInterval(interval);
		};
	});
</script>

<div class="min-h-screen bg-gradient-to-b from-gray-900 via-blue-900 to-purple-900 text-white">
	<Header />
	<HeroSection />
	<FeaturesSection />
	<TrustGuaranteesSection />
	<BenefitsSection />
	<AIFeaturesSection />
	<DashboardSection {dashboardData} {error} />
	<InfoSection />
	<Footer />
</div>
