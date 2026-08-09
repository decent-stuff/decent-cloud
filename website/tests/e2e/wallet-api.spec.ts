import { test, expect } from './fixtures/test-account';
import { identityFromSeedPhrase, signedApiCall, pubkeyHexFromSeed } from './fixtures/seed-helpers';

/**
 * Wallet API endpoint verification.
 *
 * Verifies the pre-pay wallet endpoints (GET /wallet, POST /wallet/topup) work
 * end-to-end against the warm stack: auth enforcement, balance retrieval,
 * input validation, and graceful degradation when Stripe is not configured.
 *
 * These are API-level tests (no UI); the wallet UI page is covered separately.
 */
test.describe('Wallet API', () => {
	test('GET /wallet returns null balance + empty ledger for a new user', async ({ testAccount }) => {
		const identity = identityFromSeedPhrase(testAccount.seedPhrase);
		const pubkeyHex = pubkeyHexFromSeed(testAccount.seedPhrase);

		const res = await signedApiCall(identity, 'GET', `/api/v1/users/${pubkeyHex}/wallet`);
		expect(res.status).toBe(200);
		const json = await res.json();
		expect(json.success).toBe(true);
		expect(json.data.balanceE9s).toBeNull();
		expect(json.data.recentLedger).toEqual([]);
	});

	test('POST /wallet/topup rejects non-positive amount', async ({ testAccount }) => {
		const identity = identityFromSeedPhrase(testAccount.seedPhrase);
		const pubkeyHex = pubkeyHexFromSeed(testAccount.seedPhrase);

		const res = await signedApiCall(
			identity,
			'POST',
			`/api/v1/users/${pubkeyHex}/wallet/topup`,
			{ amountUsd: 0 },
		);
		expect(res.status).toBe(200);
		const json = await res.json();
		expect(json.success).toBe(false);
		expect(json.error).toContain('greater than zero');
	});

	test('POST /wallet/topup rejects unauthorized pubkey', async ({ testAccount }) => {
		const identity = identityFromSeedPhrase(testAccount.seedPhrase);

		const res = await signedApiCall(
			identity,
			'POST',
			`/api/v1/users/0000000000000000000000000000000000000000000000000000000000000000/wallet/topup`,
			{ amountUsd: 10 },
		);
		expect(res.status).toBe(200);
		const json = await res.json();
		expect(json.success).toBe(false);
		expect(json.error).toContain('Unauthorized');
	});

	test('POST /wallet/topup returns clear error when Stripe is not configured', async ({ testAccount }) => {
		// Local dev stack has no STRIPE_SECRET_KEY — the endpoint must fail
		// fast with an actionable message, not a panic or silent 500.
		const identity = identityFromSeedPhrase(testAccount.seedPhrase);
		const pubkeyHex = pubkeyHexFromSeed(testAccount.seedPhrase);

		const res = await signedApiCall(
			identity,
			'POST',
			`/api/v1/users/${pubkeyHex}/wallet/topup`,
			{ amountUsd: 25 },
		);
		expect(res.status).toBe(200);
		const json = await res.json();
		expect(json.success).toBe(false);
		expect(json.error).toContain('Stripe');
	});
});
