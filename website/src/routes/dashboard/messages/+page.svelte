<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { navigateToLogin } from '$lib/utils/navigation';
	import { authStore } from '$lib/stores/auth';
	import { messagesStore } from '$lib/stores/messages';
	import { page } from '$app/stores';
	import ThreadListItem from '$lib/components/ThreadListItem.svelte';
	import type { MessageThread } from '$lib/types/generated/MessageThread';

	let isAuthenticated = $state(false);
	let inbox = $state<MessageThread[]>([]);
	let isLoading = $state(false);
	let error = $state<string | null>(null);
	let unsubscribeAuth: (() => void) | null = null;
	let unsubscribeInbox: (() => void) | null = null;
	let unsubscribeLoading: (() => void) | null = null;
	let unsubscribeError: (() => void) | null = null;

	onMount(async () => {
		unsubscribeAuth = authStore.isAuthenticated.subscribe(async (isAuth) => {
			isAuthenticated = isAuth;
			if (isAuth) {
				await messagesStore.loadInbox();
			}
		});

		unsubscribeInbox = messagesStore.inbox.subscribe((value) => {
			inbox = value;
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
		unsubscribeInbox?.();
		unsubscribeLoading?.();
		unsubscribeError?.();
	});

	function handleLogin() {
		navigateToLogin($page.url.pathname);
	}

	function handleThreadClick(contractId: string) {
		goto(`/dashboard/rentals/${contractId}/messages`);
	}
</script>

<div class="space-y-8">
	<div>
		<h1 class="text-4xl font-bold text-white mb-2">Messages</h1>
		<p class="text-white/60">View all your conversations</p>
	</div>

	{#if !isAuthenticated}
		<div
			class="bg-white/10 backdrop-blur-lg rounded-xl p-8 border border-white/20 text-center"
		>
			<div class="max-w-md mx-auto space-y-6">
				<span class="text-6xl">🔑</span>
				<h2 class="text-2xl font-bold text-white">Login Required</h2>
				<p class="text-white/70">
					Create an account or login to view your messages.
				</p>
				<button
					onclick={handleLogin}
					class="px-8 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold text-white hover:brightness-110 hover:scale-105 transition-all"
				>
					Login / Create Account
				</button>
			</div>
		</div>
	{:else if error}
		<div
			class="bg-red-500/20 border border-red-500/30 rounded-lg p-4 text-red-400"
		>
			<p class="font-semibold">Error loading messages</p>
			<p class="text-sm mt-1">{error}</p>
		</div>
	{:else if isLoading}
		<div class="flex justify-center items-center p-8">
			<div
				class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-400"
			></div>
		</div>
	{:else if inbox.length === 0}
		<div class="text-center py-16">
			<span class="text-6xl mb-4 block">💬</span>
			<h3 class="text-2xl font-bold text-white mb-2">No conversations yet</h3>
			<p class="text-white/60 mb-6">
				Messages will appear here when you have active rental contracts
			</p>
			<a
				href="/dashboard/rentals"
				class="inline-block px-6 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold hover:brightness-110 transition-all"
			>
				View My Rentals
			</a>
		</div>
	{:else}
		<div class="space-y-3">
			{#each inbox as thread}
				<ThreadListItem
					{thread}
					unreadCount={0}
					messageCount={0}
					onClick={() => handleThreadClick(thread.contract_id)}
				/>
			{/each}
		</div>
	{/if}
</div>
