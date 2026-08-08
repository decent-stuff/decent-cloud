import { test, expect } from './fixtures/test-account';

/**
 * E2E coverage for the consolidated dashboard action indicator (A8 / UX-009).
 *
 * Previously the dashboard showed two stacked full-width colored banners
 * (orange verify-email + yellow seed-backup) that dominated the top real
 * estate. They are now merged into ONE compact, dismissible one-line bar
 * that summarizes pending actions and expands inline to reveal their CTAs.
 *
 * The default test-account fixture is a seed-phrase identity with an
 * unverified email — exactly the scenario that previously stacked both
 * full-width banners.
 */

test.describe('dashboard action-required indicator (A8 / UX-009)', () => {
	test('seed-phrase + unverified-email user sees a single compact indicator, not stacked banners', async ({ page }) => {
		await page.goto('/dashboard');
		await expect(page.getByRole('button', { name: 'Logout' })).toBeVisible({ timeout: 15000 });

		const indicator = page.getByTestId('action-required-banner');
		await expect(indicator).toBeVisible({ timeout: 10000 });

		await expect(indicator).toContainText('2 actions needed');
		await expect(indicator).toContainText('verify your email');
		await expect(indicator).toContainText('back up your seed phrase');

		// The legacy full-width colored banners are gone.
		await expect(page.locator('h3', { hasText: 'Verify Your Email Address' })).toHaveCount(0);
	});

	test('indicator is collapsed by default; expanding reveals both CTAs', async ({ page }) => {
		await page.goto('/dashboard');
		await expect(page.getByRole('button', { name: 'Logout' })).toBeVisible({ timeout: 15000 });

		const indicator = page.getByTestId('action-required-banner');
		await expect(indicator).toBeVisible({ timeout: 10000 });

		// CTAs are hidden until the user expands.
		await expect(page.getByRole('button', { name: 'Resend email' })).toHaveCount(0);
		await expect(page.getByRole('link', { name: 'Back up now' })).toHaveCount(0);

		await page.getByRole('button', { name: 'Review' }).click();

		await expect(page.getByRole('button', { name: 'Resend email' })).toBeVisible();
		await expect(page.getByRole('link', { name: 'Back up now' })).toBeVisible();
	});

	test('dismissing one action keeps the indicator showing the remaining action', async ({ page }) => {
		await page.goto('/dashboard');
		await expect(page.getByRole('button', { name: 'Logout' })).toBeVisible({ timeout: 15000 });

		const indicator = page.getByTestId('action-required-banner');
		await expect(indicator).toBeVisible({ timeout: 10000 });

		await page.getByRole('button', { name: 'Review' }).click();
		await expect(page.getByRole('link', { name: 'Back up now' })).toBeVisible();

		// Dismiss only the seed-phrase action.
		await page.getByRole('button', { name: 'Dismiss back up your seed phrase', exact: true }).click();

		await expect(indicator).toContainText('1 action needed');
		await expect(indicator).toContainText('verify your email');
		await expect(indicator).not.toContainText('back up your seed phrase');
	});

	test('dismiss-all removes the indicator entirely', async ({ page }) => {
		await page.goto('/dashboard');
		await expect(page.getByRole('button', { name: 'Logout' })).toBeVisible({ timeout: 15000 });

		const indicator = page.getByTestId('action-required-banner');
		await expect(indicator).toBeVisible({ timeout: 10000 });

		await page.getByRole('button', { name: 'Dismiss all action reminders' }).click();

		await expect(indicator).toHaveCount(0);
	});

	test('indicator stays off marketplace and checkout routes (F4 confinement)', async ({ page }) => {
		await page.goto('/dashboard/marketplace?demo=1&offline=1');
		await expect(page).toHaveURL(/\/dashboard\/marketplace/);
		await expect(page.locator('h1:has-text("Marketplace")')).toBeVisible({ timeout: 5000 });

		await expect(page.getByTestId('action-required-banner')).toHaveCount(0);
	});
});
