import { test, expect, APIRequestContext } from '@playwright/test';
import { registerNewAccount, setupConsoleLogging } from './fixtures/auth-helpers';
import { getTestAccount } from './fixtures/test-account';
import { ed25519ph } from '@noble/curves/ed25519';
import { mnemonicToSeedSync } from 'bip39';
import { sha512 } from '@noble/hashes/sha512';

const API_BASE_URL = process.env.VITE_DECENT_CLOUD_API_URL || 'http://localhost:59011';
const ED25519_SIGN_CONTEXT = new TextEncoder().encode('decent-cloud');

function bytesToHex(bytes: Uint8Array): string {
	return Array.from(bytes)
		.map((b) => b.toString(16).padStart(2, '0'))
		.join('');
}

function seedPhraseToKeyPair(seedPhrase: string): { publicKey: Uint8Array; privateKey: Uint8Array } {
	const seed = mnemonicToSeedSync(seedPhrase).slice(0, 32);
	// Use SHA-512 for Ed25519 key derivation (matches @dfinity/identity)
	const hash = sha512(seed);
	const privateKey = hash.slice(0, 32);

	// Get public key from private key
	// For Ed25519ph, we need to use the ed25519ph module from noble
	const publicKey = ed25519ph.getPublicKey(privateKey);

	return { publicKey, privateKey };
}

async function signRequest(
	privateKey: Uint8Array,
	publicKey: Uint8Array,
	method: string,
	path: string,
	body?: string
): Promise<Record<string, string>> {
	const nonce = crypto.randomUUID();
	const timestampNs = (BigInt(Date.now()) * BigInt(1_000_000)).toString();
	const bodyStr = body || '';
	const pathWithoutQuery = path.split('?')[0];

	const message = new TextEncoder().encode(timestampNs + nonce + method + pathWithoutQuery + bodyStr);
	const signature = ed25519ph.sign(message, privateKey, { context: ED25519_SIGN_CONTEXT });

	return {
		'X-Public-Key': bytesToHex(publicKey),
		'X-Signature': bytesToHex(signature),
		'X-Timestamp': timestampNs,
		'X-Nonce': nonce,
		'Content-Type': 'application/json'
	};
}

test.describe('Chatwoot API', () => {
	test('GET /chatwoot/identity returns 401 for unauthenticated request', async ({ request }) => {
		const response = await request.get(`${API_BASE_URL}/api/v1/chatwoot/identity`);

		// Should return 401 Unauthorized for unauthenticated request
		expect(response.status()).toBe(401);
	});

	test('GET /chatwoot/identity returns identity hash for authenticated user', async ({ page }) => {
		setupConsoleLogging(page);

		// Register a new account to get credentials
		const credentials = await registerNewAccount(page);

		// Generate keypair from seed phrase
		const { publicKey, privateKey } = seedPhraseToKeyPair(credentials.seedPhrase);

		// Create signed request
		const path = '/api/v1/chatwoot/identity';
		const headers = await signRequest(privateKey, publicKey, 'GET', path);

		// Make API request with signed headers
		const response = await page.request.get(`${API_BASE_URL}${path}`, {
			headers
		});

		// Chatwoot may not be configured in test environment, so we accept either:
		// - 200 with identity data (if CHATWOOT_HMAC_SECRET is set)
		// - 200 with success: false and error message (if not configured)
		expect(response.status()).toBe(200);

		const data = await response.json();

		// Should have a success field
		expect(data).toHaveProperty('success');

		if (data.success) {
			// If Chatwoot is configured, verify response structure
			expect(data.data).toHaveProperty('identifier');
			expect(data.data).toHaveProperty('identifierHash');
			expect(data.data.identifier).toBe(bytesToHex(publicKey));
			// identifierHash should be 64 chars (SHA256 hex)
			expect(data.data.identifierHash).toHaveLength(64);
		} else {
			// If not configured, should have error message
			expect(data.error).toContain('Chatwoot not configured');
		}
	});
});

test.describe('Chatwoot Webhook', () => {
	test('POST /webhooks/chatwoot accepts valid message_created event', async ({ request }) => {
		// The webhook should accept events even without database entries
		const payload = {
			event: 'message_created',
			conversation: {
				id: 12345,
				custom_attributes: {
					contract_id: 'test_contract_abc123'
				}
			},
			message: {
				id: 98765,
				message_type: 'incoming',
				created_at: Date.now()
			}
		};

		const response = await request.post(`${API_BASE_URL}/api/v1/webhooks/chatwoot`, {
			data: payload
		});

		// Should return 200 OK
		expect(response.status()).toBe(200);
	});

	test('POST /webhooks/chatwoot handles unknown event types gracefully', async ({ request }) => {
		const payload = {
			event: 'unknown_event_type',
			data: {}
		};

		const response = await request.post(`${API_BASE_URL}/api/v1/webhooks/chatwoot`, {
			data: payload
		});

		// Should return 200 OK (ignores unknown events)
		expect(response.status()).toBe(200);
	});

	test('POST /webhooks/chatwoot rejects invalid JSON', async ({ request }) => {
		const response = await request.post(`${API_BASE_URL}/api/v1/webhooks/chatwoot`, {
			data: 'invalid json {{{',
			headers: {
				'Content-Type': 'application/json'
			}
		});

		// Should return 400 Bad Request
		expect(response.status()).toBe(400);
	});
});
