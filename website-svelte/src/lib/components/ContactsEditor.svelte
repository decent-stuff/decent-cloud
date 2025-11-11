<script lang="ts">
	import { onMount } from 'svelte';
	import type { UserApiClient } from '$lib/services/user-api';

	interface Contact {
		contact_type: string;
		contact_value: string;
		verified: boolean;
	}

	interface Props {
		pubkey: string;
		apiClient: UserApiClient;
	}

	let { pubkey, apiClient }: Props = $props();

	let contacts = $state<Contact[]>([]);
	let newContact = $state({ type: 'email', value: '' });
	let loading = $state(false);
	let error = $state<string | null>(null);

	const API_BASE =
		typeof window !== 'undefined' && import.meta.env.VITE_DECENT_CLOUD_API_URL
			? import.meta.env.VITE_DECENT_CLOUD_API_URL
			: 'https://api.decent-cloud.org';

	onMount(() => {
		loadContacts();
	});

	async function loadContacts() {
		try {
			const res = await fetch(`${API_BASE}/api/v1/users/${pubkey}/contacts`);
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

		try {
			const res = await apiClient.upsertContact(pubkey, {
				contact_type: newContact.type,
				contact_value: newContact.value
			});

			const data = await res.json();
			if (!data.success) {
				throw new Error(data.error || 'Failed to add contact');
			}

			newContact = { type: 'email', value: '' };
			await loadContacts();
		} catch (err: unknown) {
			error = err instanceof Error ? err.message : 'Failed to add contact';
		} finally {
			loading = false;
		}
	}

	async function handleDelete(type: string) {
		if (!confirm(`Delete ${type} contact?`)) return;

		try {
			const res = await apiClient.deleteContact(pubkey, type);
			const data = await res.json();
			if (!data.success) {
				throw new Error(data.error || 'Failed to delete contact');
			}
			await loadContacts();
		} catch (err: unknown) {
			error = err instanceof Error ? err.message : 'Failed to delete contact';
		}
	}
</script>

<div class="bg-white border rounded-lg p-6">
	<h2 class="text-xl font-semibold mb-4">Contact Information</h2>

	<!-- Contact list -->
	<div class="space-y-2 mb-4">
		{#if contacts.length === 0}
			<p class="text-gray-500 text-sm">No contact information added yet.</p>
		{/if}
		{#each contacts as contact}
			<div class="flex items-center justify-between p-3 bg-gray-50 rounded">
				<div>
					<span class="font-medium">{contact.contact_type}:</span>
					{contact.contact_value}
					{#if contact.verified}
						<span class="ml-2 text-xs bg-green-100 text-green-800 px-2 py-1 rounded">
							Verified
						</span>
					{/if}
				</div>
				<button
					onclick={() => handleDelete(contact.contact_type)}
					class="text-red-600 hover:text-red-800 transition-colors"
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
			class="px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
		>
			<option value="email">Email</option>
			<option value="phone">Phone</option>
			<option value="telegram">Telegram</option>
		</select>
		<input
			type="text"
			bind:value={newContact.value}
			class="flex-1 px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
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
		<div class="mt-4 p-3 bg-red-50 border border-red-200 rounded text-red-700">
			{error}
		</div>
	{/if}
</div>
