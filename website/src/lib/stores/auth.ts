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
import { hmac } from '@noble/hashes/hmac';
import { sha512 } from '@noble/hashes/sha512';
import { generateMnemonic, mnemonicToSeedSync, validateMnemonic } from 'bip39';
import {
	addSeedPhrase as persistSeedPhrase,
	clearStoredSeedPhrases,
	filterStoredSeedPhrases,
	getStoredSeedPhrases,
	setStoredSeedPhrases
} from '../utils/seed-storage';
import { API_BASE_URL } from '../services/api';

/**
 * Account information from backend (1 account → 1-10 keys)
 */
export interface AccountInfo {
	id: string; // Account ID (hex)
	username: string; // Human-readable username
	createdAt: number;
	updatedAt: number;
	publicKeys: Array<{
		id: string;
		publicKey: string;
		isActive: boolean;
		addedAt: number;
		deviceName?: string; // Stored in backend database
		disabledAt?: number;
		disabledByKeyId?: string;
	}>;
}

/**
 * Local identity (seed phrase) that generates one keypair for one account
 * A user can have multiple IdentityInfo objects (multiple seed phrases) to access multiple accounts
 */
export interface IdentityInfo {
	identity: Ed25519KeyIdentity;
	principal: Principal;
	type: 'seedPhrase';
	name?: string;
	displayName?: string;
	publicKeyBytes: Uint8Array;
	secretKeyRaw: Uint8Array;
	seedPhrase: string;
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
	const showSeedPhrase = writable(false);
	const showBackupInstructions = writable(false);
	const errorMessage = writable<string | null>(null);

	const isAuthenticated = derived(activeIdentity, ($active) => $active !== null);

	// Generate Ed25519 identity from seed phrase
	function identityFromSeed(seedPhrase: string): Ed25519KeyIdentity {
		if (!validateMnemonic(seedPhrase)) {
			throw new Error('Invalid seed phrase');
		}
		const seedBuffer = mnemonicToSeedSync(seedPhrase, '');
		const seedBytes = new Uint8Array(seedBuffer);
		const keyMaterial = hmac(sha512, 'ed25519 seed', seedBytes);
		const derivedSeed = keyMaterial.slice(0, 32);
		return Ed25519KeyIdentity.fromSecretKey(derivedSeed);
	}

	function generateNewSeedPhrase(): string {
		return generateMnemonic();
	}

	function addIdentity(
		identity: Ed25519KeyIdentity,
		publicKeyBytes: Uint8Array,
		secretKeyRaw: Uint8Array,
		seedPhrase: string
	) {
		const principal = identity.getPrincipal();

		// Check if this seed phrase is already stored
		const identitiesList = get(identities);
		const hasExactPhrase = identitiesList.some((i) => i.seedPhrase === seedPhrase);
		if (hasExactPhrase) return;

		const newIdentity: IdentityInfo = {
			identity,
			principal,
			type: 'seedPhrase',
			publicKeyBytes,
			secretKeyRaw,
			seedPhrase
		};

		identities.update((prev) => [...prev, newIdentity]);

		const current = get(activeIdentity);
		if (!current) {
			activeIdentity.set(newIdentity);
		}
	}

	async function fetchUserProfile(publicKeyBytes: Uint8Array): Promise<string | null> {
		try {
			const pubkey = Array.from(publicKeyBytes)
				.map((b) => b.toString(16).padStart(2, '0'))
				.join('');
			const res = await fetch(`${API_BASE_URL}/api/v1/users/${pubkey}/profile`);
			if (res.ok) {
				const data = await res.json();
				if (data.success && data.data?.display_name) {
					return data.data.display_name;
				}
			}
		} catch (error) {
			console.error('Failed to fetch user profile:', error);
		}
		return null;
	}

	async function loadAccountForIdentity(identityInfo: IdentityInfo): Promise<void> {
		if (!identityInfo.publicKeyBytes) return;

		try {
			const { getAccount } = await import('../services/account-api');
			const publicKeyHex = Array.from(identityInfo.publicKeyBytes)
				.map((b) => b.toString(16).padStart(2, '0'))
				.join('');

			// Try to find account by searching - for now we'll need username
			// TODO: Add API endpoint to search by public key
			// For now, accounts will be loaded after registration or manual username entry
		} catch (error) {
			console.error('Failed to load account:', error);
		}
	}

	async function registerNewAccount(
		identity: Ed25519KeyIdentity,
		username: string
	): Promise<AccountInfo> {
		const { registerAccount } = await import('../services/account-api');
		const account = await registerAccount(identity, username);

		// Update active identity with account info
		activeIdentity.update((current) => {
			if (current) {
				return { ...current, account };
			}
			return current;
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
		showSeedPhrase: {
			subscribe: showSeedPhrase.subscribe,
			set: showSeedPhrase.set
		},
		showBackupInstructions: {
			subscribe: showBackupInstructions.subscribe,
			set: showBackupInstructions.set
		},
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
		},

		async loginWithSeedPhrase(existingSeedPhrase?: string, returnUrl = '/dashboard') {
			try {
				const seedPhrase = existingSeedPhrase || generateNewSeedPhrase();
				persistSeedPhrase(seedPhrase);
				showSeedPhrase.set(true);

				const identity = identityFromSeed(seedPhrase);
				const keyPair = identity.getKeyPair();
				addIdentity(
					identity,
					new Uint8Array(identity.getPublicKey().rawKey),
					new Uint8Array(keyPair.secretKey),
					seedPhrase
				);

				if (!existingSeedPhrase) {
					showBackupInstructions.set(true);
					showSeedPhrase.set(true);
				}

				if (typeof window !== 'undefined') {
					window.location.href = returnUrl;
				}
			} catch (error) {
				console.error('Failed to login with seed phrase:', error);
				throw error;
			}
		},

		async logout() {
			identities.set([]);
			activeIdentity.set(null);
			showSeedPhrase.set(false);
			showBackupInstructions.set(false);
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

			filterStoredSeedPhrases((seedPhrase) => {
				const identity = identityFromSeed(seedPhrase);
				return identity.getPrincipal().toString() !== principal.toString();
			});
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
