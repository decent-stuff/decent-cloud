<script lang="ts">
	import { onMount } from 'svelte';
	import { browser } from '$app/environment';
	import { dashboardStore } from '$lib/stores/dashboard';
	import { authStore } from '$lib/stores/auth';
	import type { DashboardData } from '$lib/services/dashboard-data';
	import type { IdentityInfo } from '$lib/stores/auth';

	let dashboardData = $state<DashboardData>({
		dctPrice: 0,
		providerCount: 0,
		totalBlocks: 0,
		blocksUntilHalving: 0,
		rewardPerBlock: 0,
		accumulatedRewards: 0
	});
	let error = $state<string | null>(null);
	let currentIdentity = $state<IdentityInfo | null>(null);

	onMount(() => {
		if (!browser) return;

		const unsubscribeData = dashboardStore.data.subscribe((value) => {
			dashboardData = value;
		});
		const unsubscribeError = dashboardStore.error.subscribe((value) => {
			error = value;
		});
		const unsubscribeAuth = authStore.currentIdentity.subscribe((value) => {
			currentIdentity = value;
		});

		dashboardStore.load();
		authStore.updateDisplayName();
		const interval = setInterval(() => dashboardStore.load(), 10000);

		return () => {
			unsubscribeData();
			unsubscribeError();
			unsubscribeAuth();
			clearInterval(interval);
		};
	});
</script>

<div class="space-y-8">
	<!-- User Info Section -->
	{#if currentIdentity}
		<div class="bg-gradient-to-r from-blue-500/20 to-purple-600/20 backdrop-blur-lg rounded-xl p-6 border border-blue-500/30">
			<div class="flex items-center gap-4">
				<div class="text-4xl">
					{#if currentIdentity.type === 'ii'}
						🌐
					{:else}
						🔑
					{/if}
				</div>
				<div class="flex-1">
					<h2 class="text-2xl font-bold text-white mb-1">
						{#if currentIdentity.displayName}
							Welcome back, {currentIdentity.displayName}!
						{:else}
							Welcome back!
						{/if}
					</h2>
					<p class="text-white/70 text-sm">
						Logged in via {currentIdentity.type === 'ii' ? 'Internet Identity' : 'Seed Phrase'}
					</p>
					<p class="text-white/50 text-xs font-mono mt-2 break-all">
						{currentIdentity.principal.toString()}
					</p>
				</div>
			</div>
		</div>
	{/if}

	<div>
		<h1 class="text-4xl font-bold text-white mb-2">Dashboard Overview</h1>
		<p class="text-white/60">Network statistics and quick actions</p>
	</div>

	{#if error}
		<div class="bg-red-500/20 border border-red-500/30 rounded-lg p-4 text-red-400">
			<p class="font-semibold">Error loading dashboard data</p>
			<p class="text-sm mt-1">{error}</p>
		</div>
	{/if}

	<!-- Stats Grid -->
	<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
		<!-- DCT Price -->
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
			<div class="flex items-center justify-between mb-2">
				<h3 class="text-white/70 text-sm font-medium">DCT Price</h3>
				<span class="text-2xl">💰</span>
			</div>
			<p class="text-3xl font-bold text-white">${dashboardData.dctPrice.toFixed(4)}</p>
			<p class="text-white/50 text-sm mt-1">USD per token</p>
		</div>

		<!-- Total Providers -->
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
			<div class="flex items-center justify-between mb-2">
				<h3 class="text-white/70 text-sm font-medium">Providers</h3>
				<span class="text-2xl">🖥️</span>
			</div>
			<p class="text-3xl font-bold text-white">{dashboardData.providerCount}</p>
			<p class="text-white/50 text-sm mt-1">Active providers</p>
		</div>

		<!-- Total Blocks -->
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
			<div class="flex items-center justify-between mb-2">
				<h3 class="text-white/70 text-sm font-medium">Blocks</h3>
				<span class="text-2xl">🔗</span>
			</div>
			<p class="text-3xl font-bold text-white">{dashboardData.totalBlocks.toLocaleString()}</p>
			<p class="text-white/50 text-sm mt-1">Total blocks</p>
		</div>

		<!-- Reward Per Block -->
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
			<div class="flex items-center justify-between mb-2">
				<h3 class="text-white/70 text-sm font-medium">Reward Per Block</h3>
				<span class="text-2xl">🎁</span>
			</div>
			<p class="text-3xl font-bold text-white">{dashboardData.rewardPerBlock.toFixed(2)}</p>
			<p class="text-white/50 text-sm mt-1">DCT per block</p>
		</div>

		<!-- Accumulated Rewards -->
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
			<div class="flex items-center justify-between mb-2">
				<h3 class="text-white/70 text-sm font-medium">Accumulated Rewards</h3>
				<span class="text-2xl">💰</span>
			</div>
			<p class="text-3xl font-bold text-white">{dashboardData.accumulatedRewards.toFixed(2)}</p>
			<p class="text-white/50 text-sm mt-1">Pending DCT rewards</p>
		</div>

		<!-- Halving -->
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
			<div class="flex items-center justify-between mb-2">
				<h3 class="text-white/70 text-sm font-medium">Until Halving</h3>
				<span class="text-2xl">⏰</span>
			</div>
			<p class="text-3xl font-bold text-white">
				{dashboardData.blocksUntilHalving.toLocaleString()}
			</p>
			<p class="text-white/50 text-sm mt-1">Blocks remaining</p>
		</div>
	</div>

	<!-- Quick Actions -->
	<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
		<h2 class="text-2xl font-bold text-white mb-4">Quick Actions</h2>
		<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
			<a
				href="/dashboard/marketplace"
				class="p-4 bg-gradient-to-r from-blue-500/20 to-purple-600/20 rounded-lg border border-blue-500/30 hover:border-blue-400 transition-all group"
			>
				<span class="text-3xl mb-2 block">🛒</span>
				<h3 class="text-white font-semibold mb-1 group-hover:text-blue-400">Browse Marketplace</h3>
				<p class="text-white/60 text-sm">Find cloud services</p>
			</a>

			<a
				href="/dashboard/offerings"
				class="p-4 bg-gradient-to-r from-purple-500/20 to-pink-600/20 rounded-lg border border-purple-500/30 hover:border-purple-400 transition-all group"
			>
				<span class="text-3xl mb-2 block">📦</span>
				<h3 class="text-white font-semibold mb-1 group-hover:text-purple-400">Manage Offerings</h3>
				<p class="text-white/60 text-sm">Your cloud services</p>
			</a>

			<a
				href="/dashboard/validators"
				class="p-4 bg-gradient-to-r from-green-500/20 to-teal-600/20 rounded-lg border border-green-500/30 hover:border-green-400 transition-all group"
			>
				<span class="text-3xl mb-2 block">✓</span>
				<h3 class="text-white font-semibold mb-1 group-hover:text-green-400">View Validators</h3>
				<p class="text-white/60 text-sm">Network participants</p>
			</a>
		</div>
	</div>
</div>
