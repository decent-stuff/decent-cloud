import { test, expect } from './fixtures/test-account';
import { pubkeyHexFromSeed, sql } from './fixtures/seed-helpers';

/**
 * E2E coverage for the Add Device SUBMIT flow (FLOWS.md "Manage devices /
 * security" — was ⚠️: modal open/cancel + device-name edit only; the actual
 * add-device key-link submit was never asserted).
 *
 * Verified path: AccountOverview "+ Add Device" → AddDeviceModal →
 * SeedPhraseStep "Generate New" → confirm backup → handleAddDevice() signs a
 * POST that INSERTs a row into `account_public_keys` (a second device key for
 * the same account), then reloads the account. The key count goes 1 → 2.
 *
 * Serial mode is mandatory: the test mutates `account_public_keys` for the
 * shared testAccount (all testAccount users share one account/pubkey within a
 * worker). finally restores the single-key state (deletes every key except the
 * original testAccount key) so sibling specs in the same worker (e.g.
 * account-page.spec.ts "1 key" assertions) are not polluted.
 *
 * The added key CASCADES on account teardown, but we don't rely on that because
 * account teardown happens at worker end — too late for sibling files.
 */
test.describe('Add Device submit flow (/dashboard/account/security)', () => {
	test.describe.configure({ mode: 'serial' });

	test('@smoke links a generated device key and raises the device count from 1 to 2', async ({ page, testAccount }) => {
		const originalPubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const restoreSingleKey = async () => {
			await sql(`
				DELETE FROM account_public_keys
				WHERE account_id = (SELECT id FROM accounts WHERE username = '${testAccount.username.replace(/'/g, "''")}')
				AND public_key != decode('${originalPubkey}', 'hex')
			`);
		};

		await page.goto('/dashboard/account/security');

		// Pre-condition: a fresh account has exactly one device key.
		await expect(page.getByText('1 key').first()).toBeVisible({ timeout: 10000 });

		try {
			// Open the Add Device modal.
			await page.getByRole('button', { name: '+ Add Device' }).click();
			await expect(page.getByRole('heading', { name: 'Seed Phrase' })).toBeVisible();

			// Choose "Generate New" → backup step auto-generates a seed phrase.
			await page.locator('button:has-text("Generate New")').click();
			await expect(page.getByRole('heading', { name: 'Backup Your Seed Phrase' })).toBeVisible();

			// Confirm the backup checkbox (unlocks the Continue button) and submit.
			await page.locator('label:has-text("I have saved my seed phrase")').click();
			await page.getByRole('button', { name: 'Continue' }).click();

			// The success step only renders after the signed addAccountKey POST
			// resolves and the account reloads — its heading IS the "key linked"
			// signal (no network-settle wait needed).
			await expect(page.getByRole('heading', { name: 'Device Added!' })).toBeVisible({ timeout: 20000 });

			// Close the modal to reveal the updated device list.
			await page.getByRole('button', { name: 'Done' }).click();
			await expect(page.getByRole('heading', { name: 'Seed Phrase' })).toHaveCount(0);

			// The device count must now read "2 keys" (two account_public_keys rows).
			await expect(page.getByText('2 keys').first()).toBeVisible({ timeout: 10000 });

			// The new key persisted in the DB: exactly two rows for the account.
			const keyCount = await sql(
				`SELECT count(*) FROM account_public_keys WHERE account_id = (SELECT id FROM accounts WHERE username = '${testAccount.username.replace(/'/g, "''")}')`,
			);
			expect(keyCount).toBe('2');
		} finally {
			await restoreSingleKey();
		}
	});
});
