import { test, expect } from './fixtures/test-account';

/**
 * E2E coverage for /dashboard/account/profile editing — the gap left by
 * profile-page.spec.ts.
 *
 * profile-page.spec.ts already pins:
 *   - the editor renders (display-name, bio, Save Profile)
 *   - display-name + bio edits persist across save/reload
 *
 * The remaining uncovered behavior is the **Avatar URL** field, which the
 * existing test never touches. This spec pins that editing the avatar URL and
 * saving persists the value across a reload — the same write/read contract the
 * existing test pins for the name + bio fields, extended to the third editor
 * field.
 *
 * Note: there is no "website" field on this editor (SocialsEditor handles
 * external links separately) and no "empty bio validation" (bio is optional),
 * so those task-listed items have no implementation to test.
 */
test.describe('/dashboard/account/profile — avatar URL field', () => {
	test('avatar URL edit persists after save and reload', async ({ page }) => {
		await page.goto('/dashboard/account/profile');

		// Wait for the editor to hydrate and load any existing profile.
		const avatarInput = page.locator('#avatar-url');
		await expect(avatarInput).toBeVisible({ timeout: 10000 });

		const avatarUrl = `https://example.com/avatar-${Date.now()}.png`;

		// Fill the avatar URL field (the uncovered field) and save.
		await avatarInput.fill(avatarUrl);
		await page.getByRole('button', { name: 'Save Profile' }).click();

		// The success toast confirms the PUT landed.
		await expect(page.getByText('Profile updated successfully')).toBeVisible({ timeout: 10000 });

		// Reload and verify the avatar URL persisted server-side.
		await page.reload();
		await expect(page.locator('#avatar-url')).toHaveValue(avatarUrl, { timeout: 10000 });
	});
});
