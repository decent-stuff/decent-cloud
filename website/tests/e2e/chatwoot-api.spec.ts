import { test, expect } from '@playwright/test';
import { registerNewAccount, setupConsoleLogging } from './fixtures/auth-helpers';
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
		expect(response.status()).toBe(401);
	});

	test('GET /chatwoot/support-access returns 401 for unauthenticated request', async ({ request }) => {
		const response = await request.get(`${API_BASE_URL}/api/v1/chatwoot/support-access`);
		expect(response.status()).toBe(401);
	});

	test('POST /chatwoot/support-access/reset returns 401 for unauthenticated request', async ({ request }) => {
		const response = await request.post(`${API_BASE_URL}/api/v1/chatwoot/support-access/reset`);
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

	test('GET /chatwoot/support-access returns status for authenticated user', async ({ page }) => {
		setupConsoleLogging(page);
		const credentials = await registerNewAccount(page);
		const { publicKey, privateKey } = seedPhraseToKeyPair(credentials.seedPhrase);

		const path = '/api/v1/chatwoot/support-access';
		const headers = await signRequest(privateKey, publicKey, 'GET', path);

		const response = await page.request.get(`${API_BASE_URL}${path}`, { headers });
		expect(response.status()).toBe(200);

		const data = await response.json();
		expect(data).toHaveProperty('success', true);
		expect(data.data).toHaveProperty('hasAccount');
		expect(data.data).toHaveProperty('loginUrl');
		expect(typeof data.data.hasAccount).toBe('boolean');
	});

	test('POST /chatwoot/support-access/reset returns error without Platform API', async ({ page }) => {
		setupConsoleLogging(page);
		const credentials = await registerNewAccount(page);
		const { publicKey, privateKey } = seedPhraseToKeyPair(credentials.seedPhrase);

		const path = '/api/v1/chatwoot/support-access/reset';
		const headers = await signRequest(privateKey, publicKey, 'POST', path);

		const response = await page.request.post(`${API_BASE_URL}${path}`, { headers });
		expect(response.status()).toBe(200);

		const data = await response.json();
		// Without Platform API configured, should return error
		// Either "Platform API not configured" or "No support portal account"
		expect(data.success).toBe(false);
		expect(data.error).toBeTruthy();
	});
});

test.describe('Provider Message Response Metrics', () => {
	test('GET /providers/:pubkey/response-metrics returns message response metrics', async ({
		request
	}) => {
		// Use a valid 32-byte hex pubkey (64 chars)
		const validPubkey = '0'.repeat(64);
		const response = await request.get(
			`${API_BASE_URL}/api/v1/providers/${validPubkey}/response-metrics`
		);

		expect(response.status()).toBe(200);

		const data = await response.json();
		expect(data.success).toBe(true);
		// Response format from MessagesApi endpoint (message thread response times)
		expect(data.data).toHaveProperty('avgResponseTimeHours');
		expect(data.data).toHaveProperty('responseRatePct');
		expect(data.data).toHaveProperty('totalThreads');
		expect(data.data).toHaveProperty('respondedThreads');
	});

	test('GET /providers/:pubkey/response-metrics returns error for invalid pubkey', async ({
		request
	}) => {
		const response = await request.get(
			`${API_BASE_URL}/api/v1/providers/invalid-pubkey/response-metrics`
		);

		expect(response.status()).toBe(200);
		const data = await response.json();
		expect(data.success).toBe(false);
		expect(data.error).toContain('Invalid provider pubkey format');
	});
});

test.describe('Provider Contract Response Metrics', () => {
	test('GET /providers/:pubkey/contract-response-metrics returns contract response metrics', async ({
		request
	}) => {
		// Use a valid 32-byte hex pubkey (64 chars)
		const validPubkey = '0'.repeat(64);
		const response = await request.get(
			`${API_BASE_URL}/api/v1/providers/${validPubkey}/contract-response-metrics`
		);

		expect(response.status()).toBe(200);

		const data = await response.json();
		expect(data.success).toBe(true);
		// Response format from ProvidersApi endpoint (contract status response times)
		expect(data.data).toHaveProperty('avgResponseSeconds');
		expect(data.data).toHaveProperty('avgResponseHours');
		expect(data.data).toHaveProperty('slaCompliancePercent');
		expect(data.data).toHaveProperty('breachCount30d');
		expect(data.data).toHaveProperty('totalInquiries30d');
		expect(data.data).toHaveProperty('distribution');
		expect(data.data.distribution).toHaveProperty('within1hPct');
		expect(data.data.distribution).toHaveProperty('within4hPct');
		expect(data.data.distribution).toHaveProperty('within12hPct');
		expect(data.data.distribution).toHaveProperty('within24hPct');
		expect(data.data.distribution).toHaveProperty('within72hPct');
		expect(data.data.distribution).toHaveProperty('totalResponses');
	});

	test('GET /providers/:pubkey/contract-response-metrics returns error for invalid pubkey', async ({
		request
	}) => {
		const response = await request.get(
			`${API_BASE_URL}/api/v1/providers/invalid-pubkey/contract-response-metrics`
		);

		expect(response.status()).toBe(200);
		const data = await response.json();
		expect(data.success).toBe(false);
		expect(data.error).toContain('Invalid pubkey format');
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
