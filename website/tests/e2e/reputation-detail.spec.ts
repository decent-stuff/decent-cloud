import { test, expect } from './fixtures/test-account';
import { test as anonymousTest } from '@playwright/test';
import {
	pubkeyHexFromSeed,
	seedAccountDirect,
	deleteAccountByUsername,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for the /dashboard/reputation/[identifier] route (GAP route).
 *
 * The page resolves either a username or a 64-char pubkey hex, then renders a
 * reputation profile OR a "No Account Data" empty state. The "known account"
 * case seeds its own account via DB-direct INSERT (self-contained, no reliance
 * on externally-seeded state) so it exercises the populated-but-empty branch
 * (account resolves, metrics render as zero). A bogus identifier exercises the
 * not-found branch.
 *
 * Note on "View full profile": that dashboard link is provider-gated
 * (`userRole === 'provider'`), so it does not render for the non-provider
 * fixture account. The authenticated case below navigates directly to the
 * user's own reputation URL instead — same destination, deterministic path.
 */
anonymousTest.describe('/dashboard/reputation/[identifier] (anonymous)', () => {
	anonymousTest('renders the reputation profile for a known account', async ({ page }) => {
		// Self-contained: seed a fresh account and derive its pubkey, so the
		// assertion never depends on externally-seeded state (which previously
		// broke when `uxaudit` was re-seeded with a different seed phrase).
		const { username, seedPhrase } = await seedAccountDirect();
		const pubkey = pubkeyHexFromSeed(seedPhrase);
		try {
			await page.goto(`/dashboard/reputation/${username}`);

			// Scope to <main> so the assertions ignore the sidebar nav (which also
			// contains a "Reputation" link and would otherwise make text matches
			// ambiguous).
			const main = page.locator('main');

			// The pubkey must be resolved and rendered — this proves the identifier
			// lookup succeeded (the not-found branch would hide the pubkey block).
			await expect(main.getByText('Public Key')).toBeVisible({ timeout: 10000 });
			await expect(main.getByText(pubkey, { exact: true })).toBeVisible();

			// The overview metrics grid must render. A contract-based metric label
			// ("Contracts") exists only in the populated profile branch, not the
			// not-found branch — so its visibility pins which branch rendered.
			// (Previously pinned on "Balance", removed with the dead ICP
			// token-transfers feature; "Contracts" is the real reputation signal.)
			await expect(main.getByText('Contracts', { exact: true })).toBeVisible();
			await expect(main.getByText('Reputation', { exact: true })).toBeVisible();
		} finally {
			await deleteAccountByUsername(username);
		}
	});

	anonymousTest('shows a neutral uptime badge when the provider has no health checks', async ({ page }) => {
		// A fresh account has never been monitored, so the health summary
		// returns totalChecks=0 with uptimePercent=0.0. Previously that
		// fell through the percentage ladder into the red "Poor" badge —
		// unfairly penalising every new/unmonitored provider on the report
		// renters use to evaluate trust.
		const { username, seedPhrase } = await seedAccountDirect();
		const pubkey = pubkeyHexFromSeed(seedPhrase);
		try {
			await page.goto(`/dashboard/reputation/${username}`);

			const main = page.locator('main');

			// The Provider Health card must render.
			await expect(main.getByText('Provider Health (Last 30 Days)')).toBeVisible({ timeout: 10000 });

			// The Uptime cell must show a NEUTRAL badge — never the red
			// "Poor" badge that the percentage ladder produced.
			await expect(main.getByText('No health checks yet')).toBeVisible();
			await expect(main.getByText('Poor')).toHaveCount(0);
		} finally {
			await deleteAccountByUsername(username);
		}
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
		await expect(main.getByText('Contracts', { exact: true })).toBeVisible();
	});
});
