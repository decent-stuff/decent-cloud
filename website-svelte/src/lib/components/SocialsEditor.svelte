<script lang="ts">
	import { onMount } from 'svelte';
	import type { UserApiClient } from '$lib/services/user-api';

	interface Social {
		platform: string;
		username: string;
		profile_url: string | null;
	}

	interface Props {
		pubkey: string;
		apiClient: UserApiClient;
	}

	let { pubkey, apiClient }: Props = $props();

	let socials = $state<Social[]>([]);
	let newSocial = $state({ platform: 'twitter', username: '', profile_url: '' });
	let loading = $state(false);
	let error = $state<string | null>(null);

	const API_BASE =
		typeof window !== 'undefined' && import.meta.env.VITE_DECENT_CLOUD_API_URL
			? import.meta.env.VITE_DECENT_CLOUD_API_URL
			: 'https://api.decent-cloud.org';

	onMount(() => {
		loadSocials();
	});

	async function loadSocials() {
		try {
			const res = await fetch(`${API_BASE}/api/v1/users/${pubkey}/socials`);
			if (res.ok) {
				const data = await res.json();
				if (data.success && data.data) {
					socials = data.data;
				}
			}
		} catch (err) {
			console.error('Failed to load socials:', err);
		}
	}

	async function handleAdd() {
		if (!newSocial.username.trim()) return;

		loading = true;
		error = null;

		try {
			const res = await apiClient.upsertSocial(pubkey, {
				platform: newSocial.platform,
				username: newSocial.username,
				profile_url: newSocial.profile_url || undefined
			});

			const data = await res.json();
			if (!data.success) {
				throw new Error(data.error || 'Failed to add social account');
			}

			newSocial = { platform: 'twitter', username: '', profile_url: '' };
			await loadSocials();
		} catch (err: unknown) {
			error = err instanceof Error ? err.message : 'Failed to add social account';
		} finally {
			loading = false;
		}
	}

	async function handleDelete(platform: string) {
		if (!confirm(`Delete ${platform} account?`)) return;

		try {
			const res = await apiClient.deleteSocial(pubkey, platform);
			const data = await res.json();
			if (!data.success) {
				throw new Error(data.error || 'Failed to delete social account');
			}
			await loadSocials();
		} catch (err: unknown) {
			error = err instanceof Error ? err.message : 'Failed to delete social account';
		}
	}
</script>

<div class="bg-white border rounded-lg p-6">
	<h2 class="text-xl font-semibold mb-4">Social Media</h2>

	<!-- Socials list -->
	<div class="space-y-2 mb-4">
		{#if socials.length === 0}
			<p class="text-gray-500 text-sm">No social media accounts added yet.</p>
		{/if}
		{#each socials as social}
			<div class="flex items-center justify-between p-3 bg-gray-50 rounded">
				<div>
					<span class="font-medium">{social.platform}:</span>
					{social.username}
					{#if social.profile_url}
						<a
							href={social.profile_url}
							target="_blank"
							rel="noopener noreferrer"
							class="ml-2 text-blue-600 hover:underline text-sm"
						>
							View Profile
						</a>
					{/if}
				</div>
				<button
					onclick={() => handleDelete(social.platform)}
					class="text-red-600 hover:text-red-800 transition-colors"
				>
					Delete
				</button>
			</div>
		{/each}
	</div>

	<!-- Add new social -->
	<div class="space-y-2">
		<div class="flex gap-2">
			<select
				bind:value={newSocial.platform}
				class="px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
			>
				<option value="twitter">Twitter</option>
				<option value="github">GitHub</option>
				<option value="discord">Discord</option>
				<option value="linkedin">LinkedIn</option>
				<option value="reddit">Reddit</option>
			</select>
			<input
				type="text"
				bind:value={newSocial.username}
				class="flex-1 px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
				placeholder="Username"
			/>
		</div>
		<div class="flex gap-2">
			<input
				type="url"
				bind:value={newSocial.profile_url}
				class="flex-1 px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
				placeholder="Profile URL (optional)"
			/>
			<button
				onclick={handleAdd}
				disabled={!newSocial.username.trim() || loading}
				class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
			>
				Add
			</button>
		</div>
	</div>

	{#if error}
		<div class="mt-4 p-3 bg-red-50 border border-red-200 rounded text-red-700">
			{error}
		</div>
	{/if}
</div>
