<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { browser } from '$app/environment';
	import { goto } from '$app/navigation';
	import { authStore } from '$lib/stores/auth';
	import { Ed25519KeyIdentity } from '@dfinity/identity';
	import { signRequest } from '$lib/services/auth-api';
	import {
		getUserNotifications,
		getUnreadNotificationCount,
		markNotificationsRead,
		hexEncode,
		type UserNotification
	} from '$lib/services/api';
	import Icon from '$lib/components/Icons.svelte';
	import UnreadBadge from '$lib/components/UnreadBadge.svelte';
	import { formatNotificationTime } from '$lib/utils/notification-time';

	const POLL_INTERVAL_MS = 60_000;
	const MAX_NOTIFICATIONS = 10;

	let isOpen = $state(false);
	let unreadCount = $state(0);
	let notifications = $state<UserNotification[]>([]);
	let loading = $state(false);
	let activeIdentity = $state<import('$lib/stores/auth').IdentityInfo | null>(null);
	let pollTimer: ReturnType<typeof setInterval> | null = null;
	let unsubIdentity: (() => void) | null = null;
	let dropdownEl = $state<HTMLDivElement | null>(null);

	const unread = $derived(notifications.filter((n) => !n.readAt));
	const read = $derived(notifications.filter((n) => n.readAt));

	function notificationIcon(type: string): import('$lib/components/Icons.svelte').IconName {
		switch (type) {
			case 'contract_status': return 'file';
			case 'contract_provisioned': return 'server';
			case 'rental_request': return 'inbox';
			case 'offering_inquiry': return 'inbox';
			case 'password_reset_complete': return 'key';
			case 'auto_renewed': return 'refresh';
			default: return 'bell';
		}
	}

	async function getSignedHeaders(method: string, path: string) {
		if (!activeIdentity?.identity || !(activeIdentity.identity instanceof Ed25519KeyIdentity)) {
			throw new Error('No active Ed25519 identity');
		}
		const signed = await signRequest(activeIdentity.identity, method, path);
		return signed.headers;
	}

	function pubkeyHex(): string {
		if (!activeIdentity?.publicKeyBytes) throw new Error('No public key');
		return hexEncode(activeIdentity.publicKeyBytes);
	}

	async function fetchUnreadCount() {
		if (!activeIdentity?.publicKeyBytes) return;
		try {
			const hex = pubkeyHex();
			const path = `/api/v1/users/${hex}/notifications/unread-count`;
			const headers = await getSignedHeaders('GET', path);
			unreadCount = await getUnreadNotificationCount(headers, hex);
		} catch {
			// Silently ignore poll failures - network issues shouldn't crash the UI
		}
	}

	async function loadNotifications() {
		if (!activeIdentity?.publicKeyBytes) return;
		loading = true;
		try {
			const hex = pubkeyHex();
			const path = `/api/v1/users/${hex}/notifications`;
			const headers = await getSignedHeaders('GET', path);
			const all = await getUserNotifications(headers, hex);
			notifications = all.slice(0, MAX_NOTIFICATIONS);
			unreadCount = notifications.filter((n) => !n.readAt).length;
		} finally {
			loading = false;
		}
	}

	async function open() {
		isOpen = true;
		await loadNotifications();
	}

	function close() {
		isOpen = false;
	}

	async function markAllRead() {
		if (!activeIdentity?.publicKeyBytes || notifications.length === 0) return;
		try {
			const hex = pubkeyHex();
			const path = `/api/v1/users/${hex}/notifications/mark-read`;
			const headers = await getSignedHeaders('POST', path);
			await markNotificationsRead(headers, hex, []);
			notifications = notifications.map((n) => ({ ...n, readAt: Date.now() * 1_000_000 }));
			unreadCount = 0;
		} catch (err) {
			throw new Error(`Failed to mark all read: ${err}`);
		}
	}

	async function handleNotificationClick(notification: UserNotification) {
		// Mark this notification read if not already
		if (!notification.readAt && activeIdentity?.publicKeyBytes) {
			try {
				const hex = pubkeyHex();
				const path = `/api/v1/users/${hex}/notifications/mark-read`;
				const headers = await getSignedHeaders('POST', path);
				await markNotificationsRead(headers, hex, [notification.id]);
				notifications = notifications.map((n) =>
					n.id === notification.id ? { ...n, readAt: Date.now() * 1_000_000 } : n
				);
				unreadCount = Math.max(0, unreadCount - 1);
			} catch {
				// Don't block navigation on mark-read failure
			}
		}

		if (notification.contractId) {
			close();
			goto(`/dashboard/rentals/${notification.contractId}`);
		} else if (notification.offeringId) {
			close();
			goto(`/dashboard/marketplace/${notification.offeringId}`);
		}
	}

	function handleClickOutside(e: MouseEvent) {
		if (isOpen && dropdownEl && !dropdownEl.contains(e.target as Node)) {
			close();
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape' && isOpen) {
			close();
		}
	}

	onMount(() => {
		unsubIdentity = authStore.activeIdentity.subscribe((identity) => {
			activeIdentity = identity;
			if (identity?.publicKeyBytes) {
				fetchUnreadCount();
				if (pollTimer !== null) clearInterval(pollTimer);
				pollTimer = setInterval(fetchUnreadCount, POLL_INTERVAL_MS);
			} else {
				unreadCount = 0;
				notifications = [];
				if (pollTimer !== null) {
					clearInterval(pollTimer);
					pollTimer = null;
				}
			}
		});

		window.addEventListener('keydown', handleKeydown);
		document.addEventListener('click', handleClickOutside, true);
	});

	onDestroy(() => {
		unsubIdentity?.();
		if (pollTimer !== null) clearInterval(pollTimer);
		if (browser) {
			window.removeEventListener('keydown', handleKeydown);
			document.removeEventListener('click', handleClickOutside, true);
		}
	});
</script>

{#if activeIdentity}
	<div class="relative" bind:this={dropdownEl}>
		<!-- Bell button -->
		<button
			type="button"
			onclick={isOpen ? close : open}
			class="relative text-neutral-400 p-2 hover:bg-surface-hover hover:text-white transition-colors"
			aria-label="Notifications"
			aria-expanded={isOpen}
		>
			<Icon name="bell" size={20} />
			{#if unreadCount > 0}
				<span class="absolute -top-0.5 -right-0.5">
					<UnreadBadge count={unreadCount} />
				</span>
			{/if}
		</button>

		<!-- Dropdown -->
		{#if isOpen}
			<div
				role="dialog"
				aria-label="Notifications"
				aria-modal="true"
				class="absolute right-0 top-full mt-1 w-80 bg-surface border border-neutral-700 rounded-lg shadow-2xl z-50 overflow-hidden"
			>
				<!-- Header -->
				<div class="flex items-center justify-between px-4 py-3 border-b border-neutral-700">
					<span class="text-sm font-medium text-white">Notifications</span>
					{#if unread.length > 0}
						<button
							type="button"
							onclick={markAllRead}
							class="text-xs text-neutral-400 hover:text-white transition-colors"
						>
							Mark all read
						</button>
					{/if}
				</div>

				<!-- Body -->
				<div class="max-h-80 overflow-y-auto">
					{#if loading}
						<p class="px-4 py-6 text-sm text-neutral-500 text-center">Loading…</p>
					{:else if notifications.length === 0}
						<p class="px-4 py-6 text-sm text-neutral-500 text-center">No notifications</p>
					{:else}
						{#if unread.length > 0}
							<div class="px-4 py-1.5 text-xs font-medium text-neutral-500 uppercase tracking-wider">
								Unread
							</div>
							{#each unread as notification (notification.id)}
								<button
									type="button"
									onclick={() => handleNotificationClick(notification)}
									class="w-full px-4 py-3 flex items-start gap-3 text-left hover:bg-surface-hover transition-colors border-l-2 border-blue-500"
								>
									<span class="flex-shrink-0 text-blue-400 mt-0.5">
										<Icon name={notificationIcon(notification.notificationType)} size={14} />
									</span>
									<span class="flex-1 min-w-0">
										<span class="block text-sm text-white font-medium">{notification.title}</span>
										<span class="block text-xs text-neutral-400 truncate mt-0.5">
											{notification.body.length > 60 ? notification.body.slice(0, 60) + '…' : notification.body}
										</span>
									</span>
									<span class="flex-shrink-0 text-xs text-neutral-600 mt-0.5">
										{formatNotificationTime(notification.createdAt)}
									</span>
								</button>
							{/each}
						{/if}

						{#if read.length > 0}
							{#if unread.length > 0}
								<div class="border-t border-neutral-800"></div>
							{/if}
							<div class="px-4 py-1.5 text-xs font-medium text-neutral-500 uppercase tracking-wider">
								Read
							</div>
							{#each read as notification (notification.id)}
								<button
									type="button"
									onclick={() => handleNotificationClick(notification)}
									class="w-full px-4 py-3 flex items-start gap-3 text-left hover:bg-surface-hover transition-colors"
								>
									<span class="flex-shrink-0 text-neutral-600 mt-0.5">
										<Icon name={notificationIcon(notification.notificationType)} size={14} />
									</span>
									<span class="flex-1 min-w-0">
										<span class="block text-sm text-neutral-300">{notification.title}</span>
										<span class="block text-xs text-neutral-500 truncate mt-0.5">
											{notification.body.length > 60 ? notification.body.slice(0, 60) + '…' : notification.body}
										</span>
									</span>
									<span class="flex-shrink-0 text-xs text-neutral-600 mt-0.5">
										{formatNotificationTime(notification.createdAt)}
									</span>
								</button>
							{/each}
						{/if}
					{/if}
				</div>
			</div>
		{/if}
	</div>
{/if}
