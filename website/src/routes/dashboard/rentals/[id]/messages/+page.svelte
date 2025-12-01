<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { navigateToLogin } from '$lib/utils/navigation';
	import { authStore } from '$lib/stores/auth';
	import { messagesStore } from '$lib/stores/messages';
	import { hexEncode } from '$lib/services/api';
	import MessageList from '$lib/components/MessageList.svelte';
	import MessageComposer from '$lib/components/MessageComposer.svelte';
	import type { Message } from '$lib/types/generated/Message';
	import type { MessageThread } from '$lib/types/generated/MessageThread';

	let contractId = $state($page.params.id || '');
	let isAuthenticated = $state(false);
	let currentUserPubkey = $state<string>('');
	let messages = $state<Message[]>([]);
	let currentThread = $state<MessageThread | null>(null);
	let isLoading = $state(false);
	let error = $state<string | null>(null);
	let unsubscribeAuth: (() => void) | null = null;
	let unsubscribeMessages: (() => void) | null = null;
	let unsubscribeThread: (() => void) | null = null;
	let unsubscribeLoading: (() => void) | null = null;
	let unsubscribeError: (() => void) | null = null;
	let unsubscribePage: (() => void) | null = null;

	onMount(async () => {
		unsubscribePage = page.subscribe((p) => {
			contractId = p.params.id || '';
		});

		unsubscribeAuth = authStore.isAuthenticated.subscribe(async (isAuth) => {
			isAuthenticated = isAuth;
			if (isAuth && contractId) {
				const signingIdentityInfo = await authStore.getSigningIdentity();
				if (signingIdentityInfo) {
					currentUserPubkey = hexEncode(signingIdentityInfo.publicKeyBytes);
					await messagesStore.loadContractMessages(contractId);
				}
			}
		});

		unsubscribeMessages = messagesStore.messages.subscribe((value) => {
			messages = value;
		});

		unsubscribeThread = messagesStore.currentThread.subscribe((value) => {
			currentThread = value;
		});

		unsubscribeLoading = messagesStore.isLoading.subscribe((value) => {
			isLoading = value;
		});

		unsubscribeError = messagesStore.error.subscribe((value) => {
			error = value;
		});
	});

	onDestroy(() => {
		unsubscribeAuth?.();
		unsubscribeMessages?.();
		unsubscribeThread?.();
		unsubscribeLoading?.();
		unsubscribeError?.();
		unsubscribePage?.();
	});

	function handleLogin() {
		navigateToLogin($page.url.pathname);
	}

	async function handleSendMessage(body: string) {
		await messagesStore.sendMessage(contractId, body);
	}

	async function handleMarkRead(messageId: string) {
		await messagesStore.markAsRead(messageId);
	}

	function handleBack() {
		goto('/dashboard/rentals');
	}
</script>

<div class="space-y-6">
	{#if !isAuthenticated}
		<div
			class="bg-white/10 backdrop-blur-lg rounded-xl p-8 border border-white/20 text-center"
		>
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">🔑</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-white/70">
					You must be logged in to view contract messages.
				</p>
				<button
					onclick={handleLogin}
					class="px-8 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 hover:scale-105 transition-all"
				>
					Login / Create Account
				</button>
			</div>
		</div>
	{:else}
		<div class="mb-4">
			<button
				onclick={handleBack}
				class="flex items-center gap-2 text-white/70 hover:text-white transition-colors"
			>
				<svg
					class="w-5 h-5"
					fill="none"
					stroke="currentColor"
					viewBox="0 0 24 24"
				>
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M15 19l-7-7 7-7"
					></path>
				</svg>
				<span>Back to Rentals</span>
			</button>
		</div>

		<div>
			<h1 class="text-4xl font-bold text-white mb-2">
				{#if currentThread}
					{currentThread.subject}
				{:else}
					Contract Messages
				{/if}
			</h1>
			<p class="text-white/60">
				Contract: {contractId.slice(0, 8)}...{contractId.slice(-8)}
			</p>
		</div>

		{#if error}
			<div
				class="bg-red-500/20 border border-red-500/30 rounded-lg p-4 text-red-400"
			>
				<p class="font-semibold">Error loading messages</p>
				<p class="text-sm mt-1">{error}</p>
			</div>
		{/if}

		<div
			class="bg-white/10 backdrop-blur-lg rounded-xl border border-white/20 flex flex-col h-[600px]"
		>
			{#if isLoading}
				<div class="flex-1 flex justify-center items-center">
					<div
						class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-400"
					></div>
				</div>
			{:else}
				<MessageList
					{messages}
					{currentUserPubkey}
					onMarkRead={handleMarkRead}
				/>
				<MessageComposer onSend={handleSendMessage} />
			{/if}
		</div>
	{/if}
</div>
