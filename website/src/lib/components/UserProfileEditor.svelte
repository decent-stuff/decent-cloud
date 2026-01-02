<script lang="ts">
	import { onMount } from "svelte";
	import { API_BASE_URL } from "$lib/services/api";
	import { UserApiClient, handleApiResponse } from "$lib/services/user-api";
	import ContactsEditor from "./ContactsEditor.svelte";
	import SocialsEditor from "./SocialsEditor.svelte";
	import ExternalKeysEditor from "./ExternalKeysEditor.svelte";
	import type { IdentityInfo } from "$lib/stores/auth";
	import type { Ed25519KeyIdentity } from "@dfinity/identity";

	interface Props {
		identity: IdentityInfo;
		signingIdentity: IdentityInfo;
	}

	let { identity, signingIdentity }: Props = $props();

	let profile = $state({
		displayName: "",
		bio: "",
		avatarUrl: "",
	});
	let loading = $state(false);
	let error = $state<string | null>(null);
	let successMessage = $state<string | null>(null);

	// Get username from the account
	if (!identity.account?.username) {
		throw new Error("No account username found");
	}
	const username = identity.account.username;

	const apiClient = new UserApiClient(
		signingIdentity.identity as Ed25519KeyIdentity,
	);

	// Fetch existing profile
	onMount(async () => {
		try {
			const res = await fetch(
				`${API_BASE_URL}/api/v1/accounts/${username}/profile`,
			);
			if (res.ok) {
				const data = await res.json();
				if (data.success && data.data) {
					profile = {
						displayName: data.data.displayName || "",
						bio: data.data.bio || "",
						avatarUrl: data.data.avatarUrl || "",
					};
				}
			}
		} catch (err) {
			console.error("Failed to load profile:", err);
		}
	});

	async function handleSave() {
		loading = true;
		error = null;
		successMessage = null;

		try {
			const res = await apiClient.updateProfile(username, profile);

			if (!res.ok) {
				await handleApiResponse(res);
				return; // handleApiResponse will throw
			}

			const data = await res.json();
			if (!data.success) {
				throw new Error(data.error || "Failed to update profile");
			}

			// Update local profile state with saved data
			if (data.data) {
				profile = {
					displayName: data.data.displayName || "",
					bio: data.data.bio || "",
					avatarUrl: data.data.avatarUrl || "",
				};
			}

			successMessage = "Profile updated successfully!";
			setTimeout(() => {
				successMessage = null;
			}, 3000);
		} catch (err: unknown) {
			error =
				err instanceof Error ? err.message : "Unknown error occurred";
		} finally {
			loading = false;
		}
	}
</script>

<div class="space-y-6">
	<div
		class="bg-glass/10 backdrop-blur-lg rounded-xl p-6 border border-glass/15"
	>
		<h2 class="text-2xl font-bold text-white mb-4">Basic Information</h2>

		<div class="space-y-4">
			<div>
				<label
					for="display-name"
					class="block text-sm font-medium text-white/70 mb-2"
				>
					Display Name
				</label>
				<input
					id="display-name"
					type="text"
					bind:value={profile.displayName}
					class="w-full px-3 py-2 bg-glass/5 border border-glass/15 rounded-lg text-white placeholder-white/40 focus:ring-2 focus:ring-primary-500 focus:border-transparent"
					placeholder="Your display name"
				/>
			</div>

			<div>
				<label
					for="bio"
					class="block text-sm font-medium text-white/70 mb-2"
					>Bio</label
				>
				<textarea
					id="bio"
					bind:value={profile.bio}
					class="w-full px-3 py-2 bg-glass/5 border border-glass/15 rounded-lg text-white placeholder-white/40 focus:ring-2 focus:ring-primary-500 focus:border-transparent"
					rows={4}
					placeholder="Tell us about yourself"
				></textarea>
			</div>

			<div>
				<label
					for="avatar-url"
					class="block text-sm font-medium text-white/70 mb-2"
				>
					Avatar URL
				</label>
				<input
					id="avatar-url"
					type="url"
					bind:value={profile.avatarUrl}
					class="w-full px-3 py-2 bg-glass/5 border border-glass/15 rounded-lg text-white placeholder-white/40 focus:ring-2 focus:ring-primary-500 focus:border-transparent"
					placeholder="https://example.com/avatar.png"
				/>
			</div>
		</div>

		{#if error}
			<div
				class="mt-4 p-3 bg-red-500/20 border border-red-500/30 rounded text-red-400"
			>
				{error}
			</div>
		{/if}

		{#if successMessage}
			<div
				class="mt-4 p-3 bg-green-500/20 border border-green-500/30 rounded text-green-400"
			>
				{successMessage}
			</div>
		{/if}

		<button
			onclick={handleSave}
			disabled={loading}
			class="mt-6 px-6 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
		>
			{loading ? "Saving..." : "Save Profile"}
		</button>
	</div>

	<ContactsEditor {username} {apiClient} />
	<SocialsEditor {username} {apiClient} />
	<ExternalKeysEditor {username} {apiClient} />
</div>
