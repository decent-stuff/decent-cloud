import { test, expect } from './fixtures/test-account';

/**
 * E2E coverage for the dashboard banner stack (#438).
 *
 * Bug: the previous layout used a mutually exclusive {:else if} chain — a
 * seed-phrase user with an unverified email saw ONLY the email banner and
 * NEVER the seed-phrase backup warning, leaving them one mishap away from
 * permanent account loss. The fix stacks both banners (each independently
 * dismissable) inside one fixed container and recomputes <main> padding so
 * the stack never overlaps page content.
 *
 * The default test-account fixture is a seed-phrase identity with an
 * unverified email, which is exactly the scenario the bug silenced.
 */

test.describe('dashboard banner stack (#438)', () => {
	test('seed-phrase + unverified-email user sees BOTH banners simultaneously', async ({ page }) => {
		await page.goto('/dashboard');
		await expect(page.getByRole('button', { name: 'Logout' })).toBeVisible({ timeout: 15000 });

		const emailBanner = page.locator('h3', { hasText: 'Verify Your Email Address' });
		const seedBanner = page.getByText('Back up your seed phrase', { exact: false });

		// Bug #438: previously only the email banner rendered here.
		await expect(emailBanner).toBeVisible({ timeout: 10000 });
		await expect(seedBanner).toBeVisible({ timeout: 10000 });
	});

	test('banners stack vertically without overlap', async ({ page }) => {
		await page.goto('/dashboard');
		await expect(page.getByRole('button', { name: 'Logout' })).toBeVisible({ timeout: 15000 });

		const emailBanner = page.locator('h3', { hasText: 'Verify Your Email Address' }).locator('xpath=ancestor::div[contains(@class,"bg-amber-500")][1]');
		const seedBanner = page.getByText('Back up your seed phrase', { exact: false }).locator('xpath=ancestor::div[contains(@class,"border-amber-500")][1]');

		await expect(emailBanner).toBeVisible({ timeout: 10000 });
		await expect(seedBanner).toBeVisible({ timeout: 10000 });

		const emailBox = await emailBanner.boundingBox();
		const seedBox = await seedBanner.boundingBox();

		expect(emailBox).toBeTruthy();
		expect(seedBox).toBeTruthy();

		// Seed banner must sit BELOW the email banner — never overlapping.
		expect(seedBox!.y).toBeGreaterThanOrEqual(emailBox!.y + emailBox!.height - 1);
	});

	test('dismissing seed banner keeps email banner visible (independent dismissal)', async ({ page }) => {
		await page.goto('/dashboard');
		await expect(page.getByRole('button', { name: 'Logout' })).toBeVisible({ timeout: 15000 });

		const emailBanner = page.locator('h3', { hasText: 'Verify Your Email Address' });
		await expect(emailBanner).toBeVisible({ timeout: 10000 });

		// Dismiss the seed-phrase backup banner via its aria-labelled close button.
		await page.getByRole('button', { name: 'Dismiss' }).click();

		// Seed banner is gone, but email banner survives.
		await expect(page.getByText('Back up your seed phrase', { exact: false })).toHaveCount(0);
		await expect(emailBanner).toBeVisible();
	});

	test('banner exclusion still applies on marketplace and checkout routes', async ({ page }) => {
		// Marketplace and rentals are focus-critical flows — banners stay out of the way.
		// demo=1 surfaces offline demo offerings without needing provider agents.
		await page.goto('/dashboard/marketplace?demo=1&offline=1');
		await expect(page).toHaveURL(/\/dashboard\/marketplace/);
		// Wait for the marketplace shell to render before asserting absence.
		await expect(page.locator('h1:has-text("Marketplace")')).toBeVisible({ timeout: 5000 });

		await expect(page.locator('h3', { hasText: 'Verify Your Email Address' })).toHaveCount(0);
		await expect(page.getByText('Back up your seed phrase', { exact: false })).toHaveCount(0);
	});
});
