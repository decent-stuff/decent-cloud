<script lang="ts">
	import { onMount } from 'svelte';
	import { API_BASE_URL } from '$lib/services/api';
	import { handleApiResponse, type UserApiClient } from '$lib/services/user-api';

	interface Contact {
		id: number;
		contact_type: string;
		contact_value: string;
		verified: boolean;
	}

	interface Props {
		username: string;
		apiClient: UserApiClient;
	}

	let { username, apiClient }: Props = $props();

	let contacts = $state<Contact[]>([]);
	let newContact = $state({ type: 'email', value: '' });
	let loading = $state(false);
	let error = $state<string | null>(null);
	let successMessage = $state<string | null>(null);

	onMount(() => {
		loadContacts();
	});

	async function loadContacts() {
		try {
			const res = await fetch(`${API_BASE_URL}/api/v1/accounts/${username}/contacts`);
			if (res.ok) {
				const data = await res.json();
				if (data.success && data.data) {
					contacts = data.data;
				}
			}
		} catch (err) {
			console.error('Failed to load contacts:', err);
		}
	}

	async function handleAdd() {
		if (!newContact.value.trim()) return;

		loading = true;
		error = null;
		successMessage = null;

		try {
			const res = await apiClient.upsertContact(username, {
				contact_type: newContact.type,
				contact_value: newContact.value
			});
			await handleApiResponse(res);

			const data = await res.json();
			if (!data.success) {
				throw new Error(data.error || 'Failed to add contact');
			}

			newContact = { type: 'email', value: '' };
			await loadContacts();
			successMessage = 'Contact added successfully!';
			setTimeout(() => {
				successMessage = null;
			}, 3000);
		} catch (err: unknown) {
			error = err instanceof Error ? err.message : 'Failed to add contact';
		} finally {
			loading = false;
		}
	}

	async function handleDelete(id: number, type: string) {
		if (!confirm(`Delete ${type} contact?`)) return;

		error = null;
		successMessage = null;

		try {
			const res = await apiClient.deleteContact(username, id);
			await handleApiResponse(res);

			const data = await res.json();
			if (!data.success) {
				throw new Error(data.error || 'Failed to delete contact');
			}
			await loadContacts();
			successMessage = 'Contact deleted successfully!';
			setTimeout(() => {
				successMessage = null;
			}, 3000);
		} catch (err: unknown) {
			error = err instanceof Error ? err.message : 'Failed to delete contact';
		}
	}
</script>

<div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 border border-white/20">
	<h2 class="text-2xl font-bold text-white mb-4">Contact Information</h2>

	<!-- Contact list -->
	<div class="space-y-2 mb-4">
		{#if contacts.length === 0}
			<p class="text-white/50 text-sm">No contact information added yet.</p>
		{/if}
		{#each contacts as contact}
			<div class="flex items-center justify-between p-3 bg-white/5 rounded-lg border border-white/10">
				<div class="text-white">
					<span class="font-medium text-white/70">{contact.contact_type}:</span>
					{contact.contact_value}
					{#if contact.verified}
						<span class="ml-2 text-xs bg-green-500/20 text-green-400 px-2 py-1 rounded border border-green-500/30">
							Verified
						</span>
					{/if}
				</div>
				<button
					onclick={() => handleDelete(contact.id, contact.contact_type)}
					class="text-red-400 hover:text-red-300 transition-colors"
				>
					Delete
				</button>
			</div>
		{/each}
	</div>

	<!-- Add new contact -->
	<div class="flex gap-2">
		<select
			bind:value={newContact.type}
			class="px-3 py-2 bg-white/5 border border-white/20 rounded-lg text-white focus:ring-2 focus:ring-blue-500 focus:border-transparent [&>option]:bg-gray-800 [&>option]:text-white"
		>
			<option value="email">Email</option>
			<option value="phone">Phone</option>
			<option value="telegram">Telegram</option>
		</select>
		<input
			type="text"
			bind:value={newContact.value}
			class="flex-1 px-3 py-2 bg-white/5 border border-white/20 rounded-lg text-white placeholder-white/40 focus:ring-2 focus:ring-blue-500 focus:border-transparent"
			placeholder="Contact value"
		/>
		<button
			onclick={handleAdd}
			disabled={!newContact.value.trim() || loading}
			class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
		>
			Add
		</button>
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
