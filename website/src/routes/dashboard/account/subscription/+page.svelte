<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { page } from "$app/stores";
	import { authStore } from "$lib/stores/auth";
	import { navigateToLogin } from "$lib/utils/navigation";
	import { signRequest } from "$lib/services/auth-api";
	import {
		listSubscriptionPlans,
		getCurrentSubscription,
		createSubscriptionCheckout,
		createBillingPortal,
		type SubscriptionPlan,
		type AccountSubscription
	} from "$lib/services/api";
	import type { IdentityInfo } from "$lib/stores/auth";

	let currentIdentity = $state<IdentityInfo | null>(null);
	let isAuthenticated = $state(false);
	let unsubscribe: (() => void) | null = null;
	let unsubscribeAuth: (() => void) | null = null;

	// Data state
	let plans = $state<SubscriptionPlan[]>([]);
	let subscription = $state<AccountSubscription | null>(null);
	let loading = $state(false);
	let loadingPlans = $state(false);
	let upgrading = $state<string | null>(null);
	let openingPortal = $state(false);
	let error = $state("");

	onMount(() => {
		unsubscribeAuth = authStore.isAuthenticated.subscribe((isAuth) => {
			isAuthenticated = isAuth;
		});

		unsubscribe = authStore.activeIdentity.subscribe(async (value) => {
			currentIdentity = value;
			if (value?.identity) {
				await loadData();
			}
		});

		// Load plans even without auth
		loadPlans();
	});

	onDestroy(() => {
		unsubscribe?.();
		unsubscribeAuth?.();
	});

	async function loadPlans() {
		loadingPlans = true;
		try {
			plans = await listSubscriptionPlans();
		} catch (e) {
			console.error("Failed to load plans:", e);
		} finally {
			loadingPlans = false;
		}
	}

	async function loadData() {
		if (!currentIdentity?.identity) return;
		loading = true;
		error = "";
		try {
			const { headers } = await signRequest(currentIdentity.identity, "GET", "/api/v1/subscriptions/current");
			subscription = await getCurrentSubscription(headers);
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load subscription";
		} finally {
			loading = false;
		}
	}

	async function handleUpgrade(planId: string) {
		if (!currentIdentity?.identity) return;
		upgrading = planId;
		error = "";
		try {
			const { headers } = await signRequest(
				currentIdentity.identity,
				"POST",
				"/api/v1/subscriptions/checkout",
				{ plan_id: planId }
			);
			const checkoutUrl = await createSubscriptionCheckout(planId, headers);
			// Redirect to Stripe Checkout
			window.location.href = checkoutUrl;
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to create checkout";
			upgrading = null;
		}
	}

	async function handleManageSubscription() {
		if (!currentIdentity?.identity) return;
		openingPortal = true;
		error = "";
		try {
			const { headers } = await signRequest(
				currentIdentity.identity,
				"POST",
				"/api/v1/subscriptions/portal"
			);
			const portalUrl = await createBillingPortal(headers);
			// Redirect to Stripe Portal
			window.location.href = portalUrl;
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to open billing portal";
			openingPortal = false;
		}
	}

	function handleLogin() {
		navigateToLogin($page.url.pathname);
	}

	function formatPrice(cents: number): string {
		return `$${(cents / 100).toFixed(0)}`;
	}

	function parseFeatures(featuresJson: string | undefined): string[] {
		if (!featuresJson) return [];
		try {
			return JSON.parse(featuresJson);
		} catch {
			return [];
		}
	}

	function getStatusBadge(status: string): { text: string; class: string } {
		switch (status) {
			case "active":
				return { text: "Active", class: "bg-green-500/20 text-green-300 border-green-500/50" };
			case "trialing":
				return { text: "Trial", class: "bg-blue-500/20 text-blue-300 border-blue-500/50" };
			case "past_due":
				return { text: "Past Due", class: "bg-yellow-500/20 text-yellow-300 border-yellow-500/50" };
			case "canceled":
				return { text: "Canceled", class: "bg-red-500/20 text-red-300 border-red-500/50" };
			default:
				return { text: status, class: "bg-white/20 text-white/70 border-white/30" };
		}
	}

	const featureLabels: Record<string, string> = {
		"marketplace_browse": "Browse Marketplace",
		"one_rental": "1 Active Rental",
		"unlimited_rentals": "Unlimited Rentals",
		"priority_support": "Priority Support",
		"api_access": "API Access",
		"dedicated_support": "Dedicated Support",
		"sla_guarantee": "SLA Guarantee"
	};
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">Subscription</h1>
		<p class="text-white/60">Manage your subscription plan and billing</p>
	</div>

	{#if error}
		<div class="p-4 bg-red-500/20 border border-red-500/50 rounded-lg text-red-200">
			{error}
		</div>
	{/if}

	{#if !isAuthenticated}
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-8 border border-white/20 text-center">
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">⭐</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-white/70">
					Create an account or login to manage your subscription.
				</p>
				<button
					onclick={handleLogin}
					class="px-8 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 hover:scale-105 transition-all"
				>
					Login / Create Account
				</button>
			</div>
		</div>
	{:else if loading}
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-8 border border-white/20 text-center">
			<p class="text-white/60">Loading subscription...</p>
		</div>
	{:else}
		<!-- Current Subscription -->
		{#if subscription}
			{@const badge = getStatusBadge(subscription.status)}
			<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
				<div class="flex items-center justify-between mb-4">
					<h2 class="text-xl font-semibold text-white">Current Plan</h2>
					<span class="px-3 py-1 rounded-full text-sm border {badge.class}">
						{badge.text}
					</span>
				</div>

				<div class="grid grid-cols-1 md:grid-cols-2 gap-6">
					<div>
						<p class="text-3xl font-bold text-white">{subscription.plan_name}</p>
						{#if subscription.cancel_at_period_end}
							<p class="text-yellow-400 text-sm mt-2">
								Cancels at end of billing period
							</p>
						{/if}
						{#if subscription.current_period_end}
							<p class="text-white/60 text-sm mt-1">
								{subscription.cancel_at_period_end ? "Access until" : "Renews"}: {new Date(subscription.current_period_end * 1000).toLocaleDateString()}
							</p>
						{/if}
					</div>
					<div>
						<p class="text-white/70 text-sm mb-2">Your Features</p>
						<ul class="space-y-1">
							{#each subscription.features as feature}
								<li class="text-white/80 text-sm flex items-center gap-2">
									<span class="text-green-400">✓</span>
									{featureLabels[feature] || feature}
								</li>
							{/each}
						</ul>
					</div>
				</div>

				{#if subscription.stripe_subscription_id}
					<div class="mt-6 pt-4 border-t border-white/10">
						<button
							onclick={handleManageSubscription}
							disabled={openingPortal}
							class="px-6 py-2 bg-white/10 border border-white/20 rounded-lg text-white hover:bg-white/20 disabled:opacity-50 transition-colors"
						>
							{openingPortal ? "Opening..." : "Manage Subscription"}
						</button>
						<p class="text-white/40 text-xs mt-2">
							Update payment method, view invoices, or cancel subscription
						</p>
					</div>
				{/if}
			</div>
		{/if}

		<!-- Available Plans -->
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
			<h2 class="text-xl font-semibold text-white mb-6">
				{subscription?.plan_id === "free" ? "Upgrade Your Plan" : "Available Plans"}
			</h2>

			{#if loadingPlans}
				<p class="text-white/60">Loading plans...</p>
			{:else}
				<div class="grid grid-cols-1 md:grid-cols-3 gap-6">
					{#each plans as plan}
						{@const isCurrentPlan = subscription?.plan_id === plan.id}
						{@const features = parseFeatures(plan.features)}
						<div
							class="p-6 rounded-xl border transition-all {isCurrentPlan
								? 'bg-blue-500/10 border-blue-500/50'
								: 'bg-white/5 border-white/20 hover:border-white/40'}"
						>
							<h3 class="text-xl font-bold text-white">{plan.name}</h3>
							{#if plan.description}
								<p class="text-white/60 text-sm mt-1">{plan.description}</p>
							{/if}

							<div class="mt-4">
								<span class="text-3xl font-bold text-white">
									{plan.monthlyPriceCents === 0 ? "Free" : formatPrice(plan.monthlyPriceCents)}
								</span>
								{#if plan.monthlyPriceCents > 0}
									<span class="text-white/60">/month</span>
								{/if}
							</div>

							{#if plan.trialDays > 0}
								<p class="text-blue-400 text-sm mt-2">
									{plan.trialDays}-day free trial
								</p>
							{/if}

							<ul class="mt-4 space-y-2">
								{#each features as feature}
									<li class="text-white/70 text-sm flex items-center gap-2">
										<span class="text-green-400">✓</span>
										{featureLabels[feature] || feature}
									</li>
								{/each}
							</ul>

							<div class="mt-6">
								{#if isCurrentPlan}
									<span class="inline-block px-4 py-2 bg-blue-500/20 text-blue-300 rounded-lg text-sm">
										Current Plan
									</span>
								{:else if plan.id === "free"}
									<!-- Can't downgrade via checkout, use portal -->
									{#if subscription?.stripe_subscription_id}
										<button
											onclick={handleManageSubscription}
											disabled={openingPortal}
											class="w-full px-4 py-2 bg-white/10 border border-white/20 rounded-lg text-white hover:bg-white/20 disabled:opacity-50 transition-colors"
										>
											Manage Plan
										</button>
									{/if}
								{:else if plan.stripe_price_id}
									<button
										onclick={() => handleUpgrade(plan.id)}
										disabled={upgrading !== null}
										class="w-full px-4 py-2 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 disabled:opacity-50 transition-all"
									>
										{upgrading === plan.id ? "Processing..." : subscription?.plan_id === "free" ? "Upgrade" : "Switch Plan"}
									</button>
								{:else}
									<span class="inline-block px-4 py-2 bg-white/5 text-white/40 rounded-lg text-sm">
										Contact Sales
									</span>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</div>

		<!-- Help Section -->
		<div class="bg-white/5 backdrop-blur-lg rounded-xl p-6 border border-white/10">
			<h3 class="text-lg font-semibold text-white/80 mb-3">Need Help?</h3>
			<ul class="text-white/60 text-sm space-y-2">
				<li>Plans can be upgraded or downgraded at any time.</li>
				<li>When upgrading, you'll be charged the prorated difference.</li>
				<li>Cancellations take effect at the end of your billing period.</li>
				<li>For enterprise plans or custom needs, contact our sales team.</li>
			</ul>
		</div>
	{/if}
</div>
