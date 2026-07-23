import { test, expect } from './fixtures/test-account';
import { setupConsoleLogging } from './fixtures/auth-helpers';
import { seedRentableOffering, deleteOfferingsByProvider } from './fixtures/seed-helpers';

/**
 * Proves the seedRentableOffering() fixture yields a rentable marketplace offering:
 * a self_provisioned public offering under a random non-example provider pubkey is
 * online (provider_online=true) and not flagged is_example, so its "Rent Resource"
 * button is enabled. This unblocks the payment-flows / post-rental skip gaps.
 */
test.describe('Rentable offering fixture', () => {
	let seeded: { providerPubkeyHex: string; offeringName: string };

	test.beforeAll(async () => {
		seeded = await seedRentableOffering({ name: 'Fixture PoC Rentable' });
	});
	test.afterAll(async () => {
		await deleteOfferingsByProvider(seeded.providerPubkeyHex);
	});

	test('seeded self_provisioned offering shows an enabled Rent Resource button', async ({ page }) => {
		setupConsoleLogging(page);
		await page.goto('/dashboard/marketplace');
		await expect(page.locator('h1:has-text("Marketplace")')).toBeVisible();

		// The offering card for our seeded name must be present.
		const card = page.locator(`text=${seeded.offeringName}`).first();
		await expect(card).toBeVisible({ timeout: 10000 });

		// At least one enabled "Rent" button must exist. The marketplace renders
		// the action as <button>Rent</button> (enabled) for online non-example
		// offerings, "Demo only" (disabled) for examples, or hides offline ones.
		// getByRole({exact:true}) avoids matching "Rentals"/"Rental Requests".
		const enabledRent = page.getByRole('button', { name: 'Rent', exact: true }).first();
		await expect(enabledRent).toBeVisible({ timeout: 5000 });

		// Clicking it opens the rental dialog (whose title IS "Rent Resource").
		await enabledRent.click();
		await expect(page.getByRole('heading', { name: 'Rent Resource' })).toBeVisible({ timeout: 5000 });
	});
});
