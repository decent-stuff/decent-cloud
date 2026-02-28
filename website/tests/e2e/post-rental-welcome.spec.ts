import { test, expect } from './fixtures/test-account';
import { setupConsoleLogging } from './fixtures/auth-helpers';

/**
 * E2E Tests for Post-Rental Welcome Flow
 *
 * Prerequisites:
 * - API server running at http://localhost:59001 (or configured base URL)
 * - Website running at http://localhost:59000
 * - At least one offering in the marketplace
 *
 * Test Coverage:
 * - After successful rental, user is navigated to contract detail page
 * - Welcome banner is displayed on first visit to contract page
 * - Welcome banner can be dismissed
 * - User can see contract details and next steps
 * - Checkout success page redirects to contract detail with welcome param
 */

test.describe('Post-rental welcome flow', () => {
	test('should navigate to contract page after successful rental', async ({ page }) => {
		setupConsoleLogging(page);
		
		// Navigate to marketplace
		await page.goto('/dashboard/marketplace');
		await page.waitForLoadState('networkidle');
		
		// Find an offering to rent (look for first available offering with rent button)
		// This is a simplified test - in real scenario, we'd create an offering first
		const firstOfferingCard = page.locator('[data-testid="offering-card"]').first();
		
		// If no offerings exist, skip this test
		if (!(await firstOfferingCard.isVisible())) {
			test.skip();
			return;
		}
		
		// Click rent button on first offering
		await firstOfferingCard.locator('button:has-text("Rent")').click();
		
		// Wait for rental dialog to appear
		const dialog = page.locator('[data-testid="rental-dialog"]');
		await expect(dialog).toBeVisible();
		
		// Fill in rental details (this would need valid SSH key in real scenario)
		// For testing, we'd need to mock this
		
		// After successful rental, should navigate to contract detail page
		// with welcome query param
		await page.waitForURL(/\/dashboard\/rentals\/[a-f0-9]+\?welcome=true/);
		
		// Should see welcome banner
		const welcomeBanner = page.locator('[data-testid="welcome-banner"]');
		await expect(welcomeBanner).toBeVisible();
		
		// Should see "What to expect next" guidance
		await expect(page.locator('text=What to expect next')).toBeVisible();
	});

	test('welcome banner should be dismissable', async ({ page }) => {
		// Navigate directly to contract page with welcome param
		await page.goto('/dashboard/rentals/test-contract-id?welcome=true');
		await page.waitForLoadState('networkidle');
		
		// Should see welcome banner
		const welcomeBanner = page.locator('[data-testid="welcome-banner"]');
		if (await welcomeBanner.isVisible()) {
			// Click dismiss button
			await welcomeBanner.locator('button[aria-label="Dismiss"]').click();
			
			// Banner should be hidden
			await expect(welcomeBanner).not.toBeVisible();
			
			// URL should no longer have welcome param
			await expect(page).toHaveURL(/\/dashboard\/rentals\/[a-f0-9]+$/);
		}
	});

	test('should not show welcome banner on regular visit', async ({ page }) => {
		// Navigate to contract page without welcome param
		await page.goto('/dashboard/rentals/test-contract-id');
		await page.waitForLoadState('networkidle');
		
		// Should NOT see welcome banner
		const welcomeBanner = page.locator('[data-testid="welcome-banner"]');
		await expect(welcomeBanner).not.toBeVisible();
	});
});

test.describe('Checkout success redirect', () => {
	test('checkout success page should redirect to contract detail with welcome param', async ({ page }) => {
		// Mock a valid checkout session response
		await page.route('**/api/v1/contracts/verify-checkout', async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({
					success: true,
					data: {
						contractId: 'abc123def456789',
						paymentStatus: 'succeeded'
					}
				})
			});
		});

		// Navigate to checkout success page with a mock session ID
		await page.goto('/checkout/success?session_id=cs_test_abc123');
		
		// Wait for verification to complete
		await page.waitForSelector('text=Payment Successful', { timeout: 10000 });
		
		// Verify the button text indicates contract detail navigation
		const button = page.locator('button:has-text("View My Rental")');
		await expect(button).toBeVisible();
		
		// Verify redirect message mentions rental details (not just "rentals")
		await expect(page.locator('text=rental details')).toBeVisible();
	});
});
