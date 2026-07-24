import { test, expect, waitForAuthReady } from './fixtures/test-account';
import {
	pubkeyHexFromSeed,
	deleteAgentPoolsByProvider,
	sql,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for the Agent Pool CREATE flow (/dashboard/provider/agents).
 *
 * FLOWS.md "Agent pools" was ⚠️: only the "+ New Pool" button render was
 * asserted; pool create/manage was not. This spec drives the create form to its
 * signed POST and asserts the pool appears in the AgentPoolTable.
 *
 * Key backend detail (verified in `api/src/database/agent_pools.rs:~205`):
 * `Database::create_agent_pool` AUTO-CREATES the `provider_registrations` FK row
 * (`INSERT ... ON CONFLICT DO NOTHING`), so the testAccount can create a pool
 * with ZERO provider pre-seeding. The handler `create_pool` only does
 * `check_authorization` (pubkey == signer) — no onboarding gate.
 *
 * Serial mode: the pool + auto-created provider_registrations row are keyed on
 * the shared testAccount pubkey; cleanup deletes both via
 * `deleteAgentPoolsByProvider` in finally (the registration row is bytea, NOT
 * cascaded by account teardown).
 */
test.describe('Agent pool create flow (/dashboard/provider/agents)', () => {
	test.describe.configure({ mode: 'serial' });

	test('creates an agent pool and lists it in the pool table', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const poolName = `e2e-pool-${Date.now()}`;
		try {
			await page.goto('/dashboard/provider/agents');
			await waitForAuthReady(page);

			// Authenticated, data-loaded signal.
			await expect(page.getByRole('button', { name: '+ New Pool' })).toBeVisible({ timeout: 15000 });

			// Open the create form.
			await page.getByRole('button', { name: '+ New Pool' }).click();
			await expect(page.getByRole('heading', { name: 'Create Agent Pool' })).toBeVisible();

			// Fill the name; leave location/provisioner at their defaults
			// (europe / proxmox). No cloud account or pre-seed is required.
			await page.locator('#poolName').fill(poolName);

			// Submit and wait for the success banner that follows the signed POST.
			const created = page.waitForResponse(
				(resp) =>
					resp.request().method() === 'POST' &&
					resp.url().includes(`/api/v1/providers/${pubkey}/pools`),
				{ timeout: 20000 },
			);
			await page.getByRole('button', { name: 'Create Pool' }).click();
			const res = await created;
			expect(res.ok()).toBeTruthy();

			await expect(page.getByText(`Pool "${poolName}" created`)).toBeVisible({ timeout: 10000 });

			// The new pool row renders in the AgentPoolTable (pool name is the row
			// label). This proves loadData() re-fetched and the row persisted.
			// exact:true avoids also matching the pool_id cell (name + '-suffix').
			await expect(page.locator('table').getByText(poolName, { exact: true })).toBeVisible({ timeout: 10000 });

			// DB confirms the pool + the auto-created provider_registrations row.
			const poolCount = await sql(
				`SELECT count(*) FROM agent_pools WHERE provider_pubkey = decode('${pubkey}', 'hex') AND name = '${poolName.replace(/'/g, "''")}'`,
			);
			expect(poolCount).toBe('1');
			const regCount = await sql(
				`SELECT count(*) FROM provider_registrations WHERE pubkey = decode('${pubkey}', 'hex')`,
			);
			expect(regCount).toBe('1');
		} finally {
			await deleteAgentPoolsByProvider(pubkey);
		}
	});
});
