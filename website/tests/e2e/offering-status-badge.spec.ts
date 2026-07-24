import { test, expect } from './fixtures/test-account';
import { seedRentableOffering, deleteOfferingsByProvider } from './fixtures/seed-helpers';

/**
 * E2E coverage for the OfferingStatusBadge component's keyboard a11y.
 *
 * Audit #15: the "more details" tooltip opened only on mouseenter/mouseleave
 * (+ click toggle), so keyboard-only users navigating with Tab could not
 * reach the tooltip content (Trust score, Subscription, Has setup recipe,
 * Has warnings). The fix adds onfocus/onblur handlers, aria-describedby
 * pointing at the tooltip, and Escape-to-close.
 *
 * The badge + its "More details" button render only when an offering carries
 * trust/subscription/recipe/warnings metadata (OfferingStatusBadge.svelte
 * `hasTooltip`). This spec seeds its OWN offering WITH a setup recipe so the
 * badge is guaranteed to appear — it must NOT rely on ambient demo/seed data,
 * which is wiped whenever the dev DB is reset.
 */

test.describe.configure({ mode: 'serial' });

test.describe('OfferingStatusBadge keyboard a11y', () => {
	let providerPubkeyHex: string;
	let offeringName: string;

	test.beforeAll(async () => {
		// self_provisioned → always online (no agent pool needed); a recipe makes
		// hasTooltip true so the "More details" button renders.
		const seeded = await seedRentableOffering({
			name: `E2E Badge Recipe ${Date.now()}`,
			postProvisionScript: '#!/bin/bash\necho setup-complete'
		});
		providerPubkeyHex = seeded.providerPubkeyHex;
		offeringName = seeded.offeringName;
	});

	test.afterAll(async () => {
		if (providerPubkeyHex) await deleteOfferingsByProvider(providerPubkeyHex);
	});

	test('tooltip becomes visible when the badge button receives focus (#15)', async ({ page }) => {
		await page.goto('/dashboard/marketplace?offline=1');
		await expect(page.locator('h1:has-text("Marketplace")')).toBeVisible();

		// Scope to the seeded row so we test the badge that is guaranteed to exist.
		const row = page.locator('tbody tr', { hasText: offeringName }).first();
		await expect(row).toBeVisible({ timeout: 15000 });

		// The "More details" button is only rendered when hasTooltip is true
		// (our seeded offering has a setup recipe).
		const badgeButton = row.getByRole('button', { name: 'More details' });
		await expect(badgeButton).toBeVisible();
		await badgeButton.focus();

		// The tooltip must become visible. It carries role="tooltip".
		const tooltip = page.getByRole('tooltip').first();
		await expect(tooltip).toBeVisible({ timeout: 2000 });

		// The button must expose an aria-describedby pointing at the tooltip so
		// screen readers announce the same content keyboard users now see.
		const describedBy = await badgeButton.getAttribute('aria-describedby');
		expect(describedBy, 'badge button must have aria-describedby').toBeTruthy();
		const tooltipId = await tooltip.getAttribute('id');
		expect(describedBy).toContain(tooltipId);
	});

	test('Escape closes the tooltip while the badge retains focus (#15)', async ({ page }) => {
		await page.goto('/dashboard/marketplace?offline=1');
		const row = page.locator('tbody tr', { hasText: offeringName }).first();
		await expect(row).toBeVisible({ timeout: 15000 });

		const badgeButton = row.getByRole('button', { name: 'More details' });
		await badgeButton.focus();
		const tooltip = page.getByRole('tooltip').first();
		await expect(tooltip).toBeVisible({ timeout: 2000 });

		// Press Escape — the tooltip should close.
		await page.keyboard.press('Escape');
		await expect(tooltip).toHaveCount(0);
	});
});
