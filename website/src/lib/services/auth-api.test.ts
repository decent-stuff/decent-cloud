import { describe, it, expect, vi, beforeAll } from 'vitest';
import { signRequest } from './auth-api';
import { Ed25519KeyIdentity } from '@dfinity/identity';
import { sha512 } from '@noble/hashes/sha512';
import * as ed from '@noble/ed25519';

// Setup hash for ed25519 v3 before all tests
beforeAll(() => {
	ed.hashes.sha512 = sha512;
	ed.hashes.sha512Async = (m: Uint8Array) => Promise.resolve(sha512(m));
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
