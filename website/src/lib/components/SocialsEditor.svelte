<script lang="ts">
	import { onMount } from 'svelte';
	import { API_BASE_URL } from '$lib/services/api';
	import { handleApiResponse, type UserApiClient } from '$lib/services/user-api';

	interface Social {
		id: number;
		platform: string;
		username: string;
		profile_url: string | null;
	}

	interface Props {
		username: string;
		apiClient: UserApiClient;
	}

	let { username, apiClient }: Props = $props();

	let socials = $state<Social[]>([]);
	let newSocial = $state({ platform: 'twitter', username: '', profile_url: '' });
	let loading = $state(false);
	let error = $state<string | null>(null);
	let successMessage = $state<string | null>(null);

	onMount(() => {
		loadSocials();
	});

	async function loadSocials() {
		try {
			const res = await fetch(`${API_BASE_URL}/api/v1/accounts/${username}/socials`);
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
		successMessage = null;

		try {
			const res = await apiClient.upsertSocial(username, {
				platform: newSocial.platform,
				username: newSocial.username,
				profile_url: newSocial.profile_url || undefined
			});

			if (!res.ok) {
				await handleApiResponse(res);
				return;
			}

			const data = await res.json();
			if (!data.success) {
				throw new Error(data.error || 'Failed to add social account');
			}

			newSocial = { platform: 'twitter', username: '', profile_url: '' };
			await loadSocials();
			successMessage = 'Social account added successfully!';
			setTimeout(() => {
				successMessage = null;
			}, 3000);
		} catch (err: unknown) {
			error = err instanceof Error ? err.message : 'Failed to add social account';
		} finally {
			loading = false;
		}
	}

	async function handleDelete(id: number, platform: string) {
		if (!confirm(`Delete ${platform} account?`)) return;

		error = null;
		successMessage = null;

		try {
			const res = await apiClient.deleteSocial(username, id);

			if (!res.ok) {
				await handleApiResponse(res);
				return;
			}

			const data = await res.json();
			if (!data.success) {
				throw new Error(data.error || 'Failed to delete social account');
			}
			await loadSocials();
			successMessage = 'Social account deleted successfully!';
			setTimeout(() => {
				successMessage = null;
			}, 3000);
		} catch (err: unknown) {
			error = err instanceof Error ? err.message : 'Failed to delete social account';
		}
	}
</script>

<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
	<h2 class="text-2xl font-bold text-white mb-4">Social Media</h2>

	<!-- Socials list -->
	<div class="space-y-2 mb-4">
		{#if socials.length === 0}
			<p class="text-white/50 text-sm">No social media accounts added yet.</p>
		{/if}
		{#each socials as social}
			<div class="flex items-center justify-between p-3 bg-white/5 rounded-lg border border-white/10">
				<div class="text-white">
					<span class="font-medium text-white/70">{social.platform}:</span>
					{social.username}
					{#if social.profile_url}
						<a
							href={social.profile_url}
							target="_blank"
							rel="noopener noreferrer"
							class="ml-2 text-blue-400 hover:text-blue-300 hover:underline text-sm"
						>
							View Profile
						</a>
					{/if}
				</div>
				<button
					onclick={() => handleDelete(social.id, social.platform)}
					class="text-red-400 hover:text-red-300 transition-colors"
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
				class="px-3 py-2 bg-white/5 border border-white/20 rounded-lg text-white focus:ring-2 focus:ring-blue-500 focus:border-transparent [&>option]:bg-gray-800 [&>option]:text-white"
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
				class="flex-1 px-3 py-2 bg-white/5 border border-white/20 rounded-lg text-white placeholder-white/40 focus:ring-2 focus:ring-blue-500 focus:border-transparent"
				placeholder="Username"
			/>
		</div>
		<div class="flex gap-2">
			<input
				type="url"
				bind:value={newSocial.profile_url}
				class="flex-1 px-3 py-2 bg-white/5 border border-white/20 rounded-lg text-white placeholder-white/40 focus:ring-2 focus:ring-blue-500 focus:border-transparent"
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
		<div class="mt-4 p-3 bg-red-500/20 border border-red-500/30 rounded text-red-400">
			{error}
		</div>
	{/if}

	{#if successMessage}
		<div class="mt-4 p-3 bg-green-500/20 border border-green-500/30 rounded text-green-400">
			{successMessage}
		</div>
	{/if}
</div>
