<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { page } from '$app/stores';
	import { authStore, type IdentityInfo } from '$lib/stores/auth';
	import { navigateToLogin } from '$lib/utils/navigation';
	import { hexEncode, getProviderOnboarding, updateProviderOnboarding, syncProviderHelpcenter, type ProviderOnboarding } from '$lib/services/api';
	import { signRequest } from '$lib/services/auth-api';
	import { getNotificationConfig, getNotificationUsage, updateNotificationConfig, testNotificationChannel, type NotificationConfig, type NotificationUsage } from '$lib/services/notification-api';
	import { getSupportPortalStatus, resetSupportPortalPassword, createSupportPortalAccount, type SupportPortalStatus } from '$lib/services/chatwoot-api';

	interface CommonIssue { question: string; answer: string; }

	let currentIdentity = $state<IdentityInfo | null>(null);
	let isAuthenticated = $state(false);
	let unsubscribe: (() => void) | null = null;
	let unsubscribeAuth: (() => void) | null = null;

	// Support portal URLs - base URL from env, specific paths from portalStatus
	const CHATWOOT_BASE_URL = import.meta.env.VITE_CHATWOOT_BASE_URL || 'https://support.decent-cloud.org';
	const CHATWOOT_ACCOUNT_ID = import.meta.env.VITE_CHATWOOT_ACCOUNT_ID || '1';

	// Shared state
	let loading = $state(true);
	let error = $state<string | null>(null);
	let success = $state<string | null>(null);
	let articleUrl = $state<string | null>(null);

	// Help Center state
	let savingOnboarding = $state(false);
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

	// Notification state
	let savingNotif = $state(false);
	let notifyTelegram = $state(false);
	let notifyEmail = $state(false);
	let notifySms = $state(false);
	let telegramChatId = $state('');
	let notifyPhone = $state('');
	let usage = $state<NotificationUsage | null>(null);
	let testingChannel = $state<string | null>(null);
	let testResult = $state<{ sent: boolean; message: string } | null>(null);

	// Portal state
	let portalStatus = $state<SupportPortalStatus | null>(null);
	let resetting = $state(false);
	let creating = $state(false);
	let newPassword = $state<string | null>(null);

	const accountEmail = $derived(currentIdentity?.account?.email);

	// Dynamic URLs based on provider's portal status
	const supportDashboardUrl = $derived(
		portalStatus?.inboxId
			? `${CHATWOOT_BASE_URL}/app/accounts/${CHATWOOT_ACCOUNT_ID}/inbox/${portalStatus.inboxId}`
			: `${CHATWOOT_BASE_URL}/app/accounts/${CHATWOOT_ACCOUNT_ID}/dashboard`
	);
	// Help Center editing URL - goes to portal articles admin
	const helpCenterEditUrl = $derived(
		portalStatus?.portalSlug
			? `${CHATWOOT_BASE_URL}/app/accounts/${CHATWOOT_ACCOUNT_ID}/portals/${portalStatus.portalSlug}/articles`
			: null
	);

	// Options
	const supportHoursOptions = ['24/7', 'Business hours (Mon-Fri 9-17 UTC)', 'Business hours (Mon-Fri 9-17 US Eastern)', 'custom'];
	const supportChannelOptions = ['Email', 'Live Chat', 'Phone', 'Ticket System', 'Discord', 'Telegram'];
	const regionOptions = ['North America', 'South America', 'Europe', 'Asia Pacific', 'Middle East', 'Africa', 'Global'];
	const paymentMethodOptions = ['Cryptocurrency (BTC, ETH, etc.)', 'Credit Card (Stripe)', 'PayPal', 'Bank Transfer', 'ICP (Internet Computer)'];
	const refundPolicyOptions = ['30-day money-back guarantee', '14-day money-back guarantee', '7-day money-back guarantee', 'Pro-rated refunds only', 'No refunds', 'custom'];
	const slaGuaranteeOptions = ['99.99% (52 min/year downtime)', '99.9% (8.7 hours/year downtime)', '99.5% (1.8 days/year downtime)', '99% (3.6 days/year downtime)', 'No SLA guarantee'];

	onMount(() => {
		unsubscribeAuth = authStore.isAuthenticated.subscribe((v) => { isAuthenticated = v; });
		unsubscribe = authStore.currentIdentity.subscribe((v) => {
			currentIdentity = v;
			if (v?.identity) loadAll();
		});
	});

	onDestroy(() => { unsubscribe?.(); unsubscribeAuth?.(); });

	async function loadAll() {
		if (!currentIdentity?.identity) return;
		loading = true;
		error = null;
		try {
			await Promise.all([loadOnboarding(), loadNotifications(), loadPortal()]);
		} finally {
			loading = false;
		}
	}

	async function loadOnboarding() {
		if (!currentIdentity?.publicKeyBytes) return;
		const pubkeyHex = hexEncode(currentIdentity.publicKeyBytes);
		const data = await getProviderOnboarding(pubkeyHex).catch(() => null);
		if (data) {
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
			usp1 = usps[0] || ''; usp2 = usps[1] || ''; usp3 = usps[2] || '';
			commonIssues = data.common_issues ? JSON.parse(data.common_issues) : [];
		}
	}

	async function loadNotifications() {
		if (!currentIdentity?.identity) return;
		const [config, usageData] = await Promise.all([
			getNotificationConfig(currentIdentity.identity).catch(() => null),
			getNotificationUsage(currentIdentity.identity).catch(() => null)
		]);
		if (config) {
			notifyTelegram = config.notifyTelegram;
			notifyEmail = config.notifyEmail;
			notifySms = config.notifySms;
			telegramChatId = config.telegramChatId || '';
			notifyPhone = config.notifyPhone || '';
		}
		usage = usageData;
	}

	async function loadPortal() {
		if (!currentIdentity?.identity) return;
		portalStatus = await getSupportPortalStatus(currentIdentity.identity).catch(() => null);
	}

	function handleLogin() { navigateToLogin($page.url.pathname); }

	// Help Center handlers
	function toggleArray<T>(arr: T[], item: T): T[] {
		return arr.includes(item) ? arr.filter((x) => x !== item) : [...arr, item];
	}
	function addCommonIssue() { if (commonIssues.length < 10) commonIssues = [...commonIssues, { question: '', answer: '' }]; }
	function removeCommonIssue(i: number) { commonIssues = commonIssues.filter((_, idx) => idx !== i); }

	async function saveOnboarding(e: Event) {
		e.preventDefault();
		if (!currentIdentity?.identity || !currentIdentity?.publicKeyBytes) return;
		const finalSupportHours = supportHours === 'custom' ? customSupportHours : supportHours;
		const finalRefundPolicy = refundPolicy === 'custom' ? customRefundPolicy : refundPolicy;
		if (!accountEmail?.includes('@')) {
			error = 'Please add an email address in your Account settings first'; return;
		}
		if (!finalSupportHours || supportChannels.length === 0 || regions.length === 0 || paymentMethods.length === 0) {
			error = 'Please fill in all required fields'; return;
		}
		savingOnboarding = true; error = null; success = null;
		try {
			const pubkeyHex = hexEncode(currentIdentity.publicKeyBytes);
			const path = `/api/v1/providers/${pubkeyHex}/onboarding`;
			const usps = [usp1, usp2, usp3].filter((u) => u.trim());
			const filteredIssues = commonIssues.filter((i) => i.question.trim() && i.answer.trim());
			const onboardingData: Partial<ProviderOnboarding> = {
				support_email: accountEmail, support_hours: finalSupportHours, support_channels: JSON.stringify(supportChannels),
				regions: JSON.stringify(regions), payment_methods: JSON.stringify(paymentMethods),
				refund_policy: finalRefundPolicy || undefined, sla_guarantee: slaGuarantee || undefined,
				unique_selling_points: usps.length > 0 ? JSON.stringify(usps) : undefined,
				common_issues: filteredIssues.length > 0 ? JSON.stringify(filteredIssues) : undefined
			};
			const signed = await signRequest(currentIdentity.identity, 'PUT', path, onboardingData);
			await updateProviderOnboarding(pubkeyHex, onboardingData, signed.headers);
			// Auto-sync to help center after save
			const syncPath = `/api/v1/providers/${pubkeyHex}/helpcenter/sync`;
			const syncSigned = await signRequest(currentIdentity.identity, 'POST', syncPath);
			const result = await syncProviderHelpcenter(pubkeyHex, syncSigned.headers);
			success = `Help center ${result.action}!`;
			articleUrl = result.articleUrl;
		} catch (e) { error = e instanceof Error ? e.message : 'Save failed'; articleUrl = null; } finally { savingOnboarding = false; }
	}

	// Notification handlers
	async function handleTestChannel(channel: 'telegram' | 'email' | 'sms') {
		if (!currentIdentity?.identity) return;
		testingChannel = channel; testResult = null;
		try {
			testResult = await testNotificationChannel(currentIdentity.identity, channel);
			// Refresh usage counts after successful send
			if (testResult.sent) {
				usage = await getNotificationUsage(currentIdentity.identity).catch(() => null);
			}
		}
		catch (e) { testResult = { sent: false, message: e instanceof Error ? e.message : 'Test failed' }; }
		finally { testingChannel = null; }
	}

	async function saveNotifications() {
		if (!currentIdentity?.identity) return;
		savingNotif = true; error = null; success = null;
		try {
			await updateNotificationConfig(currentIdentity.identity, {
				notifyTelegram, notifyEmail, notifySms,
				telegramChatId: notifyTelegram ? telegramChatId : undefined,
				notifyPhone: notifySms ? notifyPhone : undefined
			});
			success = 'Notification settings saved!';
		} catch (e) { error = e instanceof Error ? e.message : 'Save failed'; } finally { savingNotif = false; }
	}

	// Portal handlers
	async function handlePortalReset() {
		if (!currentIdentity?.identity) return;
		resetting = true; error = null; newPassword = null;
		try { newPassword = (await resetSupportPortalPassword(currentIdentity.identity)).password; await loadPortal(); }
		catch (e) { error = e instanceof Error ? e.message : 'Reset failed'; } finally { resetting = false; }
	}

	async function handlePortalCreate() {
		if (!currentIdentity?.identity) return;
		creating = true; error = null; newPassword = null;
		try { newPassword = (await createSupportPortalAccount(currentIdentity.identity)).password; await loadPortal(); }
		catch (e) { error = e instanceof Error ? e.message : 'Create failed'; } finally { creating = false; }
	}

	function copyPassword() { if (newPassword) navigator.clipboard.writeText(newPassword); }
</script>

<div class="space-y-6">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">Provider Setup</h1>
		<p class="text-white/60">Manage your support account, notifications, and provider profile</p>
	</div>

	<!-- Section Navigation -->
	<nav class="flex flex-wrap gap-2 bg-white/5 rounded-lg p-2">
		<a href="#support" class="px-4 py-2 rounded-lg text-sm font-medium text-white/70 hover:bg-white/10 hover:text-white transition-colors">Support Portal</a>
		<a href="#notifications" class="px-4 py-2 rounded-lg text-sm font-medium text-white/70 hover:bg-white/10 hover:text-white transition-colors">Notifications</a>
		<a href="#helpcenter" class="px-4 py-2 rounded-lg text-sm font-medium text-white/70 hover:bg-white/10 hover:text-white transition-colors">Help Center Profile</a>
	</nav>

	{#if error}<div class="bg-red-500/20 border border-red-500/30 rounded-lg p-4 text-red-400">{error}</div>{/if}
	{#if success}<div class="bg-green-500/20 border border-green-500/30 rounded-lg p-4 text-green-400">{success}{#if articleUrl} <a href={articleUrl} target="_blank" rel="noopener" class="underline hover:text-green-200">View article →</a>{/if}</div>{/if}
	{#if testResult}<div class="p-4 rounded-lg {testResult.sent ? 'bg-green-500/20 text-green-200' : 'bg-yellow-500/20 text-yellow-200'}"><strong>{testResult.sent ? 'Sent' : 'Failed'}:</strong> {testResult.message}</div>{/if}

	{#if !isAuthenticated}
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-8 border border-white/20 text-center">
			<span class="text-6xl">⚙️</span>
			<h2 class="text-2xl font-bold text-white mt-4">Login Required</h2>
			<p class="text-white/70 mt-2">Login to configure your provider profile.</p>
			<button onclick={handleLogin} class="mt-4 px-8 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 transition-all">
				Login / Create Account
			</button>
		</div>
	{:else if loading}
		<div class="flex justify-center p-8"><div class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-400"></div></div>
	{:else}
		<!-- Support Portal Section (moved to top - most frequently accessed) -->
		<section id="support" class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20 space-y-6 scroll-mt-4">
			<h2 class="text-2xl font-bold text-white">Support Portal</h2>
			<p class="text-white/60 text-sm">Access your support dashboard and knowledge base</p>

			<!-- Quick Links -->
			<div class="grid md:grid-cols-2 gap-4">
				<a href={supportDashboardUrl} target="_blank" rel="noopener noreferrer" class="flex items-start gap-4 p-4 rounded-lg bg-gradient-to-br from-blue-600/20 to-purple-600/20 border border-blue-500/30 hover:border-blue-400/50 transition-colors group">
					<span class="text-3xl">🎧</span>
					<div>
						<h3 class="text-white font-semibold group-hover:text-blue-300 transition-colors">Support Dashboard <span class="text-xs opacity-50">↗</span></h3>
						<p class="text-white/60 text-sm mt-1">View and respond to customer tickets, manage conversations</p>
					</div>
				</a>
				{#if helpCenterEditUrl}
					<a href={helpCenterEditUrl} target="_blank" rel="noopener noreferrer" class="flex items-start gap-4 p-4 rounded-lg bg-gradient-to-br from-green-600/20 to-teal-600/20 border border-green-500/30 hover:border-green-400/50 transition-colors group">
						<span class="text-3xl">📝</span>
						<div>
							<h3 class="text-white font-semibold group-hover:text-green-300 transition-colors">Help Center <span class="text-xs opacity-50">↗</span></h3>
							<p class="text-white/60 text-sm mt-1">Edit and publish support articles for your customers</p>
						</div>
					</a>
				{:else}
					<div class="flex items-start gap-4 p-4 rounded-lg bg-white/5 border border-white/10">
						<span class="text-3xl opacity-50">📝</span>
						<div>
							<h3 class="text-white/50 font-semibold">Help Center</h3>
							<p class="text-white/40 text-sm mt-1">Create your portal account first to access the Help Center</p>
						</div>
					</div>
				{/if}
			</div>

			<!-- Portal Account Status -->
			<div class="bg-white/5 rounded-lg p-4 space-y-4">
				<h3 class="text-white font-medium">Portal Account</h3>
				{#if newPassword}
					<div class="bg-green-500/20 border border-green-500/50 rounded-lg p-4 space-y-3">
						<p class="text-green-300 font-semibold">Password generated:</p>
						<div class="flex items-center gap-2">
							<code class="bg-black/30 px-3 py-2 rounded font-mono text-white flex-1 text-sm">{newPassword}</code>
							<button onclick={copyPassword} class="px-4 py-2 bg-white/10 rounded hover:bg-white/20 text-white transition-colors text-sm">Copy</button>
						</div>
						<p class="text-white/60 text-xs">Save this password now - it won't be shown again.</p>
					</div>
				{/if}
				{#if portalStatus}
					<div class="flex flex-wrap items-center gap-4 text-sm">
						<div><span class="text-white/50">Status:</span> <span class="text-white font-medium {portalStatus.hasAccount ? 'text-green-400' : 'text-yellow-400'}">{portalStatus.hasAccount ? 'Active' : 'Not created'}</span></div>
						{#if portalStatus.email}<div><span class="text-white/50">Email:</span> <span class="text-white">{portalStatus.email}</span></div>{/if}
					</div>
					<div class="flex flex-wrap gap-3">
						{#if portalStatus.hasAccount}
							<button onclick={handlePortalReset} disabled={resetting} class="px-4 py-2 bg-white/10 hover:bg-white/20 rounded-lg text-white text-sm transition-colors disabled:opacity-50">{resetting ? 'Resetting...' : 'Reset Password'}</button>
						{:else}
							<button onclick={handlePortalCreate} disabled={creating} class="px-4 py-2 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg text-white text-sm font-medium hover:brightness-110 transition-all disabled:opacity-50">{creating ? 'Creating...' : 'Create Account'}</button>
						{/if}
					</div>
				{:else}
					<p class="text-white/60 text-sm">Unable to load portal status</p>
				{/if}
			</div>

			<!-- How to use the support portal -->
			<details class="bg-white/5 rounded-lg">
				<summary class="cursor-pointer p-4 text-white font-medium hover:bg-white/5 rounded-lg transition-colors">How to use the Support Portal</summary>
				<div class="px-4 pb-4 space-y-3 text-white/70 text-sm">
					<div class="space-y-2">
						<h4 class="text-white font-medium">Getting Started</h4>
						<ol class="list-decimal list-inside space-y-1">
							<li>Create your portal account using the button above (if not already done)</li>
							<li>Copy the generated password and save it securely</li>
							<li>Click "Support Dashboard" to open the portal</li>
							<li>Log in with your email and the generated password</li>
						</ol>
					</div>
					<div class="space-y-2">
						<h4 class="text-white font-medium">Managing Customer Tickets</h4>
						<ul class="list-disc list-inside space-y-1">
							<li><strong>Dashboard:</strong> Shows all open conversations and tickets</li>
							<li><strong>Conversations:</strong> Chat with customers in real-time</li>
							<li><strong>Assign:</strong> Take ownership of tickets to resolve them</li>
							<li><strong>Resolve:</strong> Mark tickets as resolved when completed</li>
						</ul>
					</div>
					<div class="space-y-2">
						<h4 class="text-white font-medium">Help Center</h4>
						<ul class="list-disc list-inside space-y-1">
							<li>Create and edit support articles for your customers</li>
							<li>Publish FAQs, guides, and documentation</li>
							<li>Help customers find answers without creating tickets</li>
						</ul>
					</div>
					<div class="space-y-2">
						<h4 class="text-white font-medium">Mobile App</h4>
						<p>Respond to tickets on the go:</p>
						<ul class="list-disc list-inside space-y-1">
							<li><a href="https://apps.apple.com/app/chatwoot/id1495796682" target="_blank" rel="noopener" class="text-blue-400 hover:underline">iOS App Store</a></li>
							<li><a href="https://play.google.com/store/apps/details?id=com.chatwoot.app" target="_blank" rel="noopener" class="text-blue-400 hover:underline">Android Play Store</a></li>
							<li>Server URL: <code class="bg-black/30 px-1.5 py-0.5 rounded text-xs">{CHATWOOT_BASE_URL}</code></li>
						</ul>
					</div>
				</div>
			</details>
		</section>

		<!-- Notifications Section -->
		<section id="notifications" class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20 space-y-6 scroll-mt-4">
			<h2 class="text-2xl font-bold text-white">Notifications</h2>
			<p class="text-white/60 text-sm">Get alerted when customers need support</p>

			<div class="space-y-4">
				<label class="flex items-start gap-4 p-4 rounded-lg border cursor-pointer transition-all {notifyEmail ? 'bg-blue-500/20 border-blue-500/50' : 'bg-white/5 border-white/20 hover:border-white/40'}">
					<input type="checkbox" bind:checked={notifyEmail} disabled={!accountEmail} class="mt-1 w-5 h-5" />
					<div class="flex-1">
						<div class="flex items-center gap-2"><span class="text-2xl">📧</span><span class="text-white font-medium">Email</span><span class="text-xs bg-green-500/30 text-green-300 px-2 py-0.5 rounded">Free</span></div>
						{#if accountEmail}<p class="text-white/60 text-sm mt-1">Notifications to <span class="text-white">{accountEmail}</span></p>
							{#if notifyEmail}<button type="button" onclick={(e) => { e.preventDefault(); e.stopPropagation(); handleTestChannel('email'); }} disabled={testingChannel === 'email'} class="mt-2 px-3 py-1 text-xs bg-white/10 hover:bg-white/20 rounded border border-white/20 text-white/80 disabled:opacity-50">{testingChannel === 'email' ? 'Sending...' : 'Send Test'}</button>{/if}
						{:else}<p class="text-yellow-400/80 text-sm mt-1">Add email in <a href="/dashboard/account/profile" class="underline">Profile</a></p>{/if}
					</div>
				</label>

				<label class="flex items-start gap-4 p-4 rounded-lg border cursor-pointer transition-all {notifyTelegram ? 'bg-blue-500/20 border-blue-500/50' : 'bg-white/5 border-white/20 hover:border-white/40'}">
					<input type="checkbox" bind:checked={notifyTelegram} class="mt-1 w-5 h-5" />
					<div class="flex-1">
						<div class="flex items-center gap-2"><span class="text-2xl">📱</span><span class="text-white font-medium">Telegram</span><span class="text-xs bg-green-500/30 text-green-300 px-2 py-0.5 rounded">Free (50/day)</span></div>
						<p class="text-white/60 text-sm mt-1">Instant notifications via Telegram</p>
						{#if notifyTelegram}
							{@const botUsername = import.meta.env.VITE_TELEGRAM_BOT_USERNAME || 'DecentCloudBot'}
							<div class="mt-3 space-y-2">
								<input type="text" bind:value={telegramChatId} placeholder="Chat ID" class="w-full px-3 py-2 bg-black/30 border border-white/20 rounded-lg text-white placeholder-white/40 focus:outline-none focus:border-blue-500" />
								<p class="text-xs text-white/50">Message <a href="https://t.me/{botUsername}" target="_blank" class="text-blue-400 hover:underline">@{botUsername}</a> with /start</p>
								{#if telegramChatId}<button type="button" onclick={(e) => { e.preventDefault(); e.stopPropagation(); handleTestChannel('telegram'); }} disabled={testingChannel === 'telegram'} class="mt-2 px-3 py-1 text-xs bg-white/10 hover:bg-white/20 rounded border border-white/20 text-white/80 disabled:opacity-50">{testingChannel === 'telegram' ? 'Sending...' : 'Send Test'}</button>{/if}
							</div>
						{/if}
					</div>
				</label>

				<label class="flex items-start gap-4 p-4 rounded-lg border cursor-pointer transition-all {notifySms ? 'bg-blue-500/20 border-blue-500/50' : 'bg-white/5 border-white/20 hover:border-white/40'}">
					<input type="checkbox" bind:checked={notifySms} class="mt-1 w-5 h-5" />
					<div class="flex-1">
						<div class="flex items-center gap-2"><span class="text-2xl">💬</span><span class="text-white font-medium">SMS</span><span class="text-xs bg-yellow-500/30 text-yellow-300 px-2 py-0.5 rounded">5 free/day</span></div>
						<p class="text-white/60 text-sm mt-1">SMS alerts to your phone</p>
						{#if notifySms}
							<div class="mt-3 space-y-2">
								<input type="tel" bind:value={notifyPhone} placeholder="+1 555-123-4567" class="w-full px-3 py-2 bg-black/30 border border-white/20 rounded-lg text-white placeholder-white/40 focus:outline-none focus:border-blue-500" />
								{#if notifyPhone}<button type="button" onclick={(e) => { e.preventDefault(); e.stopPropagation(); handleTestChannel('sms'); }} disabled={testingChannel === 'sms'} class="mt-2 px-3 py-1 text-xs bg-white/10 hover:bg-white/20 rounded border border-white/20 text-white/80 disabled:opacity-50">{testingChannel === 'sms' ? 'Sending...' : 'Send Test'}</button>{/if}
							</div>
						{/if}
					</div>
				</label>
			</div>

			{#if usage}
				<div class="grid grid-cols-3 gap-4 text-center bg-white/5 rounded-lg p-4">
					<div><div class="text-xl font-bold text-white">{usage.emailCount}</div><div class="text-white/60 text-xs">Email</div></div>
					<div><div class="text-xl font-bold text-white">{usage.telegramCount}/{usage.telegramLimit}</div><div class="text-white/60 text-xs">Telegram</div></div>
					<div><div class="text-xl font-bold text-white">{usage.smsCount}/{usage.smsLimit}</div><div class="text-white/60 text-xs">SMS</div></div>
				</div>
			{/if}

			<button onclick={saveNotifications} disabled={savingNotif} class="px-6 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 transition-all disabled:opacity-50">{savingNotif ? 'Saving...' : 'Save Notifications'}</button>
		</section>

		<!-- Help Center Profile Section (moved to bottom - one-time setup) -->
		<section id="helpcenter" class="scroll-mt-4">
			<form onsubmit={saveOnboarding} class="space-y-6">
				<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20 space-y-6">
					<div>
						<h2 class="text-2xl font-bold text-white">Help Center Profile</h2>
						<p class="text-white/60 text-sm mt-1">Configure your provider profile - this auto-generates a help article for customers</p>
					</div>

					<div class="grid md:grid-cols-2 gap-6">
						<div>
							<span class="block text-white/80 mb-2">Support Email <span class="text-red-400">*</span></span>
							{#if accountEmail}
								<div class="w-full px-4 py-3 bg-white/5 border border-white/10 rounded-lg text-white/80">{accountEmail}</div>
								<p class="text-white/50 text-xs mt-1">Using your account email. <a href="/dashboard/account/profile" class="text-blue-400 hover:underline">Change in Profile</a></p>
							{:else}
								<div class="w-full px-4 py-3 bg-yellow-500/10 border border-yellow-500/30 rounded-lg text-yellow-300">No email set</div>
								<p class="text-yellow-400/80 text-sm mt-1">Please <a href="/dashboard/account/profile" class="underline">add your email</a> in Account settings first</p>
							{/if}
						</div>
						<div>
							<label for="support-hours" class="block text-white/80 mb-2">Support Hours <span class="text-red-400">*</span></label>
							<select id="support-hours" bind:value={supportHours} required class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white focus:outline-none focus:border-blue-400">
								<option value="">Select...</option>
								{#each supportHoursOptions as opt}<option value={opt}>{opt === 'custom' ? 'Custom...' : opt}</option>{/each}
							</select>
							{#if supportHours === 'custom'}<input type="text" bind:value={customSupportHours} placeholder="e.g., Mon-Fri 9-17 PST" class="w-full mt-2 px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:border-blue-400" />{/if}
						</div>
					</div>

					<div>
						<div class="text-white/80 mb-2">Support Channels <span class="text-red-400">*</span></div>
						<div class="grid grid-cols-2 md:grid-cols-3 gap-3">
							{#each supportChannelOptions as ch}<label class="flex items-center space-x-2 cursor-pointer"><input type="checkbox" checked={supportChannels.includes(ch)} onchange={() => supportChannels = toggleArray(supportChannels, ch)} class="w-4 h-4 rounded" /><span class="text-white/80">{ch}</span></label>{/each}
						</div>
					</div>

					<div>
						<div class="text-white/80 mb-2">Regions <span class="text-red-400">*</span></div>
						<div class="grid grid-cols-2 md:grid-cols-3 gap-3">
							{#each regionOptions as r}<label class="flex items-center space-x-2 cursor-pointer"><input type="checkbox" checked={regions.includes(r)} onchange={() => regions = toggleArray(regions, r)} class="w-4 h-4 rounded" /><span class="text-white/80">{r}</span></label>{/each}
						</div>
					</div>

					<div>
						<div class="text-white/80 mb-2">Payment Methods <span class="text-red-400">*</span></div>
						<div class="grid grid-cols-1 md:grid-cols-2 gap-3">
							{#each paymentMethodOptions as m}<label class="flex items-center space-x-2 cursor-pointer"><input type="checkbox" checked={paymentMethods.includes(m)} onchange={() => paymentMethods = toggleArray(paymentMethods, m)} class="w-4 h-4 rounded" /><span class="text-white/80">{m}</span></label>{/each}
						</div>
					</div>

					<div class="grid md:grid-cols-2 gap-6">
						<div>
							<label for="refund-policy" class="block text-white/80 mb-2">Refund Policy</label>
							<select id="refund-policy" bind:value={refundPolicy} class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white focus:outline-none focus:border-blue-400">
								<option value="">Select...</option>
								{#each refundPolicyOptions as opt}<option value={opt}>{opt === 'custom' ? 'Custom...' : opt}</option>{/each}
							</select>
							{#if refundPolicy === 'custom'}<input type="text" bind:value={customRefundPolicy} placeholder="Describe policy" class="w-full mt-2 px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:border-blue-400" />{/if}
						</div>
						<div>
							<label for="sla-guarantee" class="block text-white/80 mb-2">SLA Guarantee</label>
							<select id="sla-guarantee" bind:value={slaGuarantee} class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white focus:outline-none focus:border-blue-400">
								<option value="">Select...</option>
								{#each slaGuaranteeOptions as opt}<option value={opt}>{opt}</option>{/each}
							</select>
						</div>
					</div>

					<div>
						<div class="text-white/80 mb-2">Unique Selling Points <span class="text-white/50">(max 200 chars)</span></div>
						<div class="space-y-3">
							<input type="text" bind:value={usp1} maxlength="200" placeholder="Key differentiator #1" class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:border-blue-400" />
							<input type="text" bind:value={usp2} maxlength="200" placeholder="Key differentiator #2" class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:border-blue-400" />
							<input type="text" bind:value={usp3} maxlength="200" placeholder="Key differentiator #3" class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:border-blue-400" />
						</div>
					</div>

					<div>
						<div class="flex justify-between items-center mb-2">
							<span class="text-white/80">Common Issues / FAQ</span>
							{#if commonIssues.length < 10}<button type="button" onclick={addCommonIssue} class="text-sm px-3 py-1 bg-blue-600 rounded hover:bg-blue-700 text-white">Add</button>{/if}
						</div>
						{#each commonIssues as issue, i}
							<div class="border border-white/20 rounded-lg p-3 mb-2 space-y-2">
								<div class="flex justify-between"><span class="text-white/50 text-sm">#{i + 1}</span><button type="button" onclick={() => removeCommonIssue(i)} class="text-red-400 text-sm">Remove</button></div>
								<input type="text" bind:value={issue.question} placeholder="Question" class="w-full px-3 py-2 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:border-blue-400" />
								<textarea bind:value={issue.answer} rows="2" placeholder="Answer" class="w-full px-3 py-2 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:border-blue-400"></textarea>
							</div>
						{/each}
					</div>

					<button type="submit" disabled={savingOnboarding} class="px-6 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 transition-all disabled:opacity-50">{savingOnboarding ? 'Saving & Publishing...' : 'Save & Publish'}</button>
				</div>
			</form>
		</section>
	{/if}
</div>
