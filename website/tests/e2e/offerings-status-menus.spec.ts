import { test, expect } from './fixtures/test-account';
import { pubkeyHexFromSeed, sql, nowNs } from './fixtures/seed-helpers';

/**
 * E2E coverage for the offering visibility & stock-status menus (#437).
 *
 * The previous UI exposed two `cycle` buttons that rotated through states
 * with no preview of the next value and only a `title=` tooltip that is
 * invisible on touch. These specs pin the replacement: each trigger opens a
 * menu that lists every option with a one-line description, the current
 * option is marked, and selecting an option persists via the signed PUT path.
 */

interface OfferingSeed {
	/** Stable per-test offering_id. */
	offeringId: string;
	/** Initial visibility. Default 'public'. */
	visibility?: string;
	/** Initial stock_status. Default 'in_stock'. */
	stockStatus?: string;
	/** Optional name override. Default 'Menu Test Offering'. */
	name?: string;
}

async function seedOffering(pubkeyHex: string, seed: OfferingSeed): Promise<string> {
	const visibility = seed.visibility ?? 'public';
	const stockStatus = seed.stockStatus ?? 'in_stock';
	const name = (seed.name ?? 'Menu Test Offering').replace(/'/g, "''");
	const createdAt = nowNs().toString();
	// Only the NOT NULL columns of provider_offerings (verified via \d).
	// Bytes use decode(...,'hex'); signed PUTs are accepted because the row's
	// pubkey matches the test-account identity. Returns the numeric id (PK)
	// which is what the page sets on the per-card `data-offering-id` attribute.
	const result = await sql(`
		INSERT INTO provider_offerings (
			pubkey, offering_id, offer_name, currency, monthly_price,
			visibility, product_type, billing_interval, stock_status,
			datacenter_country, datacenter_city, created_at_ns
		) VALUES (
			decode('${pubkeyHex}', 'hex'),
			'${seed.offeringId}',
			'${name}',
			'ICP', 25.0,
			'${visibility}', 'compute', 'monthly', '${stockStatus}',
			'US', 'New York', ${createdAt}
		)
		RETURNING id
	`);
	// psql emits "id\nINSERT 0 1" — take the first non-empty line.
	const numericId = result.split('\n').map((l) => l.trim()).find((l) => /^\d+$/.test(l));
	if (!numericId) throw new Error(`seedOffering did not RETURN a numeric id; got: ${result}`);
	return numericId;
}

async function deleteOfferingsForPubkey(pubkeyHex: string): Promise<void> {
	// CASCADE handles sla_targets, sli_reports, visibility_allowlist;
	// cloud_resources.offering_id is SET NULL.
	await sql(`DELETE FROM provider_offerings WHERE pubkey = decode('${pubkeyHex}', 'hex')`);
}

test.describe('Offering status menus (#437)', () => {
	test('visibility menu lists all states with descriptions and persists selection', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const offeringKey = 'menu-test-vis-001';
		try {
			const cardId = await seedOffering(pubkey, { offeringId: offeringKey, visibility: 'public' });
			await page.goto('/dashboard/offerings');
			// Card hook: data-offering-id set on the per-card wrapper (numeric PK).
			const card = page.locator(`[data-offering-id="${cardId}"]`);
			await expect(card).toBeVisible({ timeout: 10000 });

			// Trigger exposes its current state via aria-label, NOT title.
			const trigger = card.getByRole('button', { name: /^Visibility:/ });
			await expect(trigger).toBeVisible();
			await expect(trigger).toHaveText('Public');
			expect(await trigger.getAttribute('title')).toBeNull();

			// Open the menu.
			await trigger.click();
			const menu = card.locator('[data-status-menu="visibility"]');
			await expect(menu).toBeVisible();

			// All three options present with their descriptions.
			await expect(menu.getByText('Public (marketplace)')).toBeVisible();
			await expect(menu.getByText('Visible to everyone in the marketplace')).toBeVisible();
			await expect(menu.getByText('Shared (allowlist only)')).toBeVisible();
			await expect(menu.getByText('Visible only to customers on your allowlist')).toBeVisible();
			await expect(menu.getByText('Private (owner only)')).toBeVisible();
			await expect(menu.getByText('Hidden from the marketplace')).toBeVisible();

			// The current option is marked.
			await expect(menu.locator('[aria-checked="true"][data-value="public"]')).toBeVisible();

			// Selecting Shared persists via the signed PUT path. (We avoid
			// 'private' here because the provider-offerings endpoint filters
			// private rows even for the owner — that dashboard-data-fetch
			// architecture is a separate concern from #437's menu UX.)
			await menu.locator('[data-value="shared"]').click();
			await expect(menu).not.toBeVisible();
			// Trigger reflects the new state.
			await expect(trigger).toHaveText('Shared');
		} finally {
			await deleteOfferingsForPubkey(pubkey);
		}
	});

	test('stock menu lists all states with descriptions and persists selection', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const offeringKey = 'menu-test-stock-001';
		try {
			const cardId = await seedOffering(pubkey, { offeringId: offeringKey, stockStatus: 'in_stock' });
			await page.goto('/dashboard/offerings');
			const card = page.locator(`[data-offering-id="${cardId}"]`);
			await expect(card).toBeVisible({ timeout: 10000 });

			const trigger = card.getByRole('button', { name: /^Stock:/ });
			await expect(trigger).toBeVisible();
			await expect(trigger).toHaveText('In Stock');
			expect(await trigger.getAttribute('title')).toBeNull();

			await trigger.click();
			const menu = card.locator('[data-status-menu="stock"]');
			await expect(menu).toBeVisible();

			await expect(menu.getByText('In Stock')).toBeVisible();
			await expect(menu.getByText('Available for new orders')).toBeVisible();
			await expect(menu.getByText('Out of Stock')).toBeVisible();
			await expect(menu.getByText('Listed, but unavailable right now')).toBeVisible();
			await expect(menu.getByText('Discontinued')).toBeVisible();
			await expect(menu.getByText('Permanently unavailable; hidden from marketplace')).toBeVisible();

			await expect(menu.locator('[aria-checked="true"][data-value="in_stock"]')).toBeVisible();

			await menu.locator('[data-value="discontinued"]').click();
			await expect(menu).not.toBeVisible();
			await expect(trigger).toHaveText('Discontinued');
		} finally {
			await deleteOfferingsForPubkey(pubkey);
		}
	});

	test('only one status menu is open per card at a time', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const offeringKey = 'menu-test-exclusive-001';
		try {
			const cardId = await seedOffering(pubkey, { offeringId: offeringKey });
			await page.goto('/dashboard/offerings');
			const card = page.locator(`[data-offering-id="${cardId}"]`);
			await expect(card).toBeVisible({ timeout: 10000 });

			const visTrigger = card.getByRole('button', { name: /^Visibility:/ });
			const stockTrigger = card.getByRole('button', { name: /^Stock:/ });
			const visMenu = card.locator('[data-status-menu="visibility"]');
			const stockMenu = card.locator('[data-status-menu="stock"]');

			await visTrigger.click();
			await expect(visMenu).toBeVisible();
			expect(await stockMenu.isVisible()).toBe(false);

			// Opening the stock menu closes the visibility menu. Use keyboard
			// (focus + Enter) rather than a mouse click because on narrow cards
			// the badges flex-wrap onto separate rows, and an open visibility
			// panel dropping downward can visually overlap the wrapped stock
			// trigger. Keyboard activation sidesteps that layout collision
			// while still exercising the same mutual-exclusion contract.
			await stockTrigger.focus();
			await stockTrigger.press('Enter');
			await expect(stockMenu).toBeVisible();
			await expect(visMenu).not.toBeVisible();
		} finally {
			await deleteOfferingsForPubkey(pubkey);
		}
	});

	test('trigger accessible name does not depend on the legacy title tooltip', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const offeringKey = 'menu-test-a11y-001';
		try {
			const cardId = await seedOffering(pubkey, { offeringId: offeringKey });
			await page.goto('/dashboard/offerings');
			const card = page.locator(`[data-offering-id="${cardId}"]`);
			await expect(card).toBeVisible({ timeout: 10000 });

			// Both triggers expose aria-labels beginning with their domain prefix,
			// so screen-reader users hear "Visibility: Public" / "Stock: In Stock"
			// without relying on a hover-only `title`.
			await expect(card.getByRole('button', { name: /^Visibility: Public$/ })).toBeVisible();
			await expect(card.getByRole('button', { name: /^Stock: In Stock$/ })).toBeVisible();

			// No trigger in the action row may use the legacy "Click to cycle" title.
			const cycleTitles = await card
				.locator('[title*="Click to cycle"]')
				.count();
			expect(cycleTitles).toBe(0);
		} finally {
			await deleteOfferingsForPubkey(pubkey);
		}
	});
});
