import { test, expect, waitForAuthReady } from './fixtures/test-account';
import {
	accountIdHex,
	seedCloudAccount,
	deleteCloudAccountsForAccount,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for /dashboard/cloud (the cloud-accounts + cloud-resources leaves).
 *
 * Both leaf routes are auth-required in practice: their onMount handlers
 * only fetch data when an identity is present, so an anonymous visit sits
 * forever on "Loading...". The authenticated tests below assert the real
 * route content (headings, action buttons, the unauthenticated-user empty
 * state, AND the populated state driven by a DB-seeded cloud_accounts row),
 * which is what proves the route actually mounted.
 *
 * The populated-state tests seed a `cloud_accounts` row directly under the
 * testAccount's account_id (the list endpoint filters by caller) — no real
 * Hetzner/Proxmox connection is exercised, but the row renders in the list
 * and the modal-based disconnect fires a real signed DELETE.
 */

test.describe('/dashboard/cloud', () => {
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

		// Serial mode: these tests seed + delete cloud_accounts rows for the
		// shared testAccount pubkey, so they must not run in parallel.
		test.describe.configure({ mode: 'serial' });

		test('populated state: a DB-seeded cloud account renders in the list', async ({ page, testAccount }) => {
			const accountHex = await accountIdHex(testAccount.username);
			const accountName = `E2E Populated ${Date.now()}`;
			await seedCloudAccount(accountHex, { name: accountName });
			try {
				await page.goto('/dashboard/cloud/accounts');
				await waitForAuthReady(page);

				// The empty-state copy must be gone (the list is non-empty now).
				await expect(page.getByText('No cloud accounts connected')).toHaveCount(0);

				// The seeded account renders with its name + the Hetzner backend label.
				const row = page.locator('div.bg-surface', { hasText: accountName }).first();
				await expect(row).toBeVisible({ timeout: 10000 });
				await expect(row.getByRole('heading', { name: accountName })).toBeVisible();
				await expect(row.getByText(/Hetzner Cloud/)).toBeVisible();
				// The seeded row defaults to is_valid=true → "Valid" badge.
				await expect(row.getByText('Valid')).toBeVisible();
			} finally {
				await deleteCloudAccountsForAccount(accountHex);
			}
		});

		test('disconnect: the modal delete flow removes the cloud account', async ({ page, testAccount }) => {
			const accountHex = await accountIdHex(testAccount.username);
			const accountName = `E2E Disconnect ${Date.now()}`;
			await seedCloudAccount(accountHex, { name: accountName });
			try {
				await page.goto('/dashboard/cloud/accounts');
				await waitForAuthReady(page);

				const row = page.locator('div.bg-surface', { hasText: accountName }).first();
				await expect(row).toBeVisible({ timeout: 10000 });

				// The trash button opens a modal confirm (not inline — distinct
				// from the inline-confirm pattern covered elsewhere). Its
				// accessible name comes from the title="Delete account" attr.
				await row.getByRole('button', { name: 'Delete account' }).click();

				// The modal confirm dialog appears with a clear warning + Delete button.
				const dialog = page.getByRole('dialog');
				await expect(dialog.getByRole('heading', { name: 'Delete Cloud Account?' })).toBeVisible();
				const deleteBtn = dialog.getByRole('button', { name: 'Delete' });
				await expect(deleteBtn).toBeVisible();

				// Wait for the signed DELETE to round-trip before asserting removal.
				const deleteResponse = page.waitForResponse(
					(resp) => resp.request().method() === 'DELETE' && resp.url().includes('/api/v1/cloud-accounts/'),
					{ timeout: 15000 },
				);
				await deleteBtn.click();
				await deleteResponse;

				// The row disappears after the list refetches.
				await expect(page.locator('div.bg-surface', { hasText: accountName })).toHaveCount(0, { timeout: 10000 });
			} finally {
				await deleteCloudAccountsForAccount(accountHex);
			}
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
