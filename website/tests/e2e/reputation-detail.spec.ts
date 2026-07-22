import { test, expect } from './fixtures/test-account';
import { test as anonymousTest } from '@playwright/test';
import { pubkeyHexFromSeed } from './fixtures/seed-helpers';

/**
 * E2E coverage for the /dashboard/reputation/[identifier] route (GAP route).
 *
 * The page resolves either a username or a 64-char pubkey hex, then renders a
 * reputation profile OR a "No Account Data" empty state. uxaudit is a real
 * seeded account with zero activity, so it exercises the populated-but-empty
 * branch (account resolves, metrics render as zero). A bogus identifier
 * exercises the not-found branch.
 *
 * Note on "View full profile": that dashboard link is provider-gated
 * (`userRole === 'provider'`), so it does not render for the non-provider
 * fixture account. The authenticated case below navigates directly to the
 * user's own reputation URL instead — same destination, deterministic path.
 */
anonymousTest.describe('/dashboard/reputation/[identifier] (anonymous)', () => {
	anonymousTest('renders the reputation profile for a known account', async ({ page }) => {
		await page.goto('/dashboard/reputation/uxaudit');

		// Scope to <main> so the assertions ignore the sidebar nav (which also
		// contains a "Reputation" link and would otherwise make text matches
		// ambiguous).
		const main = page.locator('main');

		// The pubkey must be resolved and rendered — this proves the identifier
		// lookup succeeded (the not-found branch would hide the pubkey block).
		await expect(main.getByText('Public Key')).toBeVisible({ timeout: 10000 });
		await expect(
			main.getByText('e96b1c6b8769fdb0b34fbecfdf85c33b053cecad9517e1ab88cba614335775c1'),
		).toBeVisible();

		// The overview metrics grid must render. "Balance" as a metric label
		// exists only in the populated profile branch, not the not-found
		// branch — so its visibility pins which branch rendered.
		await expect(main.getByText('Balance', { exact: true })).toBeVisible();
		await expect(main.getByText('Contracts', { exact: true })).toBeVisible();
	});

	anonymousTest('renders the "No Account Data" state for an unknown identifier', async ({ page }) => {
		await page.goto('/dashboard/reputation/nonexistent-user-zzz');

		// The not-found branch shows this specific heading + guidance.
		await expect(page.getByRole('heading', { name: 'No Account Data' })).toBeVisible({
			timeout: 10000,
		});
		await expect(page.getByText(/is not registered in the system/)).toBeVisible();

		// The "Back to Marketplace" recovery link must be present.
		await expect(
			page.getByRole('link', { name: /Back to Marketplace/i }),
		).toBeVisible();
	});
});

test.describe('/dashboard/reputation/[identifier] (authenticated)', () => {
	test('authenticated user can view their own reputation profile', async ({ page, testAccount }) => {
		// Derive the fixture account's pubkey and view its reputation page.
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		await page.goto(`/dashboard/reputation/${pubkey}`);

		// Scope to <main> so assertions ignore the sidebar nav links.
		const main = page.locator('main');

		// The profile must render for the viewer's own account. The pubkey
		// block visibility proves the identifier resolved to the account.
		await expect(main.getByText('Public Key')).toBeVisible({ timeout: 10000 });
		await expect(main.getByText(pubkey, { exact: true })).toBeVisible();

		// The overview metrics grid must render (populated-but-empty branch).
		await expect(main.getByText('Balance', { exact: true })).toBeVisible();
	});
});
