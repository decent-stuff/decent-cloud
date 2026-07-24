import { test, expect } from './fixtures/test-account';
import { sql } from './fixtures/seed-helpers';

/**
 * E2E coverage for the external-key delete flow (inline two-step confirm).
 *
 * ExternalKeysEditor previously used a native confirm() dialog on row delete —
 * which both blocks headless e2e (Playwright auto-dismisses it, so the delete
 * never fires) and is a poor mobile UX. It now uses the inline two-step
 * pattern (first click reveals an inline Confirm button, second click
 * deletes), mirroring the offerings delete (commit 1077dd33).
 *
 * Serial mode: the spec seeds + deletes account_external_keys rows for the
 * shared testAccount pubkey, so tests must not run in parallel.
 */
test.describe.configure({ mode: 'serial' });

/** Resolve the bytea account id (hex) for a username. */
async function accountIdHex(username: string): Promise<string> {
	const row = await sql(
		`SELECT encode(id, 'hex') FROM accounts WHERE username = '${username.replace(/'/g, "''")}'`,
	);
	const hex = row.split('\n').map((l) => l.trim()).find((l) => /^[0-9a-f]+$/.test(l));
	if (!hex) throw new Error(`no account id for username ${username}`);
	return hex;
}

/** Seed one external key; returns its numeric id. */
async function seedKey(accountHex: string, label: string): Promise<string> {
	const out = await sql(`
		INSERT INTO account_external_keys (account_id, key_type, key_data, label)
		VALUES (decode('${accountHex}', 'hex'), 'ssh-ed25519', 'ssh-ed25519 AAAAe2efake', '${label.replace(/'/g, "''")}')
		RETURNING id
	`);
	const id = out.split('\n').map((l) => l.trim()).find((l) => /^\d+$/.test(l));
	if (!id) throw new Error(`seedKey did not RETURN an id; got: ${out}`);
	return id;
}

/** Remove any external keys left for the account (cleanup). */
async function cleanupKeys(accountHex: string): Promise<void> {
	await sql(`DELETE FROM account_external_keys WHERE account_id = decode('${accountHex}', 'hex')`);
}

test.describe('External key delete (inline two-step confirm)', () => {
	test('first Delete click reveals an inline confirm; second click deletes the key', async ({ page, testAccount }) => {
		const accountHex = await accountIdHex(testAccount.username);
		const label = `E2E-DEL-${Date.now()}`;
		const id = await seedKey(accountHex, label);
		try {
			// A native dialog must never appear — fail loudly if it does.
			page.on('dialog', (d) => expect(d.type(), 'native dialog must not fire').toBe('never'));

			await page.goto('/dashboard/account/profile');
			// External-key rows use the label badge text; scope to the key card
			// (the row div lacks the .flex class that contact/social rows have).
			const row = page.locator('div.p-3.bg-surface-elevated', { hasText: label });
			await expect(row).toBeVisible({ timeout: 10000 });

			// First click reveals inline Confirm/Cancel (no native dialog).
			await row.getByRole('button', { name: 'Delete' }).click();
			const confirmBtn = row.getByRole('button', { name: 'Confirm' });
			await expect(confirmBtn).toBeVisible();
			await expect(row.getByRole('button', { name: 'Cancel' })).toBeVisible();

			// Second click performs the deletion.
			await confirmBtn.click();

			// The row disappears after the list refetches.
			await expect(page.locator('div.p-3.bg-surface-elevated', { hasText: label })).toHaveCount(0, { timeout: 10000 });
			// And the server-side row is gone.
			const remaining = await sql(`SELECT count(*) FROM account_external_keys WHERE id = ${id}`);
			expect(remaining.trim()).toBe('0');
		} finally {
			await cleanupKeys(accountHex);
		}
	});

	test('Cancel aborts the deletion and keeps the key', async ({ page, testAccount }) => {
		const accountHex = await accountIdHex(testAccount.username);
		const label = `E2E-KEEP-${Date.now()}`;
		const id = await seedKey(accountHex, label);
		try {
			page.on('dialog', (d) => expect(d.type(), 'native dialog must not fire').toBe('never'));

			await page.goto('/dashboard/account/profile');
			const row = page.locator('div.p-3.bg-surface-elevated', { hasText: label });
			await expect(row).toBeVisible({ timeout: 10000 });

			await row.getByRole('button', { name: 'Delete' }).click();
			await row.getByRole('button', { name: 'Cancel' }).click();

			// Confirm is gone, the row remains, and the server row still exists.
			await expect(row.getByRole('button', { name: 'Confirm' })).toHaveCount(0);
			await expect(row).toBeVisible();
			const remaining = await sql(`SELECT count(*) FROM account_external_keys WHERE id = ${id}`);
			expect(remaining.trim()).toBe('1');
		} finally {
			await cleanupKeys(accountHex);
		}
	});
});
