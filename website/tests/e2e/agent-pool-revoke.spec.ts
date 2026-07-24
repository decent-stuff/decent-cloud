import { test, expect } from './fixtures/test-account';
import {
	pubkeyHexFromSeed,
	randomHex,
	nowNs,
	sql,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for the provider agent-pool page two-step confirms
 * (revoke agent delegation; upgrade pool) — replaces native confirm().
 *
 * The page previously used confirm() for both revoke and upgrade, which blocks
 * headless e2e (Playwright auto-dismisses it, so the action never fires) and is
 * poor on mobile. Both now use the inline two-step pattern (commit 1077dd33):
 * the first click only arms the action, revealing inline Confirm + Cancel
 * buttons; the real API call fires on the second click.
 *
 * Seeding model: the testAccount user is the PROVIDER. We seed the minimal FK
 * chain (provider_registrations → agent_pools → provider_agent_delegations)
 * directly so the page renders a delegation row with a Revoke button. No
 * first-party API is mocked.
 *
 * Serial mode: all testAccount users share one provider pubkey; these tests
 * mutate rows keyed on it.
 */
test.describe.configure({ mode: 'serial' });

interface PoolSeed {
	poolId: string;
	agentPubkey: string;
	createdRegistration: boolean;
}

async function seedPool(providerPubkey: string): Promise<PoolSeed> {
	const poolId = `e2e-pool-${randomHex(4)}`;
	const agentPubkey = randomHex(32);
	const sig = randomHex(64);
	const ns = nowNs().toString();
	// provider_registrations is the FK root for agent_pools.provider_pubkey.
	// Track whether WE created it so cleanup doesn't nuke another spec's row.
	const existed = await sql(
		`SELECT 1 FROM provider_registrations WHERE pubkey = decode('${providerPubkey}', 'hex')`,
	);
	const createdRegistration = existed !== '1';
	if (createdRegistration) {
		await sql(`
			INSERT INTO provider_registrations (pubkey, signature, created_at_ns)
			VALUES (decode('${providerPubkey}', 'hex'), decode('${sig}', 'hex'), ${ns})
		`);
	}
	await sql(`
		INSERT INTO agent_pools (pool_id, provider_pubkey, name, location, provisioner_type, created_at_ns)
		VALUES ('${poolId}', decode('${providerPubkey}', 'hex'), 'E2E Pool', 'US', 'hetzner', ${ns})
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
			'E2E Agent'
		)
	`);
	return { poolId, agentPubkey, createdRegistration };
}

async function cleanupPool(providerPubkey: string, seed: PoolSeed): Promise<void> {
	await sql(`
		DELETE FROM provider_agent_delegations WHERE agent_pubkey = decode('${seed.agentPubkey}', 'hex');
		DELETE FROM agent_pools WHERE pool_id = '${seed.poolId}';
		${seed.createdRegistration ? `DELETE FROM provider_registrations WHERE pubkey = decode('${providerPubkey}', 'hex');` : ''}
	`);
}

test.describe('Provider agent pool — revoke (inline two-step confirm)', () => {
	test('first Revoke click reveals an inline confirm; Cancel aborts and keeps the agent', async ({ page, testAccount }) => {
		const providerPubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const seed = await seedPool(providerPubkey);
		try {
			await page.goto(`/dashboard/provider/agents/${seed.poolId}`);

			// The delegation row renders a Revoke button (active delegation).
			const row = page.locator('tr', { hasText: 'E2E Agent' });
			await expect(row.getByRole('button', { name: 'Revoke' })).toBeVisible({ timeout: 10000 });

			// First click: no native dialog — it reveals inline Confirm + Cancel.
			await row.getByRole('button', { name: 'Revoke' }).click();
			await expect(row.getByRole('button', { name: 'Confirm' })).toBeVisible();
			await expect(row.getByRole('button', { name: 'Cancel' })).toBeVisible();

			// Cancel aborts: Confirm/Cancel disappear, Revoke returns.
			await row.getByRole('button', { name: 'Cancel' }).click();
			await expect(row.getByRole('button', { name: 'Confirm' })).toHaveCount(0);
			await expect(row.getByRole('button', { name: 'Revoke' })).toBeVisible();
		} finally {
			await cleanupPool(providerPubkey, seed);
		}
	});

	test('second Revoke click revokes the agent delegation (row leaves the active list)', async ({ page, testAccount }) => {
		const providerPubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const seed = await seedPool(providerPubkey);
		try {
			await page.goto(`/dashboard/provider/agents/${seed.poolId}`);
			const row = page.locator('tr', { hasText: 'E2E Agent' });
			await expect(row.getByRole('button', { name: 'Revoke' })).toBeVisible({ timeout: 10000 });

			// Arm, then confirm the revoke. Wait for the signed DELETE.
			const deleteReq = page.waitForResponse(
				(resp) =>
					resp.request().method() === 'DELETE' &&
					resp.url().includes(`/agent-delegations/${seed.agentPubkey}`),
				{ timeout: 15000 },
			);
			await row.getByRole('button', { name: 'Revoke' }).click();
			await row.getByRole('button', { name: 'Confirm' }).click();
			await deleteReq;

			// After refresh, the active delegation's Revoke button is gone (the
			// row either disappears or flips to a non-active "Revoked" state with
			// no Revoke button).
			await expect(row.getByRole('button', { name: 'Revoke' })).toHaveCount(0, { timeout: 10000 });
		} finally {
			await cleanupPool(providerPubkey, seed);
		}
	});
});
