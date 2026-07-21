import { test, expect } from './fixtures/test-account';
import type { Page } from '@playwright/test';

/**
 * E2E coverage for the /dashboard overview page (top-level dashboard index).
 *
 * The "My Resources" card always renders for an authenticated identity, even
 * with zero offerings — its subtitle copy is therefore stable and safe to
 * assert against without seeding.
 */

// Mirrors the helper in transfers.spec.ts: the test-account fixture injects
// the seed phrase via addInitScript, then the dashboard layout renders the
// "Logout" button once auth state propagates. Waiting on that button IS the
// auth-ready signal.
async function waitForAuthReady(page: Page) {
	await page.getByRole('button', { name: 'Logout' }).waitFor({ state: 'visible', timeout: 15000 });
}

test.describe('/dashboard overview', () => {
	test('My Resources subtitle uses unambiguous self-test copy (audit #2)', async ({ page }) => {
		// Audit #2: the previous subtitle read "Your infrastructure offerings -
		// rent for free (self-rental)" with a "Rent Free" button, which left
		// providers unsure what "free" meant or why they'd rent their own
		// offering. The copy was renamed to make the self-test semantics
		// explicit.
		await page.goto('/dashboard');
		await waitForAuthReady(page);

		const myResources = page.locator('h2', { hasText: 'My Resources' }).locator('..');
		// New wording must explicitly reference provisioning a test instance —
		// the literal "rent for free" phrasing must be gone.
		await expect(myResources).toContainText(/provision a test instance/i);
		await expect(myResources).not.toContainText(/rent for free/i);
	});
});
