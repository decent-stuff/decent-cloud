<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { page } from "$app/stores";
	import { authStore } from "$lib/stores/auth";
	import { navigateToLogin } from "$lib/utils/navigation";
	import SettingsTabs from "$lib/components/SettingsTabs.svelte";
	import { Ed25519KeyIdentity } from "@dfinity/identity";
	import type { IdentityInfo } from "$lib/stores/auth";
	import {
		getNotificationConfig,
		updateNotificationConfig,
		testNotificationChannel,
		getNotificationUsage,
	} from "$lib/services/notification-api";
	import type { NotificationConfig, NotificationUsage } from "$lib/services/notification-api";

	let currentIdentity = $state<IdentityInfo | null>(null);
	let isAuthenticated = $state(false);
	let unsubscribe: (() => void) | null = null;
	let unsubscribeAuth: (() => void) | null = null;

	let config = $state<NotificationConfig>({
		notifyEmail: false,
		notifyTelegram: false,
		notifySms: false,
		notifyEmailAddress: "",
		telegramChatId: "",
		notifyPhone: "",
	});
	let usage = $state<NotificationUsage | null>(null);
	let loadError = $state<string | null>(null);
	let saveError = $state<string | null>(null);
	let saveSuccess = $state(false);
	let saving = $state(false);
	let testingChannel = $state<"email" | "telegram" | "sms" | null>(null);
	let testResult = $state<{ channel: string; message: string } | null>(null);

	async function loadData() {
		const info = await authStore.getSigningIdentity();
		if (!info || !(info.identity instanceof Ed25519KeyIdentity)) return;
		const identity = info.identity;

		const [cfg, usg] = await Promise.all([
			getNotificationConfig(identity),
			getNotificationUsage(identity),
		]);

		if (cfg) {
			config = {
				notifyEmail: cfg.notifyEmail,
				notifyTelegram: cfg.notifyTelegram,
				notifySms: cfg.notifySms,
				notifyEmailAddress: cfg.notifyEmailAddress ?? "",
				telegramChatId: cfg.telegramChatId ?? "",
				notifyPhone: cfg.notifyPhone ?? "",
				chatwootPortalSlug: cfg.chatwootPortalSlug,
			};
		}
		usage = usg;
	}

	onMount(() => {
		unsubscribeAuth = authStore.isAuthenticated.subscribe((isAuth) => {
			isAuthenticated = isAuth;
		});

		unsubscribe = authStore.currentIdentity.subscribe((value) => {
			currentIdentity = value;
			if (value?.account) {
				loadData().catch((e) => {
					loadError = e instanceof Error ? e.message : String(e);
				});
			}
		});
	});

	onDestroy(() => {
		unsubscribe?.();
		unsubscribeAuth?.();
	});

	function handleLogin() {
		navigateToLogin($page.url.pathname);
	}

	async function handleSave() {
		saving = true;
		saveError = null;
		saveSuccess = false;
		try {
			const info = await authStore.getSigningIdentity();
			if (!info || !(info.identity instanceof Ed25519KeyIdentity)) {
				throw new Error("No signing identity available");
			}
			await updateNotificationConfig(info.identity, {
				notifyEmail: config.notifyEmail,
				notifyTelegram: config.notifyTelegram,
				notifySms: config.notifySms,
				notifyEmailAddress: config.notifyEmailAddress || undefined,
				telegramChatId: config.telegramChatId || undefined,
				notifyPhone: config.notifyPhone || undefined,
				chatwootPortalSlug: config.chatwootPortalSlug,
			});
			saveSuccess = true;
			setTimeout(() => {
				saveSuccess = false;
			}, 3000);
		} catch (e) {
			saveError = e instanceof Error ? e.message : String(e);
		} finally {
			saving = false;
		}
	}

	async function handleTest(channel: "email" | "telegram" | "sms") {
		testingChannel = channel;
		testResult = null;
		try {
			const info = await authStore.getSigningIdentity();
			if (!info || !(info.identity instanceof Ed25519KeyIdentity)) {
				throw new Error("No signing identity available");
			}
			const result = await testNotificationChannel(info.identity, channel);
			testResult = { channel, message: result.message };
		} catch (e) {
			testResult = {
				channel,
				message: e instanceof Error ? e.message : String(e),
			};
		} finally {
			testingChannel = null;
		}
	}

	function usageLabel(channel: "telegram" | "sms" | "email"): string {
		if (!usage) return "";
		if (channel === "telegram") return `${usage.telegramCount}/${usage.telegramLimit} sent today`;
		if (channel === "sms") return `${usage.smsCount}/${usage.smsLimit} sent today`;
		return `${usage.emailCount} sent today`;
	}
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-2xl font-bold text-white tracking-tight">Notifications</h1>
		<p class="text-neutral-500">Configure how you receive alerts and updates</p>
	</div>

	<SettingsTabs />

	{#if !isAuthenticated}
		<div class="card p-8 border border-neutral-800 text-center">
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">🔔</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-neutral-400">
					Create an account or login to configure your notification preferences.
				</p>
				<button
					onclick={handleLogin}
					class="px-8 py-3 bg-gradient-to-r from-primary-500 to-primary-600 font-semibold text-white hover:brightness-110 hover:scale-105 transition-all"
				>
					Login / Create Account
				</button>
			</div>
		</div>
	{:else if currentIdentity?.account}
		{#if loadError}
			<div class="card p-4 border border-red-800 bg-red-950/30 text-red-400">
				Failed to load notification config: {loadError}
			</div>
		{/if}

		<div class="card p-6 border border-neutral-800 space-y-6">
			<h2 class="text-xl font-semibold text-white">Notification Channels</h2>

			<!-- Email -->
			<div class="space-y-3">
				<div class="flex items-center justify-between">
					<div class="flex items-center gap-3">
						<input
							type="checkbox"
							id="notify-email"
							bind:checked={config.notifyEmail}
							class="w-4 h-4 accent-primary-500"
						/>
						<label for="notify-email" class="text-white font-medium cursor-pointer">Email</label>
						{#if usage}
							<span class="text-neutral-500 text-sm">{usageLabel("email")}</span>
						{/if}
					</div>
					{#if config.notifyEmail}
						<button
							onclick={() => handleTest("email")}
							disabled={testingChannel === "email"}
							class="text-sm px-3 py-1 border border-neutral-700 text-neutral-300 hover:border-primary-500 hover:text-primary-400 transition-colors disabled:opacity-50"
						>
							{testingChannel === "email" ? "Sending…" : "Send test"}
						</button>
					{/if}
				</div>
				{#if config.notifyEmail}
					<input
						type="email"
						bind:value={config.notifyEmailAddress}
						placeholder="your@email.com"
						class="w-full px-3 py-2 bg-surface-elevated border border-neutral-700 text-white placeholder-neutral-500 focus:outline-none focus:border-primary-500 text-sm"
					/>
				{/if}
			</div>

			<!-- Telegram -->
			<div class="space-y-3">
				<div class="flex items-center justify-between">
					<div class="flex items-center gap-3">
						<input
							type="checkbox"
							id="notify-telegram"
							bind:checked={config.notifyTelegram}
							class="w-4 h-4 accent-primary-500"
						/>
						<label for="notify-telegram" class="text-white font-medium cursor-pointer"
							>Telegram</label
						>
						{#if usage}
							<span class="text-neutral-500 text-sm">{usageLabel("telegram")}</span>
						{/if}
					</div>
					{#if config.notifyTelegram}
						<button
							onclick={() => handleTest("telegram")}
							disabled={testingChannel === "telegram"}
							class="text-sm px-3 py-1 border border-neutral-700 text-neutral-300 hover:border-primary-500 hover:text-primary-400 transition-colors disabled:opacity-50"
						>
							{testingChannel === "telegram" ? "Sending…" : "Send test"}
						</button>
					{/if}
				</div>
				{#if config.notifyTelegram}
					<input
						type="text"
						bind:value={config.telegramChatId}
						placeholder="Telegram Chat ID"
						class="w-full px-3 py-2 bg-surface-elevated border border-neutral-700 text-white placeholder-neutral-500 focus:outline-none focus:border-primary-500 text-sm"
					/>
				{/if}
			</div>

			<!-- SMS -->
			<div class="space-y-3">
				<div class="flex items-center justify-between">
					<div class="flex items-center gap-3">
						<input
							type="checkbox"
							id="notify-sms"
							bind:checked={config.notifySms}
							class="w-4 h-4 accent-primary-500"
						/>
						<label for="notify-sms" class="text-white font-medium cursor-pointer">SMS</label>
						{#if usage}
							<span class="text-neutral-500 text-sm">{usageLabel("sms")}</span>
						{/if}
					</div>
					{#if config.notifySms}
						<button
							onclick={() => handleTest("sms")}
							disabled={testingChannel === "sms"}
							class="text-sm px-3 py-1 border border-neutral-700 text-neutral-300 hover:border-primary-500 hover:text-primary-400 transition-colors disabled:opacity-50"
						>
							{testingChannel === "sms" ? "Sending…" : "Send test"}
						</button>
					{/if}
				</div>
				{#if config.notifySms}
					<input
						type="tel"
						bind:value={config.notifyPhone}
						placeholder="+1234567890"
						class="w-full px-3 py-2 bg-surface-elevated border border-neutral-700 text-white placeholder-neutral-500 focus:outline-none focus:border-primary-500 text-sm"
					/>
				{/if}
			</div>

			{#if testResult}
				<div
					class="p-3 border text-sm {testResult.message.toLowerCase().includes('fail') ||
					testResult.message.toLowerCase().includes('error')
						? 'border-red-800 bg-red-950/30 text-red-400'
						: 'border-green-800 bg-green-950/30 text-green-400'}"
				>
					{testResult.channel}: {testResult.message}
				</div>
			{/if}

			{#if saveError}
				<div class="p-3 border border-red-800 bg-red-950/30 text-red-400 text-sm">
					{saveError}
				</div>
			{/if}

			{#if saveSuccess}
				<div class="p-3 border border-green-800 bg-green-950/30 text-green-400 text-sm">
					Notification settings saved.
				</div>
			{/if}

			<button
				onclick={handleSave}
				disabled={saving}
				class="px-6 py-2 bg-primary-500 hover:bg-primary-400 text-white font-semibold transition-colors disabled:opacity-50"
			>
				{saving ? "Saving…" : "Save"}
			</button>
		</div>
	{:else}
		<p class="text-neutral-500">Loading...</p>
	{/if}
</div>
