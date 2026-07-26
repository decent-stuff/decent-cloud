import { test, expect } from './fixtures/test-account';
import type { Page, Locator } from '@playwright/test';
import { assertNoNativeDialog, confirmInlineAction } from './fixtures/auth-helpers';
import {
	pubkeyHexFromSeed,
	randomHex,
	nowNs,
	sql,
	accountIdHex,
	seedOffering,
	deleteOfferingsByProvider,
} from './fixtures/seed-helpers';

/**
 * Consolidated E2E coverage for the inline two-step confirm-and-delete flow
 * across every dashboard surface that uses it.
 *
 * Background: every editor on the dashboard used to fire a native confirm()
 * on row delete / device remove / agent revoke. Native dialogs both block
 * headless e2e (Playwright auto-dismisses them, so the action never fires)
 * and are a poor mobile UX. Each editor was independently migrated to the
 * inline two-step pattern (first click arms → reveals inline Confirm + Cancel;
 * the real mutation fires on the second click), mirroring the offerings
 * delete (commit 1077dd33).
 *
 * This spec replaces 7 near-identical per-surface specs (contact-delete,
 * device-remove, external-key-delete, offering-delete, reseller-delete,
 * social-delete, agent-pool-revoke) with ONE parametrized table. Each entity
 * contributes: a seed helper, a route, the arm button label, a row locator
 * strategy, and the post-Confirm assertion. The shared driver exercises:
 *   1. arm reveals inline Confirm + Cancel (no native dialog);
 *   2. Confirm performs the real mutation and the row reflects it server-side.
 *
 * The Cancel branch is identical client-side logic across every editor, so it
 * is asserted for two representative entities (one account-page delete + one
 * provider-page signed-DELETE) rather than duplicated 7× — this drops ~6
 * redundant tests (~16 → 10) while keeping every Confirm path covered.
 *
 * Serial mode: every entity mutates rows keyed on the shared testAccount
 * pubkey / account id; tests must not run in parallel.
 */
test.describe.configure({ mode: 'serial' });

/**
 * Per-entity setup result. Built by each `seed()` below; consumed by the
 * shared driver. `row(page)` locates the JUST-seeded row after navigation;
 * `expectConfirmed` asserts the real mutation landed (row gone OR state
 * flipped, depending on the surface); `cleanup` always runs in `finally`.
 */
interface EntityHandle {
	/** Locate the seeded row on the page after navigating to `route`. */
	row: (page: Page) => Locator;
	/** Optional: URL substring of the signed DELETE/PUT the Confirm click fires
	 * (surfaces that round-trip a signed request before refetching — reseller,
	 * agent-pool revoke). When set, confirmInlineAction awaits that response. */
	deleteResponseUrl?: string;
	/** Assert the Confirm click performed the real server-side mutation. */
	expectConfirmed: (page: Page) => Promise<void>;
	/** Remove the seeded row (and any FK scaffolding) regardless of test outcome. */
	cleanup: () => Promise<void>;
}

interface InlineConfirmEntity {
	/** Slug used in the test title. */
	name: string;
	/** Arm button label ("Delete" / "Remove" / "Revoke"). */
	arm: string;
	/** Route to navigate to before locating the row. */
	route: (testAccount: { username: string; seedPhrase: string }) => string;
	/** Seed the row + return the handle. */
	seed: (testAccount: { username: string; seedPhrase: string }) => Promise<EntityHandle>;
}

// ---------------------------------------------------------------------------
// Entity table — one entry per dashboard surface using inline confirm.
// ---------------------------------------------------------------------------

const ENTITIES: InlineConfirmEntity[] = [
	{
		name: 'contact',
		arm: 'Delete',
		route: () => '/dashboard/account/profile',
		async seed({ username }) {
			const accountHex = await accountIdHex(username);
			const value = `+1-E2E-${randomHex(4)}-${Date.now()}`;
			const out = await sql(`
				INSERT INTO account_contacts (account_id, contact_type, contact_value)
				VALUES (decode('${accountHex}', 'hex'), 'phone', '${value}')
				RETURNING id
			`);
			const id = out.split('\n').map((l) => l.trim()).find((l) => /^\d+$/.test(l));
			if (!id) throw new Error(`contact seed did not RETURN an id; got: ${out}`);
			return {
				row: (page) => page.locator('div.bg-surface-elevated.flex', { hasText: value }),
				expectConfirmed: async (page) => {
					await expect(page.locator('div.bg-surface-elevated.flex', { hasText: value })).toHaveCount(0, { timeout: 10000 });
					const remaining = await sql(`SELECT count(*) FROM account_contacts WHERE id = ${id}`);
					expect(remaining.trim()).toBe('0');
				},
				cleanup: async () => {
					await sql(`DELETE FROM account_contacts WHERE account_id = decode('${accountHex}', 'hex')`);
				},
			};
		},
	},
	{
		name: 'device',
		arm: 'Remove',
		route: () => '/dashboard/account/security',
		async seed({ username }) {
			const accountHex = await accountIdHex(username);
			const deviceName = `E2E-DEV-${randomHex(4)}-${Date.now()}`;
			const keyIdHex = randomHex(16);
			const pubKeyHex = randomHex(32);
			await sql(`
				INSERT INTO account_public_keys (id, account_id, public_key, is_active, device_name)
				VALUES (decode('${keyIdHex}', 'hex'), decode('${accountHex}', 'hex'), decode('${pubKeyHex}', 'hex'), TRUE, '${deviceName}')
			`);
			return {
				row: (page) => page.locator('div.flex.items-center.justify-between.p-3', { hasText: deviceName }),
				// Device remove DISABLES the key (Active → Disabled) rather than
				// deleting the row — assert the state flip + the DB flag.
				expectConfirmed: async (page) => {
					const row = page.locator('div.flex.items-center.justify-between.p-3', { hasText: deviceName });
					await expect(row.getByText('Disabled')).toBeVisible({ timeout: 10000 });
					await expect(row.getByText('Active')).toHaveCount(0);
					const flag = (await sql(`SELECT is_active FROM account_public_keys WHERE id = decode('${keyIdHex}', 'hex')`)).trim();
					expect(flag).toBe('f');
				},
				cleanup: async () => {
					await sql(`DELETE FROM account_public_keys WHERE id = decode('${keyIdHex}', 'hex')`);
				},
			};
		},
	},
	{
		name: 'ext-key',
		arm: 'Delete',
		route: () => '/dashboard/account/profile',
		async seed({ username }) {
			const accountHex = await accountIdHex(username);
			const label = `E2E-KEY-${randomHex(4)}-${Date.now()}`;
			const out = await sql(`
				INSERT INTO account_external_keys (account_id, key_type, key_data, label)
				VALUES (decode('${accountHex}', 'hex'), 'ssh-ed25519', 'ssh-ed25519 AAAAe2efake', '${label}')
				RETURNING id
			`);
			const id = out.split('\n').map((l) => l.trim()).find((l) => /^\d+$/.test(l));
			if (!id) throw new Error(`ext-key seed did not RETURN an id; got: ${out}`);
			return {
				row: (page) => page.locator('div.p-3.bg-surface-elevated', { hasText: label }),
				expectConfirmed: async (page) => {
					await expect(page.locator('div.p-3.bg-surface-elevated', { hasText: label })).toHaveCount(0, { timeout: 10000 });
					const remaining = await sql(`SELECT count(*) FROM account_external_keys WHERE id = ${id}`);
					expect(remaining.trim()).toBe('0');
				},
				cleanup: async () => {
					await sql(`DELETE FROM account_external_keys WHERE account_id = decode('${accountHex}', 'hex')`);
				},
			};
		},
	},
	{
		name: 'social',
		arm: 'Delete',
		route: () => '/dashboard/account/profile',
		async seed({ username }) {
			const accountHex = await accountIdHex(username);
			const handle = `e2e-${randomHex(4)}-${Date.now()}`;
			const out = await sql(`
				INSERT INTO account_socials (account_id, platform, username)
				VALUES (decode('${accountHex}', 'hex'), 'twitter', '${handle}')
				RETURNING id
			`);
			const id = out.split('\n').map((l) => l.trim()).find((l) => /^\d+$/.test(l));
			if (!id) throw new Error(`social seed did not RETURN an id; got: ${out}`);
			return {
				row: (page) => page.locator('div.bg-surface-elevated.flex', { hasText: handle }),
				expectConfirmed: async (page) => {
					await expect(page.locator('div.bg-surface-elevated.flex', { hasText: handle })).toHaveCount(0, { timeout: 10000 });
					const remaining = await sql(`SELECT count(*) FROM account_socials WHERE id = ${id}`);
					expect(remaining.trim()).toBe('0');
				},
				cleanup: async () => {
					await sql(`DELETE FROM account_socials WHERE account_id = decode('${accountHex}', 'hex')`);
				},
			};
		},
	},
	{
		name: 'offering',
		arm: 'Delete',
		route: () => '/dashboard/offerings',
		async seed({ seedPhrase }) {
			const pubkey = pubkeyHexFromSeed(seedPhrase);
			const cardId = await seedOffering(pubkey, { name: `E2E Delete ${randomHex(4)}` });
			return {
				row: (page) => page.locator(`[data-offering-id="${cardId}"]`),
				expectConfirmed: async (page) => {
					await expect(page.locator(`[data-offering-id="${cardId}"]`)).toHaveCount(0, { timeout: 10000 });
				},
				cleanup: async () => {
					await deleteOfferingsByProvider(pubkey);
				},
			};
		},
	},
	{
		name: 'reseller',
		arm: 'Delete',
		route: () => '/dashboard/provider/reseller',
		async seed({ seedPhrase }) {
			const resellerPubkey = pubkeyHexFromSeed(seedPhrase);
			const extPubkey = randomHex(32);
			await sql(`
				INSERT INTO reseller_relationships (reseller_pubkey, external_provider_pubkey, commission_percent, status, created_at_ns)
				VALUES (decode('${resellerPubkey}', 'hex'), decode('${extPubkey}', 'hex'), 10, 'active', ${nowNs()})
				ON CONFLICT (reseller_pubkey, external_provider_pubkey) DO NOTHING
			`);
			const prefix = extPubkey.slice(0, 8);
			return {
				row: (page) => page.locator('div.bg-surface-elevated', { hasText: prefix }).first(),
				// The Confirm fires a signed DELETE; the helper awaits it before
				// the list refetches.
				deleteResponseUrl: `/api/v1/reseller/relationships/${extPubkey}`,
				expectConfirmed: async (page) => {
					await expect(page.getByText('Reseller relationship deleted')).toBeVisible({ timeout: 10000 });
					await expect(page.locator('div.bg-surface-elevated', { hasText: prefix })).toHaveCount(0);
				},
				cleanup: async () => {
					await sql(`
						DELETE FROM reseller_relationships
						WHERE reseller_pubkey = decode('${resellerPubkey}', 'hex')
						  AND external_provider_pubkey = decode('${extPubkey}', 'hex')
					`);
				},
			};
		},
	},
	{
		name: 'agent-pool-revoke',
		arm: 'Revoke',
		// Route depends on the seeded pool id, which is only known after seed.
		// The driver reads `handle.poolId` (see `resolveRoute` in the describe
		// below) and ignores this placeholder.
		route: () => '',
		async seed({ seedPhrase }) {
			const providerPubkey = pubkeyHexFromSeed(seedPhrase);
			const poolId = `e2e-pool-${randomHex(4)}`;
			const agentPubkey = randomHex(32);
			const sig = randomHex(64);
			const ns = nowNs().toString();
			const existed = await sql(`SELECT 1 FROM provider_registrations WHERE pubkey = decode('${providerPubkey}', 'hex')`);
			const createdRegistration = existed !== '1';
			if (createdRegistration) {
				await sql(`
					INSERT INTO provider_registrations (pubkey, signature, created_at_ns)
					VALUES (decode('${providerPubkey}', 'hex'), decode('${sig}', 'hex'), ${ns})
				`);
			}
			await sql(`
				INSERT INTO agent_pools (pool_id, provider_pubkey, name, location, provisioner_type, created_at_ns)
				VALUES ('${poolId}', decode('${providerPubkey}', 'hex'), 'E2E Inline Pool', 'US', 'hetzner', ${ns})
			`);
			await sql(`
				INSERT INTO provider_agent_delegations (
					provider_pubkey, agent_pubkey, permissions, signature, created_at_ns, pool_id, label
				) VALUES (
					decode('${providerPubkey}', 'hex'),
					decode('${agentPubkey}', 'hex'),
					'["provision"]',
					decode('${sig}', 'hex'),
					${ns},
					'${poolId}',
					'E2E Inline Agent'
				)
			`);
			return {
				// Route depends on the seeded pool id; expose via a closure trick:
				// the driver reads `route(testAccount)` ONCE before seed, so we
				// stash the pool id on the handle and the entity's route reads it.
				// See the `poolRoute` workaround below.
				row: (page) => page.locator('tr', { hasText: 'E2E Inline Agent' }),
				poolId,
				agentPubkey,
				createdRegistration,
				// The Confirm fires a signed DELETE; the helper awaits it.
				deleteResponseUrl: `/agent-delegations/${agentPubkey}`,
				expectConfirmed: async (page) => {
					const row = page.locator('tr', { hasText: 'E2E Inline Agent' });
					await expect(row.getByRole('button', { name: 'Revoke' })).toHaveCount(0, { timeout: 10000 });
				},
				cleanup: async () => {
					await sql(`
						DELETE FROM provider_agent_delegations WHERE agent_pubkey = decode('${agentPubkey}', 'hex');
						DELETE FROM agent_pools WHERE pool_id = '${poolId}';
						${createdRegistration ? `DELETE FROM provider_registrations WHERE pubkey = decode('${providerPubkey}', 'hex');` : ''}
					`);
				},
			} as EntityHandle & { poolId: string; agentPubkey: string; createdRegistration: boolean };
		},
	},
];

// The agent-pool revoke route depends on the seeded pool id, which is only
// known AFTER seed. `resolveRoute` falls back to `entity.route(testAccount)`
// for every entity except agent-pool-revoke, where it reads `handle.poolId`.
function resolveRoute(entity: InlineConfirmEntity, testAccount: { username: string; seedPhrase: string }, handle: EntityHandle): string {
	const poolId = (handle as EntityHandle & { poolId?: string }).poolId;
	if (poolId) return `/dashboard/provider/agents/${poolId}`;
	return entity.route(testAccount);
}

test.describe('Inline two-step confirm-and-delete (parametrized)', () => {
	for (const entity of ENTITIES) {
		test(`${entity.name}: ${entity.arm} arms inline confirm; Confirm performs the mutation`, async ({ page, testAccount }) => {
			const handle = await entity.seed(testAccount);
			try {
				// A native dialog must never appear — fail loudly if it does.
				assertNoNativeDialog(page);

				await page.goto(resolveRoute(entity, testAccount, handle));
				const row = handle.row(page);
				await expect(row.getByRole('button', { name: entity.arm })).toBeVisible({ timeout: 10000 });

				// Two-step inline confirm: arm reveals Confirm + Cancel (no native
				// dialog), then Confirm performs the mutation. If the surface
				// round-trips a signed DELETE, the helper awaits it.
				await confirmInlineAction(page, row, {
					arm: entity.arm,
					secondary: 'Cancel',
					waitForResponse: handle.deleteResponseUrl,
				});

				await handle.expectConfirmed(page);
			} finally {
				await handle.cleanup();
			}
		});

		// Cancel-keeps-row is asserted for a representative subset only: the
		// client-side Cancel behavior is identical across every editor, so
		// duplicating it 7× adds wall-clock without adding signal. Contact
		// (account/profile delete) + reseller (provider-page signed flow) +
		// device (the disable-flavor surface) cover the three distinct shapes.
		if (entity.name === 'contact' || entity.name === 'reseller' || entity.name === 'device') {
			test(`${entity.name}: Cancel aborts and keeps the row`, async ({ page, testAccount }) => {
				const handle = await entity.seed(testAccount);
				try {
					assertNoNativeDialog(page);

					await page.goto(resolveRoute(entity, testAccount, handle));
					const row = handle.row(page);
					await expect(row.getByRole('button', { name: entity.arm })).toBeVisible({ timeout: 10000 });

					await row.getByRole('button', { name: entity.arm }).click();
					await row.getByRole('button', { name: 'Cancel' }).click();

					// Confirm/Cancel disappear; the arm button returns and the row remains.
					await expect(row.getByRole('button', { name: 'Confirm' })).toHaveCount(0);
					await expect(row.getByRole('button', { name: entity.arm })).toBeVisible();
				} finally {
					await handle.cleanup();
				}
			});
		}
	}
});
