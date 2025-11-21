import { describe, it, expect, vi, beforeAll } from 'vitest';
import { signRequest } from './auth-api';
import { Ed25519KeyIdentity } from '@dfinity/identity';
import { sha512 } from '@noble/hashes/sha512';
import * as ed from '@noble/ed25519';
import { ed25519ph } from '@noble/curves/ed25519';

// Setup hash for ed25519 v3 before all tests
beforeAll(() => {
	ed.hashes.sha512 = sha512;
	ed.hashes.sha512Async = (m: Uint8Array) => Promise.resolve(sha512(m));
});

function bytesToHex(bytes: Uint8Array): string {
	return Array.from(bytes)
		.map((b) => b.toString(16).padStart(2, '0'))
		.join('');
}

const ED25519_SIGN_CONTEXT = new TextEncoder().encode('decent-cloud');

/**
 * Cross-platform signature test.
 * These values MUST match the Rust test in api/src/auth/tests.rs
 */
describe('cross-platform signature compatibility', () => {
	it('produces signature matching Rust backend', () => {
		// Same seed as Rust test: [0, 1, 2, ..., 31]
		const seed = new Uint8Array(32);
		for (let i = 0; i < 32; i++) {
			seed[i] = i;
		}

		// Create identity
		const identity = Ed25519KeyIdentity.fromSecretKey(seed);
		const publicKeyBytes = new Uint8Array(identity.getPublicKey().rawKey);
		const secretKeyBytes = new Uint8Array(identity.getKeyPair().secretKey);

		// Expected public key from Rust
		const expectedPubkey = '03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8';
		expect(bytesToHex(publicKeyBytes)).toBe(expectedPubkey);

		// Same message as Rust test
		const message = new TextEncoder().encode('test message for cross-platform verification');

		// Sign with ed25519ph (prehashed) + context
		const signature = ed25519ph.sign(message, secretKeyBytes.slice(0, 32), {
			context: ED25519_SIGN_CONTEXT
		});

		// Expected signature from Rust
		const expectedSignature =
			'a2aca8ef6760241fc2b254447b9320f03fffaaa11f60365b33455b5d664abc0172627ce2258cdbde7e2eddbe20bda46e008f8041ffb61515e7f4e5a8fdab3f0f';

		expect(bytesToHex(signature)).toBe(expectedSignature);
	});
});

describe('signRequest', () => {
	it('creates signed request with correct headers and body', async () => {
		// Create a test identity
		const seed = new Uint8Array(32);
		for (let i = 0; i < 32; i++) {
			seed[i] = i;
		}
		const identity = Ed25519KeyIdentity.fromSecretKey(seed);

		const method = 'PUT';
		const path = '/api/v1/users/test-pubkey/profile';
		const bodyData = { display_name: 'Test User' };

		const result = await signRequest(identity, method, path, bodyData);

		// Verify structure
		expect(result).toHaveProperty('headers');
		expect(result).toHaveProperty('body');

		// Verify headers exist
		expect(result.headers).toHaveProperty('X-Public-Key');
		expect(result.headers).toHaveProperty('X-Signature');
		expect(result.headers).toHaveProperty('X-Timestamp');
		expect(result.headers['Content-Type']).toBe('application/json');

		// Verify body is serialized JSON
		expect(result.body).toBe(JSON.stringify(bodyData));

		// Verify public key is hex string (64 chars for 32 bytes)
		expect(result.headers['X-Public-Key']).toMatch(/^[0-9a-f]{64}$/);

		// Verify signature is hex string (128 chars for 64 bytes)
		expect(result.headers['X-Signature']).toMatch(/^[0-9a-f]{128}$/);

		// Verify timestamp is nanoseconds (numeric string)
		expect(result.headers['X-Timestamp']).toMatch(/^\d+$/);
		const timestamp = parseInt(result.headers['X-Timestamp'], 10);
		expect(timestamp).toBeGreaterThan(Date.now() * 1_000_000 - 1000000000); // within last second
	});

	it('handles requests without body data', async () => {
		const seed = new Uint8Array(32).fill(1);
		const identity = Ed25519KeyIdentity.fromSecretKey(seed);

		const method = 'DELETE';
		const path = '/api/v1/users/test-pubkey/contacts/email';

		const result = await signRequest(identity, method, path);

		expect(result.body).toBe('');
		expect(result.headers['X-Public-Key']).toMatch(/^[0-9a-f]{64}$/);
		expect(result.headers['X-Signature']).toMatch(/^[0-9a-f]{128}$/);
	});

	it('produces different signatures for different messages', async () => {
		const seed = new Uint8Array(32).fill(2);
		const identity = Ed25519KeyIdentity.fromSecretKey(seed);

		const result1 = await signRequest(identity, 'PUT', '/api/v1/test', { data: 'test1' });

		// Wait a tiny bit to ensure different timestamp
		await new Promise((resolve) => setTimeout(resolve, 1));

		const result2 = await signRequest(identity, 'PUT', '/api/v1/test', { data: 'test2' });

		// Different bodies should produce different signatures
		expect(result1.headers['X-Signature']).not.toBe(result2.headers['X-Signature']);
	});

	it('encodes public key correctly', async () => {
		const seed = new Uint8Array(32).fill(3);
		const identity = Ed25519KeyIdentity.fromSecretKey(seed);

		const result = await signRequest(identity, 'GET', '/test');

		const publicKeyBytes = new Uint8Array(identity.getPublicKey().rawKey);
		const expectedHex = Array.from(publicKeyBytes)
			.map((b) => b.toString(16).padStart(2, '0'))
			.join('');

		expect(result.headers['X-Public-Key']).toBe(expectedHex);
	});
});
