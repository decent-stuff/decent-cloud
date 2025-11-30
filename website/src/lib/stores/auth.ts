/**
 * Authentication Store - Account & Key Management
 *
 * ARCHITECTURE (Tree Structure):
 *
 * Backend:
 *   Account @alice
 *     ├─ Key K1 (from laptop)
 *     ├─ Key K2 (from phone)
 *     └─ Key K3 (from desktop)
 *   Account @bob
 *     ├─ Key K4 (from work laptop)
 *     └─ Key K5 (from tablet)
 *
 * Constraints:
 *   - Each account has 1-10 keys (multi-device support)
 *   - Each key belongs to EXACTLY ONE account (tree, not graph)
 *   - Database enforces: UNIQUE(public_key) across all accounts
 *   - Cannot remove last active key (prevents lockout)
 *
 * Local Storage:
 *   - User stores multiple seed phrases in browser localStorage
 *   - Each seed phrase generates one Ed25519 keypair
 *   - Each keypair is registered to exactly one account
 *   - User can access multiple accounts by managing multiple seed phrases
 *
 * Multi-Device Workflow:
 *   1. Create account @alice on laptop (seed phrase 1 → K1)
 *   2. On phone: generate new seed phrase 2 → K2
 *   3. Add K2 to @alice account (requires signing with K1)
 *   4. Both devices can now access @alice independently
 *
 * Key Hierarchy: ALL KEYS ARE EQUAL
 *   - No "master key" concept
 *   - Any active key can add/remove other keys
 *   - Prevents "lost master key = lost account" scenario
 */

import { writable, derived, get } from 'svelte/store';
import type { Identity } from '@dfinity/agent';
import type { Principal } from '@dfinity/principal';
import { Ed25519KeyIdentity } from '@dfinity/identity';
import { generateMnemonic } from 'bip39';
import {
	addSeedPhrase as persistSeedPhrase,
	clearStoredSeedPhrases,
	filterStoredSeedPhrases,
	getStoredSeedPhrases,
	setStoredSeedPhrases
} from '../utils/seed-storage';
import { identityFromSeed, bytesToHex, hexToBytes } from '../utils/identity';
import { API_BASE_URL } from '../services/api';

/**
 * Account information from backend (1 account → 1-10 keys)
 */
export interface AccountInfo {
	id: string; // Account ID (hex)
	username: string; // Human-readable username
	createdAt: number; // Timestamp in nanoseconds
	updatedAt: number; // Timestamp in nanoseconds
	publicKeys: Array<{
		id: string;
		publicKey: string;
		isActive: boolean;
		addedAt: number; // Timestamp in nanoseconds
		deviceName?: string; // Stored in backend database
		disabledAt?: number; // Timestamp in nanoseconds
		disabledByKeyId?: string;
	}>;
}

/**
 * Local identity (seed phrase or OAuth) that generates one keypair for one account
 * A user can have multiple IdentityInfo objects (multiple seed phrases) to access multiple accounts
 */
export interface IdentityInfo {
	identity: Ed25519KeyIdentity;
	principal: Principal;
	type: 'seedPhrase' | 'oauth';
	name?: string;
	displayName?: string;
	publicKeyBytes: Uint8Array;
	secretKeyRaw: Uint8Array;
	seedPhrase?: string; // Only for seedPhrase type, not for oauth
	account?: AccountInfo; // Backend account this keypair is registered to (1:1 mapping)
}

export interface AuthenticatedIdentityResult {
	success: true;
	identity: Identity;
	publicKeyBytes: Uint8Array;
	secretKeyRaw: Uint8Array;
}

function createAuthStore() {
	const identities = writable<IdentityInfo[]>([]);
	// Single identity store - replaces dual currentIdentity/signingIdentity pattern
	// Since II is removed, viewing and signing identities are always the same
	const activeIdentity = writable<IdentityInfo | null>(null);
	const errorMessage = writable<string | null>(null);

	const isAuthenticated = derived(activeIdentity, ($active) => $active !== null);

	function addIdentity(
		identity: Ed25519KeyIdentity,
		publicKeyBytes: Uint8Array,
		secretKeyRaw: Uint8Array,
		seedPhrase: string
	) {
		const principal = identity.getPrincipal();

		// Check if this identity already exists (by public key)
		const identitiesList = get(identities);
		const publicKeyHex = bytesToHex(publicKeyBytes);
		const existing = identitiesList.find((i) => {
			const existingKeyHex = bytesToHex(i.publicKeyBytes);
			return existingKeyHex === publicKeyHex;
		});

		// If identity exists, preserve its account data
		const newIdentity: IdentityInfo = {
			identity,
			principal,
			type: 'seedPhrase',
			publicKeyBytes,
			secretKeyRaw,
			seedPhrase,
			...(existing?.account && { account: existing.account })
		};

		// Update existing or add new
		if (existing) {
			identities.update((prev) =>
				prev.map((i) =>
					i.principal.toString() === existing.principal.toString() ? newIdentity : i
				)
			);
		} else {
			identities.update((prev) => [...prev, newIdentity]);
		}

		const current = get(activeIdentity);
		if (!current || current.principal.toString() === principal.toString()) {
			activeIdentity.set(newIdentity);
		}
	}

	async function fetchUserProfile(publicKeyBytes: Uint8Array): Promise<string | null> {
		try {
			const current = get(activeIdentity);

			// If account exists, fetch from account profile endpoint
			const res = await fetch(`${API_BASE_URL}/api/v1/accounts/${current?.account?.username}/profile`);
			if (res.ok) {
				const data = await res.json();
				if (data.success && data.data?.displayName) {
					return data.data.displayName;
				}
			}
		} catch (error) {
			console.error('Failed to fetch user profile:', error);
		}
		return null;
	}

	async function loadAccountForIdentity(identityInfo: IdentityInfo): Promise<AccountInfo | null> {
		if (!identityInfo.publicKeyBytes) return null;

		try {
			const { getAccountByPublicKey } = await import('../services/account-api');
			const publicKeyHex = bytesToHex(identityInfo.publicKeyBytes);

			const account = await getAccountByPublicKey(publicKeyHex);
			return account;
		} catch (error) {
			console.error('Failed to load account:', error);
			return null;
		}
	}

	async function registerNewAccount(
		identity: Ed25519KeyIdentity,
		username: string,
		email: string,
		seedPhrase: string
	): Promise<AccountInfo> {
		const { registerAccount } = await import('../services/account-api');
		const account = await registerAccount(identity, username, email);

		// Persist seed phrase to localStorage so it survives page reload
		persistSeedPhrase(seedPhrase);

		// Get public key bytes for this identity
		const publicKeyBytes = new Uint8Array(identity.getPublicKey().rawKey);
		const secretKeyRaw = new Uint8Array(identity.getKeyPair().secretKey);
		const principal = identity.getPrincipal();

		const identityWithAccount: IdentityInfo = {
			identity,
			principal,
			type: 'seedPhrase',
			publicKeyBytes,
			secretKeyRaw,
			seedPhrase,
			account
		};

		// Check if identity already exists
		const identitiesList = get(identities);
		const publicKeyHex = bytesToHex(publicKeyBytes);
		const existing = identitiesList.find((id) => {
			const idKeyHex = bytesToHex(id.publicKeyBytes);
			return idKeyHex === publicKeyHex;
		});

		if (existing) {
			// Update existing identity with account
			identities.update((prev) =>
				prev.map((id) =>
					id.principal.toString() === existing.principal.toString()
						? identityWithAccount
						: id
				)
			);
		} else {
			// Add new identity with account
			identities.update((prev) => [...prev, identityWithAccount]);
		}

		// Set as active identity
		activeIdentity.set(identityWithAccount);

		console.log('[registerNewAccount] Account registered and set:', {
			username: account.username,
			hasAccount: !!identityWithAccount.account,
			seedPhrasePersisted: true
		});

		return account;
	}

	async function loadAccountByUsername(username: string): Promise<AccountInfo | null> {
		try {
			const { getAccount } = await import('../services/account-api');
			const account = await getAccount(username);

			if (account) {
				// Update active identity with account info
				activeIdentity.update((current) => {
					if (current) {
						return { ...current, account };
					}
					return current;
				});
			}

			return account;
		} catch (error) {
			console.error('Failed to load account:', error);
			return null;
		}
	}

	return {
		identities: { subscribe: identities.subscribe },
		// Primary API - single identity for viewing and signing
		activeIdentity: { subscribe: activeIdentity.subscribe },
		// Backwards compatibility aliases (both point to activeIdentity)
		currentIdentity: { subscribe: activeIdentity.subscribe },
		signingIdentity: { subscribe: activeIdentity.subscribe },
		isAuthenticated: { subscribe: isAuthenticated.subscribe },
		errorMessage: {
			subscribe: errorMessage.subscribe,
			set: errorMessage.set
		},

		// Account management
		registerNewAccount,
		loadAccountByUsername,

		async initialize() {
			const oldSeedPhrase =
				typeof window !== 'undefined' ? localStorage.getItem('seed_phrase') : null;
			if (oldSeedPhrase) {
				persistSeedPhrase(oldSeedPhrase);
				localStorage.removeItem('seed_phrase');
				localStorage.removeItem('identity_key');
			}

			const storedSeedPhrases = getStoredSeedPhrases();
			const validPhrases: string[] = [];

			for (const seedPhrase of storedSeedPhrases) {
				try {
					const identity = identityFromSeed(seedPhrase);
					const keyPair = identity.getKeyPair();
					const publicKeyBytes = new Uint8Array(identity.getPublicKey().rawKey);
					const secretKeyRaw = new Uint8Array(keyPair.secretKey);

					validPhrases.push(seedPhrase);
					addIdentity(identity, publicKeyBytes, secretKeyRaw, seedPhrase);
				} catch (error) {
					console.error('Failed to authenticate with stored seed phrase:', error);
				}
			}

			if (validPhrases.length !== storedSeedPhrases.length) {
				setStoredSeedPhrases(validPhrases);
			}

			// Load account data for all identities and remove those without accounts
			const identitiesList = get(identities);
			const phrasesWithAccounts: string[] = [];

			for (const identityInfo of identitiesList) {
				const account = await loadAccountForIdentity(identityInfo);
				if (account) {
					// Update the identity with account info
					identities.update((prev) =>
						prev.map((id) =>
							id.principal.toString() === identityInfo.principal.toString()
								? { ...id, account }
								: id
						)
					);
					// Update activeIdentity if this is the current one
					const current = get(activeIdentity);
					if (current?.principal.toString() === identityInfo.principal.toString()) {
						activeIdentity.set({ ...identityInfo, account });
					}
					// Keep this seed phrase (only for seedPhrase-type identities)
					if (identityInfo.type === 'seedPhrase' && identityInfo.seedPhrase) {
						phrasesWithAccounts.push(identityInfo.seedPhrase);
					}
				}
			}

			// Remove identities without accounts
			identities.update((prev) => prev.filter((id) => id.account));

			// Remove seed phrases without accounts from storage
			if (phrasesWithAccounts.length !== validPhrases.length) {
				setStoredSeedPhrases(phrasesWithAccounts);
			}

			// If active identity has no account, clear it
			const current = get(activeIdentity);
			if (current && !current.account) {
				activeIdentity.set(null);
			}

			// Try to load OAuth session if no seed phrases found
			if (validPhrases.length === 0 && !get(activeIdentity)) {
				await this.loadOAuthSession();
			}
		},

		async loadOAuthSession(): Promise<boolean> {
			try {
				const response = await fetch(`${API_BASE_URL}/api/v1/oauth/session/keypair`, {
					credentials: 'include'
				});

				if (!response.ok) return false;

				const { success, data } = await response.json();
				if (!success || !data) return false;

				// Convert hex strings to Uint8Array
				const privateKey = hexToBytes(data.private_key);
				const publicKey = hexToBytes(data.public_key);

				// Create Ed25519 identity from private key
				const identity = Ed25519KeyIdentity.fromSecretKey(privateKey);
				const principal = identity.getPrincipal();

				// Create OAuth identity info
				const oauthIdentity: IdentityInfo = {
					identity,
					principal,
					type: 'oauth',
					publicKeyBytes: publicKey,
					secretKeyRaw: privateKey,
					seedPhrase: undefined
				};

				// Load account if linked
				if (data.username) {
					const account = await loadAccountByUsername(data.username);
					if (account) {
						oauthIdentity.account = account;
						// Add to identities and set as active
						identities.update((prev) => [...prev, oauthIdentity]);
						activeIdentity.set(oauthIdentity);
						return true;
					}
				}

				// OAuth session exists but account not found - this shouldn't happen
				// but if it does, just return false without redirect
				return false;
			} catch (error) {
				console.error('Failed to load OAuth session:', error);
				return false;
			}
		},

		async loginWithSeedPhrase(existingSeedPhrase: string, returnUrl = '/dashboard/marketplace') {
			try {
				persistSeedPhrase(existingSeedPhrase);

				const identity = identityFromSeed(existingSeedPhrase);
				const keyPair = identity.getKeyPair();
				const publicKeyBytes = new Uint8Array(identity.getPublicKey().rawKey);
				const secretKeyRaw = new Uint8Array(keyPair.secretKey);

				addIdentity(identity, publicKeyBytes, secretKeyRaw, existingSeedPhrase);

				// Load account data immediately after adding identity
				const account = await loadAccountForIdentity({
					identity,
					principal: identity.getPrincipal(),
					type: 'seedPhrase',
					publicKeyBytes,
					secretKeyRaw,
					seedPhrase: existingSeedPhrase
				});

				if (account) {
					// Update activeIdentity with account data
					activeIdentity.update((current) => {
						if (current && current.seedPhrase === existingSeedPhrase) {
							return { ...current, account };
						}
						return current;
					});
				}
			} catch (error) {
				console.error('Failed to login with seed phrase:', error);
				throw error;
			}
		},

		async logout() {
			// Clear OAuth cookies if this is an OAuth session
			const currentIdentity = get(activeIdentity);
			if (currentIdentity?.type === 'oauth') {
				try {
					await fetch(`${API_BASE_URL}/api/v1/oauth/logout`, {
						method: 'POST',
						credentials: 'include'
					});
				} catch (error) {
					console.error('Failed to clear OAuth session:', error);
				}
			}

			// Clear frontend state
			identities.set([]);
			activeIdentity.set(null);
			errorMessage.set(null);
			clearStoredSeedPhrases();
		},

		switchIdentity(principal: Principal) {
			const identitiesList = get(identities);
			const targetIdentity = identitiesList.find(
				(i) => i.principal.toString() === principal.toString()
			);
			if (!targetIdentity) return;

			activeIdentity.set(targetIdentity);
		},

		signOutIdentity(principal: Principal) {
			identities.update((prev) => {
				const remaining = prev.filter(
					(i) => i.principal.toString() !== principal.toString()
				);

				if (remaining.length === 0) {
					activeIdentity.set(null);
					return remaining;
				}

				const current = get(activeIdentity);
				if (current?.principal.toString() === principal.toString()) {
					activeIdentity.set(remaining[0]);
				}

				return remaining;
			});

			// Only filter seed phrases for seedPhrase-type identities
			const identitiesList = get(identities);
			const identity = identitiesList.find((i) => i.principal.toString() === principal.toString());
			if (identity?.type === 'seedPhrase') {
				filterStoredSeedPhrases((seedPhrase) => {
					const id = identityFromSeed(seedPhrase);
					return id.getPrincipal().toString() !== principal.toString();
				});
			}
		},

		backupSeedPhrase(principal: Principal): string | null {
			const identitiesList = get(identities);
			const identity = identitiesList.find(
				(i) => i.principal.toString() === principal.toString()
			);

			if (!identity) {
				errorMessage.set('Identity not found');
				return null;
			}

			if (identity.type !== 'seedPhrase' || !identity.seedPhrase) {
				errorMessage.set('This identity does not have a seed phrase (OAuth login)');
				return null;
			}

			return identity.seedPhrase;
		},

		async getAuthenticatedIdentity(): Promise<AuthenticatedIdentityResult | null> {
			const current = get(activeIdentity);
			if (!current) {
				return null;
			}

			return {
				success: true,
				identity: current.identity,
				publicKeyBytes: current.publicKeyBytes,
				secretKeyRaw: current.secretKeyRaw
			};
		},

		// Alias for backwards compatibility - same as getAuthenticatedIdentity
		async getSigningIdentity(): Promise<AuthenticatedIdentityResult | null> {
			return this.getAuthenticatedIdentity();
		},

		async updateDisplayName() {
			const current = get(activeIdentity);
			if (!current) {
				return;
			}

			const displayName = await fetchUserProfile(current.publicKeyBytes);
			if (displayName) {
				activeIdentity.update((identity) => {
					if (identity) {
						return { ...identity, displayName };
					}
					return identity;
				});
			}
		}
	};
}

export const authStore = createAuthStore();
