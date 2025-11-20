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
import { AuthClient } from '@dfinity/auth-client';
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
		deviceName?: string; // Stored locally only
	}>;
}

/**
 * Local identity (seed phrase) that generates one keypair for one account
 * A user can have multiple IdentityInfo objects (multiple seed phrases) to access multiple accounts
 */
export interface IdentityInfo {
	identity: Identity;
	principal: Principal;
	type: 'ii' | 'seedPhrase';
	name?: string;
	displayName?: string;
	publicKeyBytes?: Uint8Array;
	secretKeyRaw?: Uint8Array;
	seedPhrase?: string;
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
	const currentIdentity = writable<IdentityInfo | null>(null);
	const signingIdentity = writable<IdentityInfo | null>(null);
	const authClient = writable<AuthClient | null>(null);
	const showSeedPhrase = writable(false);
	const showBackupInstructions = writable(false);
	const errorMessage = writable<string | null>(null);

	const isAuthenticated = derived(currentIdentity, ($current) => $current !== null);

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
		identity: Identity,
		type: IdentityInfo['type'],
		publicKeyBytes?: Uint8Array,
		secretKeyRaw?: Uint8Array,
		seedPhrase?: string
	) {
		const principal = identity.getPrincipal();
		const newIdentity: IdentityInfo = {
			identity,
			principal,
			type,
			publicKeyBytes,
			secretKeyRaw,
			seedPhrase
		};

		identities.update((prev) => {
			if (type === 'seedPhrase' && seedPhrase) {
				const hasExactPhrase = prev.some((i) => i.seedPhrase === seedPhrase);
				if (hasExactPhrase) return prev;
			} else {
				const existing = prev.find((i) => i.principal.toString() === principal.toString());
				if (existing) {
					if (
						existing.type !== type ||
						!existing.publicKeyBytes ||
						!existing.secretKeyRaw
					) {
						return prev.map((i) =>
							i.principal.toString() === principal.toString()
								? { ...i, type, publicKeyBytes, secretKeyRaw }
								: i
						);
					}
					return prev;
				}
			}
			return [...prev, newIdentity];
		});

		const current = get(currentIdentity);
		if (!current) {
			currentIdentity.set(newIdentity);
		}

		if (type !== 'seedPhrase' && !get(signingIdentity)) {
			const identitiesList = get(identities);
			const hasSeedIdentity = identitiesList.some((i) => i.type === 'seedPhrase');
			const hasStoredSeedPhrases = getStoredSeedPhrases().length > 0;

			if (!hasSeedIdentity && !hasStoredSeedPhrases) {
				try {
					const newSeedPhrase = generateNewSeedPhrase();
					const newIdentity = identityFromSeed(newSeedPhrase);
					const keyPair = newIdentity.getKeyPair();
					const pubBytes = new Uint8Array(newIdentity.getPublicKey().rawKey);
					const secBytes = new Uint8Array(keyPair.secretKey);

					const seedIdentity = {
						identity: newIdentity,
						principal: newIdentity.getPrincipal(),
						type: 'seedPhrase' as const,
						publicKeyBytes: pubBytes,
						secretKeyRaw: secBytes,
						seedPhrase: newSeedPhrase
					};

					persistSeedPhrase(newSeedPhrase);
					identities.update((prev) => [...prev, seedIdentity]);
					signingIdentity.set(seedIdentity);
					showSeedPhrase.set(true);
					showBackupInstructions.set(true);
				} catch (error) {
					console.error('Failed to create seed identity:', error);
					errorMessage.set(
						'A seed-based identity is required for signing updates. Failed to create one automatically.'
					);
				}
			}
		} else if (type === 'seedPhrase') {
			signingIdentity.set(newIdentity);
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

		// Update current identity with account info
		currentIdentity.update((current) => {
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
				// Update current identity with account info
				currentIdentity.update((current) => {
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
		currentIdentity: { subscribe: currentIdentity.subscribe },
		signingIdentity: { subscribe: signingIdentity.subscribe },
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
			let foundSigningIdentity = false;

			for (const seedPhrase of storedSeedPhrases) {
				try {
					const identity = identityFromSeed(seedPhrase);
					const keyPair = identity.getKeyPair();
					const publicKeyBytes = new Uint8Array(identity.getPublicKey().rawKey);
					const secretKeyRaw = new Uint8Array(keyPair.secretKey);

					validPhrases.push(seedPhrase);
					addIdentity(identity, 'seedPhrase', publicKeyBytes, secretKeyRaw, seedPhrase);

					if (!foundSigningIdentity) {
						signingIdentity.set({
							identity,
							principal: identity.getPrincipal(),
							type: 'seedPhrase',
							publicKeyBytes,
							secretKeyRaw,
							seedPhrase
						});
						foundSigningIdentity = true;
					}
				} catch (error) {
					console.error('Failed to authenticate with stored seed phrase:', error);
				}
			}

			if (validPhrases.length !== storedSeedPhrases.length) {
				setStoredSeedPhrases(validPhrases);
			}

			try {
				const client = await AuthClient.create();
				authClient.set(client);
				const isAuthenticated = await client.isAuthenticated();
				if (isAuthenticated) {
					const identity = client.getIdentity();
					addIdentity(identity, 'ii');
				}
			} catch (error) {
				console.error('Failed to initialize AuthClient:', error);
			}
		},

		async loginWithII(returnUrl = '/dashboard') {
			const client = get(authClient);
			if (!client) return;

			const days = 1;
			const maxTimeToLive = BigInt(days) * BigInt(24) * BigInt(3600000000000);

			await client.login({
				maxTimeToLive,
				identityProvider: 'https://identity.ic0.app',
				onSuccess: async () => {
					const identity = client.getIdentity();
					addIdentity(identity, 'ii');
					if (typeof window !== 'undefined') {
						window.location.href = returnUrl;
					}
				}
			});
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
					'seedPhrase',
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
			const client = get(authClient);
			if (client) {
				await client.logout();
			}

			identities.set([]);
			currentIdentity.set(null);
			signingIdentity.set(null);
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

			currentIdentity.set(targetIdentity);

			if (targetIdentity.type === 'seedPhrase') {
				signingIdentity.set(targetIdentity);
			} else {
				const currentSigning = get(signingIdentity);
				if (!currentSigning || currentSigning.type !== 'seedPhrase') {
					const seedIdentity = identitiesList.find((i) => i.type === 'seedPhrase');
					if (seedIdentity) {
						signingIdentity.set(seedIdentity);
					}
				}
			}

			if (targetIdentity.type === 'ii') {
				AuthClient.create().then((client) => authClient.set(client));
			}
		},

		signOutIdentity(principal: Principal) {
			identities.update((prev) => {
				const remaining = prev.filter(
					(i) => i.principal.toString() !== principal.toString()
				);

				if (remaining.length === 0) {
					currentIdentity.set(null);
					signingIdentity.set(null);
					return remaining;
				}

				const nextSeedIdentity = remaining.find((i) => i.type === 'seedPhrase');

				const current = get(currentIdentity);
				if (current?.principal.toString() === principal.toString()) {
					currentIdentity.set(nextSeedIdentity || remaining[0]);
				}

				const signing = get(signingIdentity);
				if (!signing || signing.principal.toString() === principal.toString()) {
					signingIdentity.set(nextSeedIdentity || null);
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

			if (identity.type !== 'seedPhrase') {
				errorMessage.set('This identity does not use a seed phrase');
				return null;
			}

			if (identity.seedPhrase) {
				return identity.seedPhrase;
			}

			const storedPhrases = getStoredSeedPhrases();
			const matchingPhrase = storedPhrases.find((phrase: string) => {
				try {
					const testIdentity = identityFromSeed(phrase);
					return testIdentity.getPrincipal().toString() === principal.toString();
				} catch {
					return false;
				}
			});

			if (matchingPhrase) {
				return matchingPhrase;
			}

			errorMessage.set('Seed phrase not found - possible data loss');
			return null;
		},

		async getAuthenticatedIdentity(): Promise<AuthenticatedIdentityResult | null> {
			const current = get(currentIdentity);
			if (
				!current ||
				current.type !== 'seedPhrase' ||
				!current.publicKeyBytes ||
				!current.secretKeyRaw
			) {
				return null;
			}

			return {
				success: true,
				identity: current.identity,
				publicKeyBytes: current.publicKeyBytes,
				secretKeyRaw: current.secretKeyRaw
			};
		},

		async getSigningIdentity(): Promise<AuthenticatedIdentityResult | null> {
			const signing = get(signingIdentity);
			if (!signing || !signing.publicKeyBytes || !signing.secretKeyRaw) {
				return null;
			}

			return {
				success: true,
				identity: signing.identity,
				publicKeyBytes: signing.publicKeyBytes,
				secretKeyRaw: signing.secretKeyRaw
			};
		},

		async updateDisplayName() {
			const current = get(currentIdentity);
			if (!current || !current.publicKeyBytes) {
				return;
			}

			const displayName = await fetchUserProfile(current.publicKeyBytes);
			if (displayName) {
				currentIdentity.update((identity) => {
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
