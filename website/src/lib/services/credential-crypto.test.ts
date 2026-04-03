import { describe, it, expect } from 'vitest';
import { decryptCredentials } from './credential-crypto';
import { x25519 } from '@noble/curves/ed25519';
import { xchacha20poly1305 } from '@noble/ciphers/chacha.js';
import { sha512 } from '@noble/hashes/sha512';
import { randomBytes } from '@noble/ciphers/utils.js';

function base64Encode(bytes: Uint8Array): string {
	let binary = '';
	for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i]);
	return btoa(binary);
}

/** Clamp an X25519 scalar (same as Rust server and frontend ed25519SecretToX25519). */
function clamp(scalar: Uint8Array): Uint8Array {
	const s = new Uint8Array(scalar);
	s[0] &= 248;
	s[31] &= 127;
	s[31] |= 64;
	return s;
}

/** Derive X25519 secret from an Ed25519 seed (32-byte secret), matching the frontend. */
function secretToX25519(seed: Uint8Array): Uint8Array {
	const hash = sha512(seed.slice(0, 32));
	return clamp(hash.slice(0, 32));
}

/**
 * Encrypt credentials to produce a JSON string matching the Rust server format.
 * `recipientSeed` is the 32-byte Ed25519 seed of the intended recipient.
 */
function makeEncryptedJson(
	credentials: string,
	recipientSeed: Uint8Array,
	version: 1 | 2,
	aadBytes?: Uint8Array
): string {
	const recipientX25519Secret = secretToX25519(recipientSeed);
	const recipientX25519Pubkey = x25519.getPublicKey(recipientX25519Secret);

	const ephemeralSecret = clamp(randomBytes(32));
	const ephemeralPubkey = x25519.getPublicKey(ephemeralSecret);
	const sharedSecret = x25519.getSharedSecret(ephemeralSecret, recipientX25519Pubkey);

	const domain =
		version === 2 ? 'credential-encryption-v2-aad' : 'credential-encryption-v1';
	const keyMaterial = sha512(
		new Uint8Array([...new TextEncoder().encode(domain), ...sharedSecret])
	);
	const encryptionKey = keyMaterial.slice(0, 32);

	const nonce = randomBytes(24);
	const cipher = xchacha20poly1305(encryptionKey, nonce, version === 2 ? aadBytes : undefined);
	const ciphertext = cipher.encrypt(new TextEncoder().encode(credentials));

	const obj: Record<string, unknown> = {
		version,
		ephemeral_pubkey: base64Encode(ephemeralPubkey),
		nonce: base64Encode(nonce),
		ciphertext: base64Encode(ciphertext)
	};
	if (version === 2 && aadBytes) obj.aad = base64Encode(aadBytes);
	return JSON.stringify(obj);
}

describe('decryptCredentials', () => {
	it('decrypts version 1 credentials', async () => {
		const seed = x25519.utils.randomSecretKey();
		const encrypted = makeEncryptedJson('hunter2', seed, 1);
		expect(await decryptCredentials(encrypted, seed)).toBe('hunter2');
	});

	it('decrypts version 2 credentials with AAD', async () => {
		const seed = x25519.utils.randomSecretKey();
		const contractId = new Uint8Array(32).fill(0xab);
		const encrypted = makeEncryptedJson('SecurePass!', seed, 2, contractId);
		expect(await decryptCredentials(encrypted, seed)).toBe('SecurePass!');
	});

	it('decrypts version 2 credentials without aad field', async () => {
		const seed = x25519.utils.randomSecretKey();
		const encrypted = makeEncryptedJson('NoAadPass', seed, 2, undefined);
		expect(await decryptCredentials(encrypted, seed)).toBe('NoAadPass');
	});

	it('throws for unsupported version', async () => {
		const seed = x25519.utils.randomSecretKey();
		const fake = JSON.stringify({ version: 3, ephemeral_pubkey: 'AA==', nonce: 'AA==', ciphertext: 'AA==' });
		await expect(decryptCredentials(fake, seed)).rejects.toThrow('Unsupported encryption version: 3');
	});

	it('fails decryption with wrong secret key', async () => {
		const seed = x25519.utils.randomSecretKey();
		const wrongSeed = x25519.utils.randomSecretKey();
		const encrypted = makeEncryptedJson('secret', seed, 2, new Uint8Array(8));
		await expect(decryptCredentials(encrypted, wrongSeed)).rejects.toThrow();
	});

	it('fails version 2 decryption when stored AAD is tampered', async () => {
		const seed = x25519.utils.randomSecretKey();
		const correctAad = new Uint8Array(32).fill(0x01);
		const encrypted = makeEncryptedJson('secret', seed, 2, correctAad);
		const parsed = JSON.parse(encrypted);
		parsed.aad = base64Encode(new Uint8Array(32).fill(0x02));
		await expect(decryptCredentials(JSON.stringify(parsed), seed)).rejects.toThrow();
	});
});
