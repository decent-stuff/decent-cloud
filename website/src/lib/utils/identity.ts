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
