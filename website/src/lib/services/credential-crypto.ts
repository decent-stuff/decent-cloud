/**
 * Credential decryption utilities
 *
 * Decrypts VM credentials that were encrypted with the user's Ed25519 public key.
 * Uses Ed25519→X25519 key conversion and XChaCha20Poly1305 authenticated encryption.
 */

import { x25519 } from '@noble/curves/ed25519';
import { xchacha20poly1305 } from '@noble/ciphers/chacha.js';
import { sha512 } from '@noble/hashes/sha512';
import type { EncryptedCredentials } from './api';

/**
 * Convert Ed25519 secret key to X25519 secret key
 *
 * Ed25519 uses SHA-512 hash of seed for scalar derivation with clamping.
 * This matches the Rust implementation.
 */
function ed25519SecretToX25519(ed25519Secret: Uint8Array): Uint8Array {
	// Ed25519 seed is first 32 bytes
	const seed = ed25519Secret.slice(0, 32);

	// Hash with SHA-512
	const hash = sha512(seed);

	// Take first 32 bytes and apply clamping
	const scalar = hash.slice(0, 32);
	scalar[0] &= 248;
	scalar[31] &= 127;
	scalar[31] |= 64;

	return scalar;
}

/**
 * Decrypt credentials using the user's Ed25519 private key
 *
 * @param encryptedJson - JSON string containing encrypted credentials
 * @param ed25519SecretKey - User's Ed25519 secret key (32 or 64 bytes)
 * @returns Decrypted credentials string
 */
export async function decryptCredentials(
	encryptedJson: string,
	ed25519SecretKey: Uint8Array
): Promise<string> {
	// Parse encrypted credentials
	const encrypted: EncryptedCredentials = JSON.parse(encryptedJson);

	if (encrypted.version !== 1) {
		throw new Error(`Unsupported encryption version: ${encrypted.version}`);
	}

	// Decode base64 fields
	const ephemeralPubkey = base64ToBytes(encrypted.ephemeral_pubkey);
	const nonce = base64ToBytes(encrypted.nonce);
	const ciphertext = base64ToBytes(encrypted.ciphertext);

	// Convert Ed25519 secret to X25519
	const x25519Secret = ed25519SecretToX25519(ed25519SecretKey);

	// Perform X25519 key exchange
	const sharedSecret = x25519.getSharedSecret(x25519Secret, ephemeralPubkey);

	// Derive decryption key using SHA-512 (same as Rust implementation)
	const keyMaterial = sha512(
		new Uint8Array([
			...new TextEncoder().encode('credential-encryption-v1'),
			...sharedSecret
		])
	);
	const decryptionKey = keyMaterial.slice(0, 32);

	// Decrypt with XChaCha20Poly1305
	const cipher = xchacha20poly1305(decryptionKey, nonce);
	const plaintext = cipher.decrypt(ciphertext);

	return new TextDecoder().decode(plaintext);
}

/**
 * Convert base64 string to Uint8Array
 */
function base64ToBytes(base64: string): Uint8Array {
	const binary = atob(base64);
	const bytes = new Uint8Array(binary.length);
	for (let i = 0; i < binary.length; i++) {
		bytes[i] = binary.charCodeAt(i);
	}
	return bytes;
}
