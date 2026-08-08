import { test, expect } from '@playwright/test';
import { API_BASE_URL } from './fixtures/api-base';
import { revealSeedPhraseOptions } from './fixtures/auth-helpers';

/**
 * UX-003: seed-phrase auth has zero education for non-crypto buyers. These
 * specs guard three copy changes: (1) an inline "what is a seed phrase?" note
 * on the auth chooser, (2) a prominent permanent-loss warning on the backup
 * step, and (3) a seed-phrase-specific recovery link.
 *
 * They stay green on any stack config: the warm stack returns
 * `google_oauth=false` (no Google creds in dev) so the seed-phrase surface is
 * the default and the education renders without an extra click; when OAuth IS
 * configured, the seed path is expanded first. The capabilities endpoint is
 * first-party and is NEVER mocked — the spec reads the real value and adapts.
 */

test.describe('Seed-phrase education (UX-003)', () => {
	test('@smoke login chooser shows inline seed-phrase education', async ({ page }) => {
		await page.goto('/login');

		const capsRes = await page.request.get(`${API_BASE_URL}/api/v1/auth/capabilities`);
		const { google_oauth: googleOAuthEnabled } = await capsRes.json();
		if (googleOAuthEnabled) {
			await revealSeedPhraseOptions(page);
		}

		await expect(
			page.getByText('A seed phrase is a recovery code', { exact: false }),
		).toBeVisible({ timeout: 10_000 });
	});

	test('@smoke seed backup step warns the seed cannot be recovered if lost', async ({ page }) => {
		await page.goto('/login');

		const capsRes = await page.request.get(`${API_BASE_URL}/api/v1/auth/capabilities`);
		const { google_oauth: googleOAuthEnabled } = await capsRes.json();
		if (googleOAuthEnabled) {
			await revealSeedPhraseOptions(page);
		}

		// Reach generate (backup) mode. "Generate New" is SSR'd but its onclick
		// binds on hydration; pace the retries so seed generation isn't
		// double-triggered (mirrors login-registration-cta.spec.ts).
		const generateNew = page.getByRole('button', { name: 'Generate New' });
		const heading = page.getByRole('heading', { name: 'Backup Your Seed Phrase' });
		for (let attempt = 0; attempt < 12 && !(await heading.isVisible().catch(() => false)); attempt++) {
			await generateNew.click({ timeout: 1000 }).catch(() => {});
			await page.waitForTimeout(250);
		}
		await expect(heading).toBeVisible({ timeout: 10_000 });

		// Permanent-loss warning — seed phrases have NO recovery path.
		await expect(page.getByText(/If you lose your seed phrase/i)).toBeVisible();
		await expect(page.getByText(/cannot be recovered/i)).toBeVisible();
	});

	test('@smoke recovery link on the login page uses seed-phrase-specific copy', async ({ page }) => {
		await page.goto('/login');

		const recoveryLink = page.getByRole('link', { name: /Lost your seed phrase.*Recover/i });
		await expect(recoveryLink).toBeVisible();
		await recoveryLink.click();
		await expect(page).toHaveURL('/recover');
	});
});
