import { test, expect } from '@playwright/test';
import { clickAndRetry, revealSeedPhraseOptions } from './fixtures/auth-helpers';

test.describe('Login page registration CTA', () => {
	test('shows a discoverable Create account link on the login page', async ({ page }) => {
		await page.goto('/login');

		// The button is SSR-rendered; toBeVisible auto-retries, so no wait needed.
		await expect(page.getByRole('button', { name: 'Create an account' })).toBeVisible();
	});

	test('Create account link jumps directly to seed backup (generate mode)', async ({ page }) => {
		await page.goto('/login');

		// The button is SSR'd but its onclick binds on hydration; clickAndRetry
		// waits for the resulting "Backup Your Seed Phrase" step to appear.
		const heading = page.getByRole('heading', { name: 'Backup Your Seed Phrase' });
		await clickAndRetry(
			page,
			page.getByRole('button', { name: 'Create an account' }),
			heading,
		);

		// Should land on the "Backup Your Seed Phrase" step, not the choose/import screen
		await expect(heading).toBeVisible();
		// 12 seed-word boxes should be present
		await expect(page.locator('.grid.grid-cols-3 > div')).toHaveCount(12);
	});

	test('Sign in with seed phrase shows the choose (import/generate) screen', async ({ page }) => {
		await page.goto('/login');
		await revealSeedPhraseOptions(page);

		// Existing users land on the mode-chooser
		await expect(page.getByRole('heading', { name: 'Seed Phrase' })).toBeVisible();
		await expect(page.getByRole('button', { name: 'Import Existing' })).toBeVisible();
		await expect(page.getByRole('button', { name: 'Generate New' })).toBeVisible();
	});
});
