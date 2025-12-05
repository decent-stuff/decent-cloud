<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { page } from '$app/stores';
	import { authStore } from '$lib/stores/auth';
	import { navigateToLogin } from '$lib/utils/navigation';
	import type { IdentityInfo } from '$lib/stores/auth';
	import {
		getNotificationConfig,
		getNotificationUsage,
		updateNotificationConfig,
		type NotificationConfig,
		type NotificationUsage
	} from '$lib/services/notification-api';

	let currentIdentity = $state<IdentityInfo | null>(null);
	let isAuthenticated = $state(false);
	let unsubscribe: (() => void) | null = null;
	let unsubscribeAuth: (() => void) | null = null;

	let loading = $state(true);
	let saving = $state(false);
	let error = $state<string | null>(null);
	let success = $state<string | null>(null);

	// Form state - multi-channel booleans
	let notifyTelegram = $state(false);
	let notifyEmail = $state(false);
	let notifySms = $state(false);
	let telegramChatId = $state('');
	let notifyPhone = $state('');
	let usage = $state<NotificationUsage | null>(null);

	// Account email comes from the account, not a separate field
	const accountEmail = $derived(currentIdentity?.account?.email);

	onMount(() => {
		unsubscribeAuth = authStore.isAuthenticated.subscribe((isAuth) => {
			isAuthenticated = isAuth;
		});

		unsubscribe = authStore.currentIdentity.subscribe((value) => {
			currentIdentity = value;
			if (value?.identity) {
				loadConfig();
			}
		});
	});

	onDestroy(() => {
		unsubscribe?.();
		unsubscribeAuth?.();
	});

	async function loadConfig() {
		if (!currentIdentity?.identity) return;
		loading = true;
		error = null;
		try {
			const [config, usageData] = await Promise.all([
				getNotificationConfig(currentIdentity.identity),
				getNotificationUsage(currentIdentity.identity)
			]);
			if (config) {
				notifyTelegram = config.notifyTelegram;
				notifyEmail = config.notifyEmail;
				notifySms = config.notifySms;
				telegramChatId = config.telegramChatId || '';
				notifyPhone = config.notifyPhone || '';
			}
			usage = usageData;
		} catch (e) {
			// No config yet is fine
			console.log('No existing notification config');
		} finally {
			loading = false;
		}
	}

	async function handleSave() {
		if (!currentIdentity?.identity) return;
		saving = true;
		error = null;
		success = null;

		try {
			await updateNotificationConfig(currentIdentity.identity, {
				notifyTelegram,
				notifyEmail,
				notifySms,
				telegramChatId: notifyTelegram ? telegramChatId : undefined,
				notifyPhone: notifySms ? notifyPhone : undefined
			});
			success = 'Notification settings saved!';
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to save settings';
		} finally {
			saving = false;
		}
	}

	function handleLogin() {
		navigateToLogin($page.url.pathname);
	}
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">Notifications</h1>
		<p class="text-white/60">Configure how you receive support escalation alerts</p>
	</div>

	{#if !isAuthenticated}
		<div
			class="bg-white/10 backdrop-blur-lg rounded-xl p-8 border border-white/20 text-center"
		>
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">🔔</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-white/70">
					Create an account or login to configure notification preferences.
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
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-8 border border-white/20">
			<p class="text-white/60">Loading...</p>
		</div>
	{:else}
		<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20 space-y-6">
			<h2 class="text-xl font-semibold text-white">Notification Channels</h2>
			<p class="text-white/60 text-sm">
				Select one or more channels to receive alerts when a customer conversation is escalated.
			</p>

			{#if error}
				<div class="p-4 bg-red-500/20 border border-red-500/50 rounded-lg text-red-200">
					{error}
				</div>
			{/if}

			{#if success}
				<div class="p-4 bg-green-500/20 border border-green-500/50 rounded-lg text-green-200">
					{success}
				</div>
			{/if}

			<!-- Channel Selection -->
			<div class="space-y-4">
				<!-- Email Option -->
				<label
					class="flex items-start gap-4 p-4 rounded-lg border cursor-pointer transition-all {notifyEmail
						? 'bg-blue-500/20 border-blue-500/50'
						: 'bg-white/5 border-white/20 hover:border-white/40'}"
				>
					<input
						type="checkbox"
						bind:checked={notifyEmail}
						disabled={!accountEmail}
						class="mt-1 w-5 h-5"
					/>
					<div class="flex-1">
						<div class="flex items-center gap-2">
							<span class="text-2xl">📧</span>
							<span class="text-white font-medium">Email</span>
							<span class="text-xs bg-green-500/30 text-green-300 px-2 py-0.5 rounded">
								Free
							</span>
						</div>
						{#if accountEmail}
							<p class="text-white/60 text-sm mt-1">
								Notifications will be sent to <span class="text-white">{accountEmail}</span>
							</p>
						{:else}
							<p class="text-yellow-400/80 text-sm mt-1">
								Add an email in <a href="/dashboard/account/profile" class="underline">Profile Settings</a> to enable
							</p>
						{/if}
					</div>
				</label>

				<!-- Telegram Option -->
				<label
					class="flex items-start gap-4 p-4 rounded-lg border cursor-pointer transition-all {notifyTelegram
						? 'bg-blue-500/20 border-blue-500/50'
						: 'bg-white/5 border-white/20 hover:border-white/40'}"
				>
					<input
						type="checkbox"
						bind:checked={notifyTelegram}
						class="mt-1 w-5 h-5"
					/>
					<div class="flex-1">
						<div class="flex items-center gap-2">
							<span class="text-2xl">📱</span>
							<span class="text-white font-medium">Telegram</span>
							<span class="text-xs bg-green-500/30 text-green-300 px-2 py-0.5 rounded">
								Free (50/day)
							</span>
						</div>
						<p class="text-white/60 text-sm mt-1">
							Receive instant notifications via Telegram. Reply directly to respond.
						</p>
						{#if notifyTelegram}
							{@const botUsername = import.meta.env.VITE_TELEGRAM_BOT_USERNAME || 'DecentCloudBot'}
							<div class="mt-3 space-y-2">
								<label class="block text-sm text-white/80">
									Telegram Chat ID
									<input
										type="text"
										bind:value={telegramChatId}
										placeholder="e.g. 123456789"
										class="mt-1 w-full px-3 py-2 bg-black/30 border border-white/20 rounded-lg text-white placeholder-white/40 focus:outline-none focus:border-blue-500"
									/>
								</label>
								<p class="text-xs text-white/50">
									1. Message <a
										href="https://t.me/{botUsername}"
										target="_blank"
										class="text-blue-400 hover:underline">@{botUsername}</a
									> with <code>/start</code><br />
									2. Copy the Chat ID from the bot's response
								</p>
							</div>
						{/if}
					</div>
				</label>

				<!-- SMS Option -->
				<label
					class="flex items-start gap-4 p-4 rounded-lg border cursor-pointer transition-all {notifySms
						? 'bg-blue-500/20 border-blue-500/50'
						: 'bg-white/5 border-white/20 hover:border-white/40'}"
				>
					<input
						type="checkbox"
						bind:checked={notifySms}
						class="mt-1 w-5 h-5"
					/>
					<div class="flex-1">
						<div class="flex items-center gap-2">
							<span class="text-2xl">💬</span>
							<span class="text-white font-medium">SMS</span>
							<span class="text-xs bg-yellow-500/30 text-yellow-300 px-2 py-0.5 rounded">
								5 free/day
							</span>
						</div>
						<p class="text-white/60 text-sm mt-1">
							Receive SMS alerts to your phone number
						</p>
						{#if notifySms}
							<div class="mt-3 space-y-2">
								<label class="block text-sm text-white/80">
									Phone Number
									<input
										type="tel"
										bind:value={notifyPhone}
										placeholder="+1 555-123-4567"
										class="mt-1 w-full px-3 py-2 bg-black/30 border border-white/20 rounded-lg text-white placeholder-white/40 focus:outline-none focus:border-blue-500"
									/>
								</label>
								<p class="text-xs text-white/50">
									Include country code (e.g. +1 for US)
								</p>
							</div>
						{/if}
					</div>
				</label>
			</div>

			<!-- Save Button -->
			<div class="pt-4">
				<button
					onclick={handleSave}
					disabled={saving}
					class="px-6 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
				>
					{saving ? 'Saving...' : 'Save Settings'}
				</button>
			</div>
		</div>

		<!-- Usage Stats -->
		{#if usage}
			<div class="bg-white/5 backdrop-blur-lg rounded-xl p-6 border border-white/10">
				<h3 class="text-lg font-semibold text-white mb-4">Today's Usage</h3>
				<div class="grid grid-cols-3 gap-4">
					<div class="text-center">
						<div class="text-2xl font-bold text-white">{usage.emailCount}</div>
						<div class="text-white/60 text-sm">Email</div>
						<div class="text-xs text-green-400">Unlimited</div>
					</div>
					<div class="text-center">
						<div class="text-2xl font-bold text-white">{usage.telegramCount}/{usage.telegramLimit}</div>
						<div class="text-white/60 text-sm">Telegram</div>
						<div class="text-xs {usage.telegramCount >= usage.telegramLimit ? 'text-red-400' : 'text-green-400'}">
							{usage.telegramLimit - usage.telegramCount} remaining
						</div>
					</div>
					<div class="text-center">
						<div class="text-2xl font-bold text-white">{usage.smsCount}/{usage.smsLimit}</div>
						<div class="text-white/60 text-sm">SMS</div>
						<div class="text-xs {usage.smsCount >= usage.smsLimit ? 'text-red-400' : 'text-green-400'}">
							{usage.smsLimit - usage.smsCount} remaining
						</div>
					</div>
				</div>
			</div>
		{/if}

		<!-- Usage Info -->
		<div class="bg-white/5 backdrop-blur-lg rounded-xl p-6 border border-white/10">
			<h3 class="text-lg font-semibold text-white mb-3">About Notifications</h3>
			<ul class="text-white/60 text-sm space-y-2">
				<li>
					• Notifications are sent to all selected channels when a conversation is escalated
				</li>
				<li>• You can reply directly from Telegram to respond in the chat</li>
				<li>• Email notifications link to your Chatwoot dashboard</li>
				<li>• Free tier limits reset daily at midnight UTC</li>
			</ul>
		</div>
	{/if}
</div>
