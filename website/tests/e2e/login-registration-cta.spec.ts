import { test, expect } from '@playwright/test';
import { clickAndRetry, revealSeedPhraseOptions } from './fixtures/auth-helpers';

test.describe('Login page registration CTA', () => {
	// F8: the login page exposes exactly one primary create-account path. When the
	// seed-phrase chooser is visible (OAuth off) it is the "Generate New" card;
	// when the chooser is hidden (OAuth on) it is the "Create an account" link.
	// The redundant duplicate is suppressed, so the spec asserts a discoverable
	// create path exists and reaches seed backup regardless of the OAuth config.
	test('shows a discoverable create-account path on the login page', async ({ page }) => {
		await page.goto('/login');

		// One of the two create entry points must be present (mutually exclusive
		// depending on OAuth capability), never both at once.
		const createAccountLink = page.getByRole('button', { name: 'Create an account' });
		const generateNewCard = page.getByRole('button', { name: 'Generate New' });
		await expect(createAccountLink.or(generateNewCard).first()).toBeVisible();
		// When the chooser is visible the redundant link is suppressed.
		if (await generateNewCard.isVisible()) {
			await expect(createAccountLink).toHaveCount(0);
		}
	});

	test('the create-account path reaches seed backup (generate mode)', async ({ page }) => {
		await page.goto('/login');

		const heading = page.getByRole('heading', { name: 'Backup Your Seed Phrase' });
		const createAccountLink = page.getByRole('button', { name: 'Create an account' });
		const generateNewCard = page.getByRole('button', { name: 'Generate New' });

		// Click whichever create path is present. Both controls' onclick handlers
		// bind on hydration, so retry until the "Backup Your Seed Phrase" heading
		// appears — but pace the retries (250ms) so the "Generate New" card, which
		// fires seed generation on click, isn't double-triggered into a crash.
		const ready = () => heading.isVisible().catch(() => false);
		for (let attempt = 0; attempt < 12 && !(await ready()); attempt++) {
			const target = (await generateNewCard.isVisible().catch(() => false))
				? generateNewCard
				: createAccountLink;
			await target.click({ timeout: 1000 }).catch(() => {});
			await page.waitForTimeout(250);
		}

		// Should land on the "Backup Your Seed Phrase" step, not the choose/import screen
		await expect(heading).toBeVisible({ timeout: 10000 });
		// 12 seed-word boxes should be present
		await expect(page.locator('.grid.grid-cols-3 > div')).toHaveCount(12);
	});

	test('Sign in with seed phrase shows the choose (import/generate) screen', async ({ page }) => {
		await page.goto('/login');
		await revealSeedPhraseOptions(page);

		// Existing users land on the mode-chooser
		await expect(page.getByRole('heading', { name: 'Seed Phrase' })).toBeVisible();
		await expect(page.getByRole('button', { name: 'Import Existing' })).toBeVisible();
		await expect(page.getByRole('button', { name: 'Generate New' })).toBeVisible();
	});
});
