<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { authStore } from '$lib/stores/auth';
	import { getChatwootIdentity } from '$lib/services/chatwoot-api';
	import { bytesToHex } from '$lib/utils/identity';
	import type { IdentityInfo } from '$lib/stores/auth';

	interface Props {
		websiteToken: string;
		baseUrl?: string;
	}

	let { websiteToken, baseUrl = 'https://support.decent-cloud.org' }: Props = $props();

	let currentIdentity = $state<IdentityInfo | null>(null);
	let unsubscribe: (() => void) | null = null;
	let scriptLoaded = false;

	onMount(() => {
		unsubscribe = authStore.activeIdentity.subscribe(async (identity) => {
			currentIdentity = identity;
			if (scriptLoaded && identity) {
				await authenticateUser(identity);
			}
		});

		loadChatwootScript();
	});

	onDestroy(() => {
		unsubscribe?.();
	});

	function loadChatwootScript() {
		if (typeof window === 'undefined') return;

		const script = document.createElement('script');
		script.src = `${baseUrl}/packs/js/sdk.js`;
		script.defer = true;
		script.async = true;

		script.onload = async () => {
			scriptLoaded = true;

			// @ts-expect-error - Chatwoot global
			window.chatwootSettings = {
				hideMessageBubble: false,
				position: 'right',
				locale: 'en',
				type: 'standard'
			};

			// @ts-expect-error - Chatwoot SDK
			window.chatwootSDK.run({
				websiteToken,
				baseUrl
			});

			// Authenticate if user is logged in
			if (currentIdentity) {
				await authenticateUser(currentIdentity);
			}
		};

		document.head.appendChild(script);
	}

	async function authenticateUser(identity: IdentityInfo) {
		if (!identity.identity) return;

		try {
			const chatwootIdentity = await getChatwootIdentity(identity.identity);
			if (!chatwootIdentity) return;

			// Chatwoot requires at least one of: avatar_url, email, or name
			// Use username if available, otherwise truncated pubkey as fallback
			const pubkeyHex = bytesToHex(identity.publicKeyBytes);
			const name = identity.account?.username || `${pubkeyHex.slice(0, 8)}...${pubkeyHex.slice(-6)}`;
			const email = identity.account?.email;

			// @ts-expect-error - Chatwoot global
			if (window.$chatwoot) {
				// @ts-expect-error - Chatwoot global
				window.$chatwoot.setUser(chatwootIdentity.identifier, {
					identifier_hash: chatwootIdentity.identifierHash,
					name,
					email
				});
			}
		} catch (error) {
			console.error('Failed to authenticate Chatwoot user:', error);
		}
	}
</script>
