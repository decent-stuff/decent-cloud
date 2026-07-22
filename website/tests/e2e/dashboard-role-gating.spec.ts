import { test, expect } from './fixtures/test-account';

test.describe('Dashboard role-based content gating (M1)', () => {
	test('new user does not see provider trust metrics or red flags', async ({ page }) => {
		await page.goto('/dashboard');
		await expect(page.getByRole('heading', { name: 'Dashboard' })).toBeVisible();

		// Provider-only panels must not appear for a brand-new account
		await expect(page.getByText('Your Trust Score')).toBeHidden({ timeout: 10000 });
		await expect(page.getByText('Red Flags Detected')).toBeHidden();
		await expect(page.getByText('Infrastructure Uptime')).toBeHidden();
		await expect(page.getByText('New Provider')).toBeHidden();

		// New-user CTAs should be visible instead
		await expect(page.getByText('Ready to get started?')).toBeVisible();
		await expect(page.getByRole('link', { name: 'Browse Marketplace Find and' })).toBeVisible();
	});
});
