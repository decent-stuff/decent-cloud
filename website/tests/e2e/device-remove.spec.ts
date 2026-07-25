import { test, expect } from './fixtures/test-account';
import { assertNoNativeDialog } from './fixtures/auth-helpers';
import { sql, randomHex, accountIdHex } from './fixtures/seed-helpers';

/**
 * E2E coverage for the device-key remove flow (inline two-step confirm).
 *
 * AccountOverview previously used a native confirm() dialog on device removal
 * — which both blocks headless e2e (Playwright auto-dismisses it, so the
 * removal never fires) and is a poor mobile UX. It now uses the inline
 * two-step pattern (first click reveals an inline Confirm button, second
 * click removes), mirroring the offerings delete (commit 1077dd33).
 *
 * A device is only removable when more than one active key exists
 * (canRemoveKey guards the last active key). The testAccount starts with one
 * active key, so each test seeds a SECOND active key (a real device row) so
 * the Remove affordance appears, then removes/cancels on that seeded key.
 *
 * Serial mode: the spec mutates account_public_keys for the shared testAccount
 * pubkey, so tests must not run in parallel.
 */
test.describe.configure({ mode: 'serial' });

/** Seed a second active key with a device name; returns its 16-byte hex id. */
async function seedDevice(accountHex: string, deviceName: string): Promise<string> {
	const keyIdHex = randomHex(16);
	const pubKeyHex = randomHex(32);
	await sql(`
		INSERT INTO account_public_keys (id, account_id, public_key, is_active, device_name)
		VALUES (decode('${keyIdHex}', 'hex'), decode('${accountHex}', 'hex'), decode('${pubKeyHex}', 'hex'), TRUE, '${deviceName.replace(/'/g, "''")}')
	`);
	return keyIdHex;
}

/** Hard-delete a seeded key by id (cleanup, whether disabled by the test or not). */
async function deleteKey(keyIdHex: string): Promise<void> {
	await sql(`DELETE FROM account_public_keys WHERE id = decode('${keyIdHex}', 'hex')`);
}

/** is_active flag ('t'/'f') for a key id. */
async function keyIsActive(keyIdHex: string): Promise<string> {
	return (await sql(`SELECT is_active FROM account_public_keys WHERE id = decode('${keyIdHex}', 'hex')`)).trim();
}

test.describe('Device remove (inline two-step confirm)', () => {
	test('first Remove click reveals an inline confirm; second click disables the device', async ({ page, testAccount }) => {
		const accountHex = await accountIdHex(testAccount.username);
		const deviceName = `E2E-DEL-${Date.now()}`;
		const keyIdHex = await seedDevice(accountHex, deviceName);
		try {
			// A native dialog must never appear — fail loudly if it does.
			assertNoNativeDialog(page);

			await page.goto('/dashboard/account/security');
			const row = page.locator('div.flex.items-center.justify-between.p-3', { hasText: deviceName });
			await expect(row).toBeVisible({ timeout: 10000 });
			await expect(row.getByText('Active')).toBeVisible();

			// First click reveals inline Confirm/Cancel (no native dialog).
			await row.getByRole('button', { name: 'Remove' }).click();
			const confirmBtn = row.getByRole('button', { name: 'Confirm' });
			await expect(confirmBtn).toBeVisible();
			await expect(row.getByRole('button', { name: 'Cancel' })).toBeVisible();

			// Second click disables the device server-side.
			await confirmBtn.click();

			// The device flips Active → Disabled, and the DB row is disabled.
			await expect(row.getByText('Disabled')).toBeVisible({ timeout: 10000 });
			await expect(row.getByText('Active')).toHaveCount(0);
			expect(await keyIsActive(keyIdHex)).toBe('f');
		} finally {
			await deleteKey(keyIdHex);
		}
	});

	test('Cancel aborts the removal and keeps the device active', async ({ page, testAccount }) => {
		const accountHex = await accountIdHex(testAccount.username);
		const deviceName = `E2E-KEEP-${Date.now()}`;
		const keyIdHex = await seedDevice(accountHex, deviceName);
		try {
			assertNoNativeDialog(page);

			await page.goto('/dashboard/account/security');
			const row = page.locator('div.flex.items-center.justify-between.p-3', { hasText: deviceName });
			await expect(row).toBeVisible({ timeout: 10000 });

			await row.getByRole('button', { name: 'Remove' }).click();
			await row.getByRole('button', { name: 'Cancel' }).click();

			// Confirm is gone, the device stays Active, and the DB row is unchanged.
			await expect(row.getByRole('button', { name: 'Confirm' })).toHaveCount(0);
			await expect(row.getByText('Active')).toBeVisible();
			expect(await keyIsActive(keyIdHex)).toBe('t');
		} finally {
			await deleteKey(keyIdHex);
		}
	});
});
