import { test, expect } from '@playwright/test';

test.describe('Offline Provider Warning', () => {
	test('should show prominent warning banner when viewing offline provider offering', async ({ page }) => {
		await page.goto('/dashboard/marketplace/45');

		await page.waitForSelector('h1', { timeout: 10000 });

		const warningBanner = page.locator('text=This provider is currently offline');
		await expect(warningBanner).toBeVisible();

		await expect(page.locator('text=Your rental request will be queued')).toBeVisible();
		await expect(page.locator('text=Expected wait: unknown')).toBeVisible();

		const banner = page.locator('.bg-amber-500\\/10').first();
		await expect(banner).toBeVisible();
	});

	test('should show offline badge next to offering title', async ({ page }) => {
		await page.goto('/dashboard/marketplace/45');

		await page.waitForSelector('h1', { timeout: 10000 });

		const offlineBadge = page.locator('span:has-text("Offline")').first();
		await expect(offlineBadge).toBeVisible();
	});

	test('should not show offline warning for online provider offering', async ({ page }) => {
		await page.goto('/dashboard/marketplace');

		await page.waitForSelector('h1:has-text("Marketplace")', { timeout: 10000 });

		await page.waitForTimeout(1000);

		const onlineOffering = page.locator('a[href^="/dashboard/marketplace/"]').first();
		if (await onlineOffering.isVisible()) {
			const href = await onlineOffering.getAttribute('href');
			if (href && !href.includes('/45')) {
				await onlineOffering.click();
				await page.waitForSelector('h1', { timeout: 10000 });

				const warningBanner = page.locator('text=This provider is currently offline');
				const isVisible = await warningBanner.isVisible().catch(() => false);
				expect(isVisible).toBe(false);
			}
		}
	});
});
