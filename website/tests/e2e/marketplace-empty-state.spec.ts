import { test, expect } from './fixtures/test-account';
import {
	seedProviderRegistration,
	seedOffering,
	deleteOfferingsByProvider,
	sql,
	randomHex,
	nowNs,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for the marketplace default-hide empty state.
 *
 * Offline offerings (providers with no online agents) are hidden by default
 * (showOfflineOfferings=false). When every offering is hidden this way, the
 * marketplace renders an empty state with a one-click "Show N offerings" reveal
 * action that toggles showOfflineOfferings on — distinct from "Clear all
 * filters", which only appears when user filters narrow the results.
 *
 * This spec self-seeds an OFFLINE-but-revealable offering to reproduce that
 * empty state. The marketplace list (`search_offerings`) drops any offering
 * with neither a resolved agent pool nor a `self_provisioned` source, so a
 * plain no-pool offering never reaches the list to be hidden. To be list-
 * visible AND offline, the seeded offering's provider needs a region-matched
 * agent pool whose agents are all offline: `resolved_pool_id` is then set
 * (passes the list filter) while `provider_online` stays false (hidden by
 * default, surfaced by the reveal).
 *
 * PARALLEL-ISOLATION (#477): the test asserts an empty visible list
 * (`filteredOfferings.length === 0` → "No providers are online right now"),
 * which only holds
 * when NO online offerings exist in the current view. Under Playwright's
 * default parallel workers, sibling specs seed always-online
 * (`self_provisioned`) offerings into the SAME test DB (search-dsl seeds 6 in
 * beforeAll and keeps them for ~20s; compare-share / rentable-offering-fixture
 * do likewise). Those online rows leak into an unfiltered marketplace view and
 * flip the list non-empty → flake.
 *
 * The fix scopes the view to this suite's own provider via
 * `?provider=<pubkey>` — the SAME URL param the production "View all from
 * provider" link uses (`marketplace/[id]/+page.svelte`). `providerFilter` is
 * applied client-side in `userFiltered` and is transparent to the
 * reveal-empty-state branch (it is NOT in the "Clear all filters" active-filter
 * gate at `marketplace/+page.svelte`), so the reveal button still fires when
 * the only offering in view is offline. Other workers' offerings never share
 * this random pubkey, so they cannot perturb the list. This is option (c) from
 * the task brief ("disjoint provider identity so the empty-state query filters
 * to its own scope") and is strictly more robust than serializing against one
 * spec, because it immunizes against ALL concurrent online-offering seeders.
 */
test.describe('Marketplace default-hide empty state', () => {
	// Fresh random provider pubkey → uniquely owned by this seed, so tearing the
	// whole provider down in afterAll is safe under parallel workers.
	let providerPubkeyHex: string | undefined;
	let poolId: string | undefined;

	test.beforeAll(async () => {
		providerPubkeyHex = randomHex(32);
		poolId = `e2e-pool-${Date.now()}-${Math.floor(Math.random() * 1_000_000)}`;
		// provider_registrations is required (agent_pools.provider_pubkey FK);
		// idempotent so a shared pubkey would be safe too, but we use a random one.
		await seedProviderRegistration(providerPubkeyHex);
		// Region pool with NO online agents. seedOffering hardcodes
		// datacenter_country='US' → region 'na', so a pool in location 'na' is
		// what the marketplace query auto-matches the offering to.
		await sql(`
			INSERT INTO agent_pools (pool_id, provider_pubkey, name, location, provisioner_type, created_at_ns)
			VALUES ('${poolId}', decode('${providerPubkeyHex}', 'hex'), 'E2E NA Pool', 'na', 'manual', ${nowNs()})
		`);
		// Plain (non-self_provisioned) offering → resolved via the region pool
		// above, offline because the pool has no online agents.
		await seedOffering(providerPubkeyHex, { name: 'E2E Hidden Offline Offering' });
	});

	test.afterAll(async () => {
		if (providerPubkeyHex) {
			// Random pubkey → safe to remove everything for it (no other worker
			// shares it). Offerings first (FK), then the pool + registration.
			await deleteOfferingsByProvider(providerPubkeyHex);
			await sql(`
				DELETE FROM agent_pools WHERE pool_id = '${poolId}';
				DELETE FROM provider_registrations WHERE pubkey = decode('${providerPubkeyHex}', 'hex');
			`);
		}
	});

	test('offers a reveal action when all offerings are hidden by default', async ({ page }) => {
		// Scope to this suite's own provider so parallel workers' online
		// offerings (search-dsl's 6, etc.) cannot leak into the view and flip
		// it non-empty. See the PARALLEL-ISOLATION note at the top of this file.
		await page.goto(`/dashboard/marketplace?provider=${providerPubkeyHex}`);

		// Empty state: no visible offerings after the default-hide filters run.
		// All matching offerings are offline, so the headline explains that
		// rather than reading as a dead marketplace.
		await expect(page.getByText('No providers are online right now')).toBeVisible({ timeout: 10000 });
		await expect(page.locator('[id^="offering-"]')).toHaveCount(0);

		// The reveal button (not "Clear all filters") surfaces the offline
		// offerings. It reads "Show N offering(s)".
		const reveal = page.getByRole('button', { name: /^Show \d+ offering/ });
		await expect(reveal).toBeVisible();

		// Clicking reveal surfaces the previously-hidden offerings.
		await reveal.click();
		await expect(page.locator('[id^="offering-"]').first()).toBeVisible({ timeout: 10000 });
	});
});
