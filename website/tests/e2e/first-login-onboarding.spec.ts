import { test, expect } from '@playwright/test';

test.describe('First login onboarding', () => {
	test.skip('guides a new user through all onboarding steps once', async ({ page }) => {
		await page.route('**/api/v1/accounts/*/external-keys', async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({
					success: true,
					data: [],
				}),
			});
		});

		await page.route('**/api/v1/stats', async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({
					success: true,
					data: {
						totalProviders: 10,
						activeProviders: 5,
						totalOfferings: 20,
						totalContracts: 100,
						activeValidators: 3,
						totalTransfers: 50,
						totalVolumeE9s: 1000000000,
					},
				}),
			});
		});

		await page.route('**/api/v1/accounts', async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({
					success: true,
					data: {
						id: 'test-account-id',
						username: 'testuser123',
						email: 'test@example.com',
						created_at: Date.now() * 1000000,
					},
				}),
			});
		});

		await page.route('**/api/v1/prices/icp', async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({ priceUsd: 12.34 }),
			});
		});

		await page.route('**/api/v1/users/*/activity', async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({
					success: true,
					data: {
						rentals_as_requester: [],
						rentals_as_provider: [],
						offerings_provided: [],
					},
				}),
			});
		});

		await page.route('**/api/v1/providers/*/trust-metrics', async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({
					success: true,
					data: {
						trust_score: 90,
						total_contracts: 0,
						completion_rate: 0,
						repeat_customers: 0,
						time_to_delivery_avg_ns: null,
					},
				}),
			});
		});

		await page.route('**/api/v1/provider/my-offerings', async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({
					success: true,
					data: [],
				}),
			});
		});

		const mockSeed = 'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about';
		
		await page.goto('/');
		
		await page.evaluate((phrase) => {
			const stored = JSON.parse(localStorage.getItem('seed_phrases') || '[]');
			if (!stored.includes(phrase)) {
				stored.push(phrase);
				localStorage.setItem('seed_phrases', JSON.stringify(stored));
			}
		}, mockSeed);

		await page.goto('/dashboard');
		await page.waitForLoadState('networkidle');

		await expect(page.getByRole('heading', { name: 'Complete your profile' })).toBeVisible({ timeout: 10000 });
		await page.waitForTimeout(1000);

		await page.locator('.fixed.inset-0.z-50 button:has-text("Continue")').click({ force: true });
		await expect(page.getByRole('heading', { name: 'Add your SSH key' })).toBeVisible();
		await expect(page.getByText('No SSH key found yet. Add one in Security settings.')).toBeVisible();

		await page.locator('.fixed.inset-0.z-50 button:has-text("Continue")').click({ force: true });
		await expect(page.getByRole('heading', { name: 'Choose your next action' })).toBeVisible();

		await page.locator('.fixed.inset-0.z-50 button:has-text("Stay on dashboard")').click({ force: true });
		await expect(page.getByRole('heading', { name: 'Choose your next action' })).not.toBeVisible();

		await page.reload();
		await expect(page.getByRole('heading', { name: 'Complete your profile' })).not.toBeVisible();
	});
});
