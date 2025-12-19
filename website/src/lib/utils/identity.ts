import { Ed25519KeyIdentity } from '@dfinity/identity';
import { mnemonicToSeedSync, validateMnemonic } from 'bip39';
import { hmac } from '@noble/hashes/hmac';
import { sha512 } from '@noble/hashes/sha512';

/**
 * Derives an Ed25519 identity from a BIP39 seed phrase.
 * Uses HMAC-SHA512 with 'ed25519 seed' as the key to derive a 32-byte seed.
 * @throws {Error} If the seed phrase is invalid
 */
export function identityFromSeed(seedPhrase: string): Ed25519KeyIdentity {
	if (!validateMnemonic(seedPhrase)) {
		throw new Error('Invalid seed phrase');
	}
	const seedBuffer = mnemonicToSeedSync(seedPhrase, '');
	const seedBytes = new Uint8Array(seedBuffer);
	const keyMaterial = hmac(sha512, 'ed25519 seed', seedBytes);
	const derivedSeed = keyMaterial.slice(0, 32);
	return Ed25519KeyIdentity.fromSecretKey(derivedSeed);
}

/**
 * Converts a Uint8Array to a hexadecimal string.
 */
export function bytesToHex(bytes: Uint8Array): string {
	return Array.from(bytes)
		.map((b) => b.toString(16).padStart(2, '0'))
		.join('');
}

/**
 * Converts a hexadecimal string to a Uint8Array.
 * @throws {Error} If the hex string is invalid
 */
export function hexToBytes(hex: string): Uint8Array {
	if (hex.length % 2 !== 0) {
		throw new Error('Invalid hex string: length must be even');
	}
	const bytes = new Uint8Array(hex.length / 2);
	for (let i = 0; i < hex.length; i += 2) {
		bytes[i / 2] = parseInt(hex.substring(i, i + 2), 16);
	}
	return bytes;
}

/**
 * Normalizes a public key to a hex string.
 * Handles both hex strings and byte arrays (from backend responses).
 */
export function normalizePubkey(pubkey: string | number[]): string {
	if (typeof pubkey === 'string') {
		return pubkey;
	}
	return bytesToHex(new Uint8Array(pubkey));
}

/**
 * Truncates a public key hex string for display.
 * Shows first 6 and last 6 characters with ellipsis.
 * Consistent format across the entire UI.
 */
export function truncatePubkey(pubkey: string, visible: number = 6): string {
	if (!pubkey) return '';
	if (pubkey.length <= visible * 2) {
		return pubkey;
	}
	return `${pubkey.slice(0, visible)}...${pubkey.slice(-visible)}`;
}

/**
 * Checks if a string is a valid Ed25519 public key hex string.
 * Ed25519 keys are 32 bytes = 64 hex characters.
 */
export function isPubkeyHex(identifier: string): boolean {
	return /^[0-9a-fA-F]{64}$/.test(identifier);
}

/**
 * Resolves an identifier (pubkey or username) to a pubkey.
 * If it's already a pubkey hex, returns it directly.
 * If it's a username, looks up the account and returns the first active public key.
 * Returns null if not found.
 */
export async function resolveIdentifierToPubkey(identifier: string): Promise<string | null> {
	if (isPubkeyHex(identifier)) {
		return identifier;
	}
	// It's a username - look up the account
	const { getAccount } = await import('$lib/services/account-api');
	const account = await getAccount(identifier);
	if (!account || !account.publicKeys || account.publicKeys.length === 0) {
		return null;
	}
	// Return the first active public key
	const activeKey = account.publicKeys.find((k) => k.isActive);
	return activeKey?.publicKey ?? account.publicKeys[0].publicKey;
}

/**
 * Get the display name for an identifier - username if available, truncated pubkey otherwise.
 */
export async function getDisplayNameForPubkey(pubkey: string): Promise<string> {
	const { getAccountByPublicKey } = await import('$lib/services/account-api');
	const account = await getAccountByPublicKey(pubkey);
	return account?.username ?? truncatePubkey(pubkey);
}
