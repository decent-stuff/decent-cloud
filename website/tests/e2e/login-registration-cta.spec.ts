import { test, expect } from '@playwright/test';

test.describe('Login page registration CTA', () => {
	test('shows a discoverable Create account link on the login page', async ({ page }) => {
		await page.goto('/login');
		await page.waitForLoadState('networkidle');

		await expect(page.getByRole('button', { name: 'Create an account' })).toBeVisible();
	});

	test('Create account link jumps directly to seed backup (generate mode)', async ({ page }) => {
		await page.goto('/login');
		await page.waitForLoadState('networkidle');

		await page.getByRole('button', { name: 'Create an account' }).click();

		// Should land on the "Backup Your Seed Phrase" step, not the choose/import screen
		await expect(page.getByRole('heading', { name: 'Backup Your Seed Phrase' })).toBeVisible();
		// 12 seed-word boxes should be present
		await expect(page.locator('.grid.grid-cols-3 > div')).toHaveCount(12);
	});

	test('Sign in with seed phrase shows the choose (import/generate) screen', async ({ page }) => {
		await page.goto('/login');
		await page.waitForLoadState('networkidle');

		await page.getByRole('button', { name: 'Sign in with seed phrase instead' }).click();

		// Existing users land on the mode-chooser
		await expect(page.getByRole('heading', { name: 'Seed Phrase' })).toBeVisible();
		await expect(page.getByRole('button', { name: 'Import Existing' })).toBeVisible();
		await expect(page.getByRole('button', { name: 'Generate New' })).toBeVisible();
	});
});
