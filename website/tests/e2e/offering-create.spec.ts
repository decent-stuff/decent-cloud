import { test, expect, waitForAuthReady } from './fixtures/test-account';
import {
	identityFromSeedPhrase,
	signedApiCall,
	deleteOfferingsByProvider,
	pubkeyHexFromSeed,
	accountIdHex,
	seedCloudAccount,
	deleteCloudAccountsForAccount,
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
 *
 * The second describe block covers the monthly-price auto-suggest (GH #442):
 * when a Hetzner server with a known cost is selected, #monthly-price is
 * pre-filled with `cost × 1.15` (provider-overridable). The catalog endpoint
 * fundamentally requires a real Hetzner API token (outbound external HTTP),
 * which is unavailable in the test environment, so we mock the catalog
 * RESPONSE at the API boundary (an explicit, documented exception to the
 * no-first-party-mocks rule: all other calls — list cloud accounts, create
 * offering — go through the real API and the real signed-submit logic).
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

/**
 * E2E coverage for the monthly-price auto-suggest in the create-offering
 * wizard (GH #442): when a Hetzner server with a known cost is selected,
 * `#monthly-price` is pre-filled with `cost × 1.15` (provider-overridable).
 *
 * Catalog mock: the catalog endpoint depends on a real Hetzner API token
 * (outbound external HTTP), unavailable in the test environment. We mock the
 * catalog response at the API boundary. This is the only mock — the create
 * submit, account lookup, and cloud-accounts list all hit the real API.
 */
const HETZNER_SERVER_COST = 4.59; // cx22-style monthly cost
const EXPECTED_SUGGESTION = Math.round(HETZNER_SERVER_COST * 1.15 * 100) / 100; // 5.28
const OVERRIDE_PRICE = 12.34;

test.describe('Create-offering wizard: monthly-price auto-suggest (#442)', () => {
	test.describe.configure({ mode: 'serial' });

	test('monthly price is pre-filled with cost × 1.15 when a Hetzner server is selected', async ({ page, testAccount }) => {
		const accountHex = await accountIdHex(testAccount.username);
		const cloudAccountName = `E2E Suggest ${Date.now()}`;
		await seedCloudAccount(accountHex, { name: cloudAccountName });
		try {
			// Mock only the catalog endpoint — its content comes from a real
			// Hetzner API call that requires credentials unavailable in tests.
			await page.route('**/api/v1/cloud-accounts/*/catalog', (route) =>
				route.fulfill({
					status: 200,
					contentType: 'application/json',
					body: JSON.stringify({
						success: true,
						data: {
							serverTypes: [
								{
									id: '1',
									name: 'cx22',
									cores: 2,
									memoryGb: 4,
									diskGb: 40,
									priceMonthly: HETZNER_SERVER_COST,
								},
							],
							locations: [{ id: '1', name: 'nbg1', city: 'Nuremberg', country: 'DE' }],
							images: [{ id: '1', name: 'ubuntu-24.04', osType: 'linux' }],
						},
						error: null,
					}),
				}),
			);

			await page.goto('/dashboard/offerings/create');
			await waitForAuthReady(page);

			// Step 1: Basics — minimal valid input.
			await page.locator('#offer-name').fill('E2E Suggest Test');
			await page.locator('#offering-id').fill(`e2e-suggest-${Date.now()}`);
			await page.getByRole('button', { name: /Next: Infrastructure/ }).click();

			// Step 2: Infrastructure — pick the seeded cloud account, then a server.
			await page.locator('#cloud-account').selectOption({ index: 1 });
			await expect(page.locator('#server-type')).toBeVisible({ timeout: 10_000 });
			await page.locator('#server-type').selectOption('cx22');
			await page.locator('#location').selectOption('nbg1');
			await page.locator('#image').selectOption('ubuntu-24.04');
			await page.getByRole('button', { name: /Next: Pricing/ }).click();

			// Step 3: Pricing — assert the monthly-price input is pre-filled
			// with cost × 1.15 (rounded to 2 decimals), and the hint mentions
			// the markup so the provider knows it is editable.
			const monthlyPriceInput = page.locator('#monthly-price');
			await expect(monthlyPriceInput).toHaveValue(EXPECTED_SUGGESTION.toString());
			await expect(page.getByText(/suggested at 15% markup/i)).toBeVisible();
		} finally {
			await deleteCloudAccountsForAccount(accountHex);
		}
	});

	test('provider can override the suggested monthly price and the override is what gets submitted', async ({ page, testAccount }) => {
		const accountHex = await accountIdHex(testAccount.username);
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const cloudAccountName = `E2E Override ${Date.now()}`;
		await seedCloudAccount(accountHex, { name: cloudAccountName });
		try {
			await page.route('**/api/v1/cloud-accounts/*/catalog', (route) =>
				route.fulfill({
					status: 200,
					contentType: 'application/json',
					body: JSON.stringify({
						success: true,
						data: {
							serverTypes: [
								{
									id: '1',
									name: 'cx22',
									cores: 2,
									memoryGb: 4,
									diskGb: 40,
									priceMonthly: HETZNER_SERVER_COST,
								},
							],
							locations: [{ id: '1', name: 'nbg1', city: 'Nuremberg', country: 'DE' }],
							images: [{ id: '1', name: 'ubuntu-24.04', osType: 'linux' }],
						},
						error: null,
					}),
				}),
			);

			await page.goto('/dashboard/offerings/create');
			await waitForAuthReady(page);

			await page.locator('#offer-name').fill('E2E Override Test');
			await page.locator('#offering-id').fill(`e2e-override-${Date.now()}`);
			await page.getByRole('button', { name: /Next: Infrastructure/ }).click();

			await page.locator('#cloud-account').selectOption({ index: 1 });
			await expect(page.locator('#server-type')).toBeVisible({ timeout: 10_000 });
			await page.locator('#server-type').selectOption('cx22');
			await page.locator('#location').selectOption('nbg1');
			await page.locator('#image').selectOption('ubuntu-24.04');
			await page.getByRole('button', { name: /Next: Pricing/ }).click();

			// Sanity: suggestion is in place before we override.
			const monthlyPriceInput = page.locator('#monthly-price');
			await expect(monthlyPriceInput).toHaveValue(EXPECTED_SUGGESTION.toString());

			// Override: clear and type the custom price.
			await monthlyPriceInput.fill('');
			await monthlyPriceInput.fill(OVERRIDE_PRICE.toString());
			await expect(monthlyPriceInput).toHaveValue(OVERRIDE_PRICE.toString());

			// Submit and capture the actual POST request body — the override
			// (not the suggestion) must be what reaches the API.
			//
			// We assert on the REQUEST body, not the response: a Hetzner-provisioned
			// offering goes through `validate_hetzner_offering_inner`, which needs
			// CREDENTIAL_ENCRYPTION_KEY + real Hetzner creds to decrypt. The warm
			// stack leaves that env unset, so the validator bails with a
			// credential-encryption error UNRELATED to this feature. The
			// "what gets submitted" assertion is on the request, which fires
			// before the response regardless.
			const createPath = `/api/v1/providers/${pubkey}/offerings`;
			const [createRequest] = await Promise.all([
				page.waitForRequest(
					(req) => req.method() === 'POST' && req.url().includes(createPath),
					{ timeout: 15_000 },
				),
				page.getByRole('button', { name: /Create Offering/ }).click(),
			]);

			const submittedMonthly = createRequest.postDataJSON().monthly_price;
			expect(
				submittedMonthly,
				'submitted monthly_price must be the override, not the suggestion',
			).toBe(OVERRIDE_PRICE);
		} finally {
			// Clean up both the cloud account and any offering row the real submit created.
			await deleteOfferingsByProvider(pubkey);
			await deleteCloudAccountsForAccount(accountHex);
		}
	});
});
