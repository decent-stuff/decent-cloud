import { test, expect } from './fixtures/test-account';
import {
	pubkeyHexFromSeed,
	randomHex,
	nowNs,
	sql,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for the reseller relationship delete flow (two-step inline
 * confirm).
 *
 * The delete previously used a native confirm() dialog — which both blocks
 * headless e2e (Playwright auto-dismisses it, so the delete never fires) and
 * is a poor mobile UX. It now uses the inline two-step pattern (first Delete
 * click reveals an inline Confirm button, second click deletes), mirroring the
 * offerings delete pattern (commit 1077dd33).
 *
 * Seeding model: reseller_relationships has no FKs on its pubkey columns, so a
 * row with reseller_pubkey = testAccount pubkey renders directly. The signed
 * GET /reseller/relationships and DELETE /reseller/relationships/:ext both
 * filter by the caller's reseller_pubkey. No first-party API is mocked.
 *
 * Serial mode: all testAccount users share one pubkey; these tests mutate rows
 * keyed on it.
 */
test.describe.configure({ mode: 'serial' });

async function seedRelationship(resellerPubkey: string): Promise<string> {
	const extPubkey = randomHex(32);
	await sql(`
		INSERT INTO reseller_relationships (reseller_pubkey, external_provider_pubkey, commission_percent, status, created_at_ns)
		VALUES (decode('${resellerPubkey}', 'hex'), decode('${extPubkey}', 'hex'), 10, 'active', ${nowNs()})
		ON CONFLICT (reseller_pubkey, external_provider_pubkey) DO NOTHING
	`);
	return extPubkey;
}

async function deleteRelationship(resellerPubkey: string, extPubkey: string): Promise<void> {
	await sql(`
		DELETE FROM reseller_relationships
		WHERE reseller_pubkey = decode('${resellerPubkey}', 'hex')
		  AND external_provider_pubkey = decode('${extPubkey}', 'hex')
	`);
}

test.describe('Reseller relationship delete (inline two-step confirm)', () => {
	test('first Delete click reveals an inline confirm; second click deletes the relationship', async ({ page, testAccount }) => {
		const resellerPubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const extPubkey = await seedRelationship(resellerPubkey);
		try {
			await page.goto('/dashboard/provider/reseller');

			// The relationship card renders (header falls back to the ext pubkey prefix).
			const card = page.locator('div.bg-surface-elevated', { hasText: extPubkey.slice(0, 8) }).first();
			await expect(card.getByRole('button', { name: 'Delete' })).toBeVisible({ timeout: 10000 });

			// First click: no native dialog — it reveals an inline Confirm + Cancel.
			await card.getByRole('button', { name: 'Delete' }).click();
			const confirmBtn = card.getByRole('button', { name: 'Confirm' });
			await expect(confirmBtn).toBeVisible();
			await expect(card.getByRole('button', { name: 'Cancel' })).toBeVisible();

			// Second click: performs the deletion. Wait for the signed DELETE.
			const deleteReq = page.waitForResponse(
				(resp) =>
					resp.request().method() === 'DELETE' &&
					resp.url().includes(`/api/v1/reseller/relationships/${extPubkey}`),
				{ timeout: 15000 },
			);
			await confirmBtn.click();
			await deleteReq;

			// Success message surfaces and the card is gone after the list refetches.
			await expect(page.getByText('Reseller relationship deleted')).toBeVisible({ timeout: 10000 });
			await expect(page.locator('div.bg-surface-elevated', { hasText: extPubkey.slice(0, 8) })).toHaveCount(0);
		} finally {
			await deleteRelationship(resellerPubkey, extPubkey);
		}
	});

	test('Cancel aborts the deletion and keeps the relationship', async ({ page, testAccount }) => {
		const resellerPubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const extPubkey = await seedRelationship(resellerPubkey);
		try {
			await page.goto('/dashboard/provider/reseller');
			const card = page.locator('div.bg-surface-elevated', { hasText: extPubkey.slice(0, 8) }).first();
			await expect(card.getByRole('button', { name: 'Delete' })).toBeVisible({ timeout: 10000 });

			await card.getByRole('button', { name: 'Delete' }).click();
			await card.getByRole('button', { name: 'Cancel' }).click();

			// Confirm/Cancel disappear; the plain Delete button returns.
			await expect(card.getByRole('button', { name: 'Confirm' })).toHaveCount(0);
			await expect(card.getByRole('button', { name: 'Delete' })).toBeVisible();
		} finally {
			await deleteRelationship(resellerPubkey, extPubkey);
		}
	});
});
