<script lang="ts">
	import { onMount } from 'svelte';
	import { authStore } from '$lib/stores/auth';
	import { hexEncode } from '$lib/services/api';
	import { signRequest } from '$lib/services/auth-api';
	import {
		getProviderOnboarding,
		updateProviderOnboarding,
		syncProviderHelpcenter,
		type ProviderOnboarding
	} from '$lib/services/api';

	interface CommonIssue {
		question: string;
		answer: string;
	}

	let currentIdentity = $state<any>(null);
	let loading = $state(true);
	let saving = $state(false);
	let syncing = $state(false);
	let error = $state<string | null>(null);
	let successMessage = $state<string | null>(null);

	// Form fields
	let supportEmail = $state('');
	let supportHours = $state('');
	let customSupportHours = $state('');
	let supportChannels = $state<string[]>([]);
	let regions = $state<string[]>([]);
	let paymentMethods = $state<string[]>([]);
	let refundPolicy = $state('');
	let customRefundPolicy = $state('');
	let slaGuarantee = $state('');
	let usp1 = $state('');
	let usp2 = $state('');
	let usp3 = $state('');
	let commonIssues = $state<CommonIssue[]>([]);

	// Predefined options from spec
	const supportHoursOptions = [
		'24/7',
		'Business hours (Mon-Fri 9-17 UTC)',
		'Business hours (Mon-Fri 9-17 US Eastern)',
		'custom'
	];

	const supportChannelOptions = ['Email', 'Live Chat', 'Phone', 'Ticket System', 'Discord', 'Telegram'];

	const regionOptions = [
		'North America',
		'South America',
		'Europe',
		'Asia Pacific',
		'Middle East',
		'Africa',
		'Global'
	];

	const paymentMethodOptions = [
		'Cryptocurrency (BTC, ETH, etc.)',
		'Credit Card (Stripe)',
		'PayPal',
		'Bank Transfer',
		'ICP (Internet Computer)'
	];

	const refundPolicyOptions = [
		'30-day money-back guarantee',
		'14-day money-back guarantee',
		'7-day money-back guarantee',
		'Pro-rated refunds only',
		'No refunds',
		'custom'
	];

	const slaGuaranteeOptions = [
		'99.99% (52 min/year downtime)',
		'99.9% (8.7 hours/year downtime)',
		'99.5% (1.8 days/year downtime)',
		'99% (3.6 days/year downtime)',
		'No SLA guarantee'
	];

	onMount(() => {
		const unsubscribe = authStore.activeIdentity.subscribe((identity) => {
			currentIdentity = identity;
		});

		if (currentIdentity?.publicKeyBytes) {
			loadOnboarding();
		}

		return unsubscribe;
	});

	async function loadOnboarding() {
		try {
			loading = true;
			error = null;

			if (!currentIdentity?.publicKeyBytes) {
				error = 'Please authenticate to view onboarding';
				return;
			}

			const pubkeyHex = hexEncode(currentIdentity.publicKeyBytes);
			const data = await getProviderOnboarding(pubkeyHex);

			if (data) {
				supportEmail = data.support_email || '';
				supportHours = data.support_hours || '';
				if (supportHours && !supportHoursOptions.slice(0, -1).includes(supportHours)) {
					customSupportHours = supportHours;
					supportHours = 'custom';
				}

				supportChannels = data.support_channels ? JSON.parse(data.support_channels) : [];
				regions = data.regions ? JSON.parse(data.regions) : [];
				paymentMethods = data.payment_methods ? JSON.parse(data.payment_methods) : [];

				refundPolicy = data.refund_policy || '';
				if (refundPolicy && !refundPolicyOptions.slice(0, -1).includes(refundPolicy)) {
					customRefundPolicy = refundPolicy;
					refundPolicy = 'custom';
				}

				slaGuarantee = data.sla_guarantee || '';

				const usps = data.unique_selling_points ? JSON.parse(data.unique_selling_points) : [];
				usp1 = usps[0] || '';
				usp2 = usps[1] || '';
				usp3 = usps[2] || '';

				commonIssues = data.common_issues ? JSON.parse(data.common_issues) : [];
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load onboarding data';
			console.error('Error loading onboarding:', e);
		} finally {
			loading = false;
		}
	}

	function toggleChannel(channel: string) {
		if (supportChannels.includes(channel)) {
			supportChannels = supportChannels.filter((c) => c !== channel);
		} else {
			supportChannels = [...supportChannels, channel];
		}
	}

	function toggleRegion(region: string) {
		if (regions.includes(region)) {
			regions = regions.filter((r) => r !== region);
		} else {
			regions = [...regions, region];
		}
	}

	function togglePaymentMethod(method: string) {
		if (paymentMethods.includes(method)) {
			paymentMethods = paymentMethods.filter((m) => m !== method);
		} else {
			paymentMethods = [...paymentMethods, method];
		}
	}

	function addCommonIssue() {
		if (commonIssues.length < 10) {
			commonIssues = [...commonIssues, { question: '', answer: '' }];
		}
	}

	function removeCommonIssue(index: number) {
		commonIssues = commonIssues.filter((_, i) => i !== index);
	}

	async function handleSubmit(event: Event) {
		event.preventDefault();

		if (!currentIdentity?.identity || !currentIdentity?.publicKeyBytes) {
			error = 'Authentication required';
			return;
		}

		// Validation
		if (!supportEmail || !supportEmail.includes('@')) {
			error = 'Valid support email is required';
			return;
		}

		const finalSupportHours = supportHours === 'custom' ? customSupportHours : supportHours;
		if (!finalSupportHours) {
			error = 'Support hours is required';
			return;
		}

		if (supportChannels.length === 0) {
			error = 'At least one support channel is required';
			return;
		}

		if (regions.length === 0) {
			error = 'At least one region is required';
			return;
		}

		if (paymentMethods.length === 0) {
			error = 'At least one payment method is required';
			return;
		}

		// Validate USPs character count
		if (usp1 && usp1.length > 200) {
			error = 'Unique selling point 1 must be 200 characters or less';
			return;
		}
		if (usp2 && usp2.length > 200) {
			error = 'Unique selling point 2 must be 200 characters or less';
			return;
		}
		if (usp3 && usp3.length > 200) {
			error = 'Unique selling point 3 must be 200 characters or less';
			return;
		}

		// Filter out empty USPs
		const usps = [usp1, usp2, usp3].filter((u) => u.trim());

		// Filter out empty common issues
		const filteredIssues = commonIssues.filter((issue) => issue.question.trim() && issue.answer.trim());

		const finalRefundPolicy = refundPolicy === 'custom' ? customRefundPolicy : refundPolicy;

		try {
			saving = true;
			error = null;
			successMessage = null;

			const pubkeyHex = hexEncode(currentIdentity.publicKeyBytes);
			const path = `/api/v1/providers/${pubkeyHex}/onboarding`;

			const onboardingData: Partial<ProviderOnboarding> = {
				support_email: supportEmail,
				support_hours: finalSupportHours,
				support_channels: JSON.stringify(supportChannels),
				regions: JSON.stringify(regions),
				payment_methods: JSON.stringify(paymentMethods),
				refund_policy: finalRefundPolicy || undefined,
				sla_guarantee: slaGuarantee || undefined,
				unique_selling_points: usps.length > 0 ? JSON.stringify(usps) : undefined,
				common_issues: filteredIssues.length > 0 ? JSON.stringify(filteredIssues) : undefined
			};

			const signed = await signRequest(currentIdentity.identity, 'PUT', path, onboardingData);

			if (!signed.body) {
				throw new Error('Failed to sign request');
			}

			await updateProviderOnboarding(currentIdentity.publicKeyBytes, onboardingData, signed.headers);

			successMessage = 'Onboarding data saved successfully!';
			setTimeout(() => {
				successMessage = null;
			}, 5000);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to save onboarding data';
			console.error('Error saving onboarding:', e);
		} finally {
			saving = false;
		}
	}

	async function handleSync() {
		if (!currentIdentity?.identity || !currentIdentity?.publicKeyBytes) {
			error = 'Authentication required';
			return;
		}

		try {
			syncing = true;
			error = null;
			successMessage = null;

			const pubkeyHex = hexEncode(currentIdentity.publicKeyBytes);
			const path = `/api/v1/providers/${pubkeyHex}/helpcenter/sync`;

			const signed = await signRequest(currentIdentity.identity, 'POST', path, {});

			const result = await syncProviderHelpcenter(currentIdentity.publicKeyBytes, signed.headers);

			successMessage = `Help center ${result.action} successfully! Article ID: ${result.article_id}`;
			setTimeout(() => {
				successMessage = null;
			}, 5000);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to sync help center';
			console.error('Error syncing help center:', e);
		} finally {
			syncing = false;
		}
	}
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">Help Center Setup</h1>
		<p class="text-white/60">
			Complete your provider onboarding to generate a help center article for your customers
		</p>
	</div>

	{#if successMessage}
		<div class="bg-green-500/20 border border-green-500/30 rounded-lg p-4 text-green-400">
			<p class="font-semibold">Success!</p>
			<p class="text-sm mt-1">{successMessage}</p>
		</div>
	{/if}

	{#if error}
		<div class="bg-red-500/20 border border-red-500/30 rounded-lg p-4 text-red-400">
			<p class="font-semibold">Error</p>
			<p class="text-sm mt-1">{error}</p>
		</div>
	{/if}

	{#if loading}
		<div class="flex justify-center items-center p-8">
			<div class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-400"></div>
		</div>
	{:else}
		<form onsubmit={handleSubmit} class="space-y-6">
			<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20 space-y-6">
				<h3 class="text-xl font-semibold text-white">Support Information</h3>

				<!-- Support Email -->
				<div>
					<label for="support-email" class="block text-white/80 mb-2">
						Support Email <span class="text-red-400">*</span>
					</label>
					<input
						id="support-email"
						type="email"
						bind:value={supportEmail}
						required
						class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:border-blue-400 transition-colors"
						placeholder="support@example.com"
					/>
				</div>

				<!-- Support Hours -->
				<div>
					<label for="support-hours" class="block text-white/80 mb-2">
						Support Hours <span class="text-red-400">*</span>
					</label>
					<select
						id="support-hours"
						bind:value={supportHours}
						required
						class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white focus:outline-none focus:border-blue-400 transition-colors"
					>
						<option value="">Select support hours...</option>
						{#each supportHoursOptions as option}
							<option value={option}>{option === 'custom' ? 'Custom...' : option}</option>
						{/each}
					</select>
					{#if supportHours === 'custom'}
						<input
							type="text"
							bind:value={customSupportHours}
							placeholder="e.g., Mon-Fri 9-17 PST"
							class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:border-blue-400 transition-colors mt-2"
						/>
					{/if}
				</div>

				<!-- Support Channels -->
				<div>
					<div class="block text-white/80 mb-2">
						Support Channels <span class="text-red-400">*</span>
					</div>
					<div class="grid grid-cols-2 md:grid-cols-3 gap-3">
						{#each supportChannelOptions as channel}
							<label class="flex items-center space-x-2 cursor-pointer">
								<input
									type="checkbox"
									checked={supportChannels.includes(channel)}
									onchange={() => toggleChannel(channel)}
									class="w-4 h-4 rounded border-white/20 bg-white/10 text-blue-500 focus:ring-blue-400"
								/>
								<span class="text-white/80">{channel}</span>
							</label>
						{/each}
					</div>
				</div>
			</div>

			<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20 space-y-6">
				<h3 class="text-xl font-semibold text-white">Service Details</h3>

				<!-- Regions -->
				<div>
					<div class="block text-white/80 mb-2">
						Regions <span class="text-red-400">*</span>
					</div>
					<div class="grid grid-cols-2 md:grid-cols-3 gap-3">
						{#each regionOptions as region}
							<label class="flex items-center space-x-2 cursor-pointer">
								<input
									type="checkbox"
									checked={regions.includes(region)}
									onchange={() => toggleRegion(region)}
									class="w-4 h-4 rounded border-white/20 bg-white/10 text-blue-500 focus:ring-blue-400"
								/>
								<span class="text-white/80">{region}</span>
							</label>
						{/each}
					</div>
				</div>

				<!-- Payment Methods -->
				<div>
					<div class="block text-white/80 mb-2">
						Payment Methods <span class="text-red-400">*</span>
					</div>
					<div class="grid grid-cols-1 md:grid-cols-2 gap-3">
						{#each paymentMethodOptions as method}
							<label class="flex items-center space-x-2 cursor-pointer">
								<input
									type="checkbox"
									checked={paymentMethods.includes(method)}
									onchange={() => togglePaymentMethod(method)}
									class="w-4 h-4 rounded border-white/20 bg-white/10 text-blue-500 focus:ring-blue-400"
								/>
								<span class="text-white/80">{method}</span>
							</label>
						{/each}
					</div>
				</div>

				<!-- Refund Policy -->
				<div>
					<label for="refund-policy" class="block text-white/80 mb-2">Refund Policy</label>
					<select
						id="refund-policy"
						bind:value={refundPolicy}
						class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white focus:outline-none focus:border-blue-400 transition-colors"
					>
						<option value="">Select refund policy...</option>
						{#each refundPolicyOptions as option}
							<option value={option}>{option === 'custom' ? 'Custom...' : option}</option>
						{/each}
					</select>
					{#if refundPolicy === 'custom'}
						<input
							type="text"
							bind:value={customRefundPolicy}
							placeholder="Describe your refund policy"
							class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:border-blue-400 transition-colors mt-2"
						/>
					{/if}
				</div>

				<!-- SLA Guarantee -->
				<div>
					<label for="sla-guarantee" class="block text-white/80 mb-2">SLA Guarantee</label>
					<select
						id="sla-guarantee"
						bind:value={slaGuarantee}
						class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white focus:outline-none focus:border-blue-400 transition-colors"
					>
						<option value="">Select SLA guarantee...</option>
						{#each slaGuaranteeOptions as option}
							<option value={option}>{option}</option>
						{/each}
					</select>
				</div>
			</div>

			<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20 space-y-6">
				<h3 class="text-xl font-semibold text-white">Marketing Information</h3>

				<!-- Unique Selling Points -->
				<div>
					<div class="block text-white/80 mb-2">Unique Selling Points (max 200 chars each)</div>
					<div class="space-y-3">
						<div>
							<textarea
								bind:value={usp1}
								maxlength="200"
								rows="2"
								placeholder="Key differentiator #1"
								class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:border-blue-400 transition-colors"
							></textarea>
							<p class="text-xs text-white/50 mt-1">{usp1.length}/200 characters</p>
						</div>
						<div>
							<textarea
								bind:value={usp2}
								maxlength="200"
								rows="2"
								placeholder="Key differentiator #2"
								class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:border-blue-400 transition-colors"
							></textarea>
							<p class="text-xs text-white/50 mt-1">{usp2.length}/200 characters</p>
						</div>
						<div>
							<textarea
								bind:value={usp3}
								maxlength="200"
								rows="2"
								placeholder="Key differentiator #3"
								class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:border-blue-400 transition-colors"
							></textarea>
							<p class="text-xs text-white/50 mt-1">{usp3.length}/200 characters</p>
						</div>
					</div>
				</div>
			</div>

			<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20 space-y-6">
				<div class="flex items-center justify-between">
					<h3 class="text-xl font-semibold text-white">Common Issues / FAQ</h3>
					{#if commonIssues.length < 10}
						<button
							type="button"
							onclick={addCommonIssue}
							class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
						>
							Add Issue
						</button>
					{/if}
				</div>

				{#if commonIssues.length === 0}
					<p class="text-white/60 text-sm">
						Add frequently asked questions to help your customers (optional, max 10)
					</p>
				{:else}
					<div class="space-y-4">
						{#each commonIssues as issue, index}
							<div class="border border-white/20 rounded-lg p-4 space-y-3">
								<div class="flex items-start justify-between">
									<span class="text-white/60 text-sm">Issue #{index + 1}</span>
									<button
										type="button"
										onclick={() => removeCommonIssue(index)}
										class="text-red-400 hover:text-red-300 text-sm"
									>
										Remove
									</button>
								</div>
								<input
									type="text"
									bind:value={issue.question}
									placeholder="Question"
									class="w-full px-4 py-2 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:border-blue-400 transition-colors"
								/>
								<textarea
									bind:value={issue.answer}
									rows="3"
									placeholder="Answer"
									class="w-full px-4 py-2 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:border-blue-400 transition-colors"
								></textarea>
							</div>
						{/each}
					</div>
				{/if}
			</div>

			<div class="flex gap-4">
				<button
					type="submit"
					disabled={saving}
					class="px-6 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 hover:scale-105 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
				>
					{saving ? 'Saving...' : 'Save Onboarding Data'}
				</button>

				<button
					type="button"
					onclick={handleSync}
					disabled={syncing}
					class="px-6 py-3 bg-green-600 rounded-lg font-semibold text-white hover:bg-green-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
				>
					{syncing ? 'Syncing...' : 'Sync to Help Center'}
				</button>
			</div>
		</form>
	{/if}
</div>
