<script lang="ts">
	import { onMount } from 'svelte';
	import { browser } from '$app/environment';
	import { dashboardStore } from '$lib/stores/dashboard';
	import Header from '$lib/components/Header.svelte';
	import HeroSection from '$lib/components/HeroSection.svelte';
	import TrustGuaranteesSection from '$lib/components/TrustGuaranteesSection.svelte';
	import DashboardSection from '$lib/components/DashboardSection.svelte';
	import Footer from '$lib/components/Footer.svelte';
	import type { DashboardData } from '$lib/services/dashboard-data';

	let dashboardData = $state<DashboardData>({
		totalProviders: 0,
		activeProviders: 0,
		totalOfferings: 0,
		totalContracts: 0,
		activeValidators: 0,
		totalTransfers: 0,
		totalVolumeE9s: 0
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

<div class="min-h-screen bg-base text-white">
	<Header />
	<HeroSection />
	<TrustGuaranteesSection />
	<DashboardSection {dashboardData} {error} />
	<Footer />
</div>
