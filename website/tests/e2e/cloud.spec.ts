import { test, expect } from './fixtures/test-account';

/**
 * E2E coverage for /dashboard/cloud (the cloud-accounts + cloud-resources leaves).
 *
 * Both leaf routes are auth-required in practice: their onMount handlers
 * only fetch data when an identity is present, so an anonymous visit sits
 * forever on "Loading...". The authenticated tests below assert the real
 * route content (headings, action buttons, and the unauthenticated-user
 * empty state), which is what proves the route actually mounted.
 *
 * Seeding cloud accounts/resources would require valid upstream credentials
 * (Hetzner/Proxmox), so we assert the empty-state scaffolding instead —
 * that's enough to prove the route rendered its real content.
 */

test.describe('/dashboard/cloud', () => {
	// Helper: each page.goto() reloads the SPA, so we have to wait for the
	// auth store to re-initialise before asserting content. Same pattern as
	// transfers.spec.ts — the Logout button is the auth-ready signal.
	async function waitForAuthReady(page: import('@playwright/test').Page) {
		await page.getByRole('button', { name: 'Logout' }).waitFor({ state: 'visible', timeout: 15000 });
	}

	test.describe('cloud accounts', () => {
		test('authenticated visit renders the Cloud Accounts heading and Add Account button', async ({ page }) => {
			await page.goto('/dashboard/cloud/accounts');
			await waitForAuthReady(page);

			await expect(page.getByRole('heading', { name: 'Cloud Accounts', exact: true })).toBeVisible();
			await expect(
				page.getByText('Connect your Hetzner or Proxmox accounts for self-provisioning'),
			).toBeVisible();
			await expect(page.getByRole('button', { name: 'Add Account' })).toBeVisible();
		});

		test('empty state: fresh user sees "No cloud accounts connected"', async ({ page }) => {
			await page.goto('/dashboard/cloud/accounts');
			await waitForAuthReady(page);

			// Empty-state copy is unique to this route's empty branch
			await expect(page.getByText('No cloud accounts connected')).toBeVisible();
			await expect(page.getByText('Add your first cloud account')).toBeVisible();
		});

		test('"Add Account" modal exposes the Hetzner + Proxmox provider options', async ({ page }) => {
			await page.goto('/dashboard/cloud/accounts');
			await waitForAuthReady(page);

			await page.getByRole('button', { name: 'Add Account' }).click();

			// The provider <select> options are unique to this route's modal.
			// <option> elements are present but not "visible" until opened, so
			// assert on the select itself plus its child option text.
			await expect(page.getByRole('heading', { name: 'Add Cloud Account' })).toBeVisible();
			const providerSelect = page.locator('select#backendType');
			await expect(providerSelect).toBeVisible();
			await expect(providerSelect.locator('option', { hasText: 'Hetzner Cloud' })).toHaveCount(1);
			await expect(providerSelect.locator('option', { hasText: 'Proxmox VE' })).toHaveCount(1);
			await expect(page.getByLabel('Account Name')).toBeVisible();
		});
	});

	test.describe('cloud resources', () => {
		test('authenticated visit renders the Cloud Resources heading and Provision VM button', async ({ page }) => {
			await page.goto('/dashboard/cloud/resources');
			await waitForAuthReady(page);

			await expect(page.getByRole('heading', { name: 'Cloud Resources', exact: true })).toBeVisible();
			await expect(
				page.getByText('Self-provisioned VMs on your connected cloud accounts'),
			).toBeVisible();
			// The "Provision VM" button is disabled when no valid account exists,
			// but is still rendered — assert its presence, not its enabled state.
			await expect(page.getByRole('button', { name: 'Provision VM' })).toBeVisible();
		});

		test('empty state: fresh user without cloud accounts sees the validation hint', async ({ page }) => {
			await page.goto('/dashboard/cloud/resources');
			await waitForAuthReady(page);

			// Fresh user has no valid cloud accounts → the yellow hint renders.
			// This copy is unique to this route's "needs account" branch.
			await expect(
				page.getByText(/You need to add a valid cloud account before you can provision resources/i),
			).toBeVisible();
			await expect(page.getByRole('link', { name: 'Add a cloud account' })).toHaveAttribute(
				'href',
				'/dashboard/cloud/accounts',
			);
		});
	});
});
