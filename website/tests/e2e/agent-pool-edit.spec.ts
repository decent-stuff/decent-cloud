import { test, expect, waitForAuthReady } from './fixtures/test-account';
import {
	pubkeyHexFromSeed,
	identityFromSeedPhrase,
	signedApiCall,
	deleteAgentPoolsByProvider,
	sql,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for the Agent Pool EDIT flow (rename via signed PUT) and the
 * pool DETAIL page (/dashboard/provider/agents/[pool_id]).
 *
 * FLOWS.md marked "Agent pools" ✅ but only for the CREATE flow
 * (agent-pool-create.spec.ts). The detail page route and the signed PUT rename
 * path (UpdatePoolRequest → Database::update_agent_pool) were UNTESTED. The
 * detail page itself has no inline rename UI — the edit form lives on the
 * list page (AgentPoolTable → startEdit). To cover the rename contract without
 * duplicating the create spec's UI-form path, this spec drives the signed PUT
 * directly (signedApiCall) and asserts the new name surfaces in BOTH the list
 * table UI AND the detail page header + breadcrumb on next visit.
 *
 * Backend detail: the PUT path is `/api/v1/providers/:pubkey/pools/:pool_id`
 * with an UpdatePoolRequest body (all fields optional); the handler only does
 * `check_authorization` (pubkey == signer), so the testAccount can rename a
 * pool it owns with zero provider pre-seeding.
 *
 * Serial mode: the pool + auto-created provider_registrations row are keyed on
 * the shared testAccount pubkey; finally cleanup deletes both via
 * `deleteAgentPoolsByProvider` (the registration row is bytea, NOT cascaded by
 * account teardown).
 */
test.describe('Agent pool edit flow (/dashboard/provider/agents/[pool_id])', () => {
	test.describe.configure({ mode: 'serial' });

	test('rename persists in DB, list table, and detail page header', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const identity = identityFromSeedPhrase(testAccount.seedPhrase);
		const originalName = `e2e-pool-${Date.now()}`;
		const renamed = `e2e-renamed-${Date.now()}`;
		try {
			// 1. Seed a pool via the signed POST create (the same handler
			//    agent-pool-create.spec.ts exercises, but called directly so
			//    this spec is independent of that spec's UI form). The response
			//    body is ApiResponse<AgentPool> with camelCase poolId.
			const createRes = await signedApiCall(identity, 'POST', `/api/v1/providers/${pubkey}/pools`, {
				name: originalName,
				location: 'europe',
				provisionerType: 'proxmox',
			});
			const createText = await createRes.text();
			expect(createRes.status, `create body=${createText}`).toBe(200);
			const createJson = JSON.parse(createText);
			expect(createJson.success).toBe(true);
			const poolId: string = createJson.data.poolId;
			expect(poolId).toBeTruthy();

			// 2. Rename via the signed PUT (UpdatePoolRequest). This is the
			//    untested contract this spec pins: name only, leaving
			//    location/provisionerType untouched.
			const putRes = await signedApiCall(
				identity,
				'PUT',
				`/api/v1/providers/${pubkey}/pools/${poolId}`,
				{ name: renamed },
			);
			const putText = await putRes.text();
			expect(putRes.status, `put body=${putText}`).toBe(200);
			const putJson = JSON.parse(putText);
			expect(putJson.success).toBe(true);
			expect(putJson.data).toBe(true);

			// 3. DB confirms the rename persisted at the source of truth.
			const dbName = await sql(
				`SELECT name FROM agent_pools WHERE pool_id = '${poolId.replace(/'/g, "''")}'`,
			);
			expect(dbName).toBe(renamed);

			// 4. The list page re-renders the new name in the AgentPoolTable.
			//    The table cell renders the name verbatim; exact:true avoids
			//    matching the pool_id cell (name + '-suffix').
			await page.goto('/dashboard/provider/agents');
			await waitForAuthReady(page);
			await expect(page.getByRole('button', { name: '+ New Pool' })).toBeVisible({ timeout: 15_000 });
			await expect(page.locator('table').getByText(renamed, { exact: true })).toBeVisible({ timeout: 10_000 });

			// 5. The detail page header + breadcrumb reflect the new name on a
			//    fresh visit — previously UNTESTED route. The pool name renders
			//    in both the <h1> header and the trailing breadcrumb span.
			await page.goto(`/dashboard/provider/agents/${poolId}`);
			await waitForAuthReady(page);
			await expect(page.getByRole('heading', { name: renamed, exact: true })).toBeVisible({ timeout: 15_000 });
			// Breadcrumb trailing segment is a <span> (not a link); scope to the
			// nav to avoid matching the same text in the <h1>.
			const breadcrumb = page.locator('nav.text-sm.text-neutral-500');
			await expect(breadcrumb.getByText(renamed, { exact: true })).toBeVisible();
		} finally {
			await deleteAgentPoolsByProvider(pubkey);
		}
	});
});
