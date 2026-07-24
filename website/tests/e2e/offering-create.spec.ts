import { test, expect } from './fixtures/test-account';
import {
	identityFromSeedPhrase,
	signedApiCall,
	deleteOfferingsByProvider,
	pubkeyHexFromSeed,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for the offering CREATE contract (POST /providers/:pubkey/offerings).
 *
 * Coverage gap closed here: "create offering full submit". The create flow was
 * previously broken end-to-end (regression GH #440): the frontend omits `pubkey`
 * from the create body (it is derived from the URL path by the handler), but the
 * backend deserialized the body into the full `Offering` struct, whose `pubkey`
 * field was non-optional with no default — so every create returned 400. The fix
 * is a backend `#[serde(default)]` so the omitted field deserializes to "" and
 * the handler overwrites it from the path.
 *
 * This spec drives the REAL signed create endpoint with a body that deliberately
 * OMITS pubkey, asserting the create succeeds — i.e. it locks in the contract
 * the frontend relies on. No mocks; serial mode + finally cleanup of any rows.
 */

const OFFER_NAME = 'E2E Create Contract Test';

test.describe('Offering create contract (POST /providers/:pubkey/offerings)', () => {
	test.describe.configure({ mode: 'serial' });

	test('create succeeds when the body omits pubkey (path-derived)', async ({ testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const identity = identityFromSeedPhrase(testAccount.seedPhrase);
		const path = `/api/v1/providers/${pubkey}/offerings`;

		// Minimal valid offering payload — mirrors offerings/create handleSubmit,
		// and intentionally omits `pubkey` (the frontend contract).
		const offering = {
			offering_id: `e2e-create-${Date.now()}`,
			offer_name: OFFER_NAME,
			currency: 'USD',
			monthly_price: 9,
			setup_fee: 0,
			visibility: 'public',
			product_type: 'vps',
			virtualization_type: 'kvm',
			billing_interval: 'monthly',
			billing_unit: 'month',
			is_subscription: true,
			subscription_interval_days: 30,
			stock_status: 'in_stock',
			unmetered_bandwidth: false,
			datacenter_country: 'DE',
			datacenter_city: 'Falkenstein',
			is_example: false,
			is_draft: false,
		};

		try {
			const res = await signedApiCall(identity, 'POST', path, offering);
			const json = await res.json();

			// Before the fix this returned 400 "parse request payload error".
			expect(res.status, `create status, body=${JSON.stringify(json)}`).toBe(200);
			expect(json.success).toBe(true);
			expect(typeof json.data).toBe('number');
			expect(json.error).toBeFalsy();
		} finally {
			await deleteOfferingsByProvider(pubkey);
		}
	});
});
