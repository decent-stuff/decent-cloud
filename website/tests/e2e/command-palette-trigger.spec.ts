import { test, expect, waitForAuthReady } from './fixtures/test-account';

/**
 * The CommandPalette (Cmd/Ctrl+K) is a powerful quick-nav, but on desktop it
 * had NO visible trigger — only the keyboard shortcut opened it, and nothing
 * hinted the shortcut existed. Mobile had a search button in the header; the
 * desktop sidebar did not. These specs lock in a visible, clickable trigger in
 * the desktop sidebar so the feature is discoverable without guessing keys.
 */
test.describe('command palette desktop trigger', () => {
	// The page fixture no longer pre-navigates (see fixtures/test-account.ts),
	// so land on /dashboard explicitly. The palette trigger lives in the
	// dashboard sidebar, which only renders once auth has settled.
	test.beforeEach(async ({ page }) => {
		await page.goto('/dashboard');
		await waitForAuthReady(page);
	});

	test('@smoke sidebar shows a clickable command-palette trigger on desktop', async ({ page }) => {
		// The trigger is an always-visible button in the dashboard sidebar.
		const trigger = page.getByRole('button', { name: /search|command/i }).first();
		await expect(trigger).toBeVisible();

		// It advertises the keyboard shortcut so users learn Cmd/Ctrl+K exists.
		await expect(page.locator('aside')).toContainText('K');

		// Clicking it opens the palette dialog.
		await trigger.click();
		await expect(page.getByRole('dialog', { name: 'Command palette' })).toBeVisible();
		await expect(page.getByRole('option', { name: /Marketplace/i })).toBeVisible();
	});

	test('Escape closes the palette opened from the sidebar trigger', async ({ page }) => {
		const trigger = page.getByRole('button', { name: /search|command/i }).first();
		await trigger.click();
		const dialog = page.getByRole('dialog', { name: 'Command palette' });
		await expect(dialog).toBeVisible();

		await page.keyboard.press('Escape');
		await expect(dialog).not.toBeVisible();
	});

	test('arrow keys move highlight, Enter selects, and Escape closes (window handler)', async ({ page }) => {
		// Regression: the modal's keydown stopPropagation used to swallow every
		// key before the <svelte:window onkeydown> handler ran, so the palette's
		// own keyboard nav (advertised in its footer) did nothing. Open via the
		// keyboard shortcut to exercise the same window-handler path.
		await page.keyboard.press('Control+k');
		const dialog = page.getByRole('dialog', { name: 'Command palette' });
		await expect(dialog).toBeVisible();

		const options = page.getByRole('option');
		await expect(options.nth(0)).toHaveAttribute('aria-selected', 'true');

		// ArrowDown moves highlight to the second option.
		await page.keyboard.press('ArrowDown');
		await expect(options.nth(0)).toHaveAttribute('aria-selected', 'false');
		await expect(options.nth(1)).toHaveAttribute('aria-selected', 'true');

		// Enter selects the highlighted option and navigates.
		await page.keyboard.press('Enter');
		await expect(dialog).not.toBeVisible();
	});

	test('@smoke authenticated palette lists provider actions and navigates on select', async ({ page }) => {
		// Authenticated users should see provider-facing quick actions in the
		// palette, not just generic nav. Open via the keyboard shortcut.
		await page.keyboard.press('Control+k');
		const dialog = page.getByRole('dialog', { name: 'Command palette' });
		await expect(dialog).toBeVisible();

		// A provider action must be present.
		const createOffering = dialog.getByRole('option', { name: /Create Offering/i });
		await expect(createOffering).toBeVisible();

		// Selecting it navigates to the create-offering route.
		await createOffering.click();
		await expect(dialog).not.toBeVisible();
		await expect(page).toHaveURL(/\/dashboard\/offerings\/create/);
	});
});
