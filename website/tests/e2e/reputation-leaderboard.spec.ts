import { test, expect } from '@playwright/test';
import {
	sql,
	randomHex,
	pubkeyHexFromSeed,
	seedAccountDirect,
	deleteAccountByUsername,
	deleteContractsByProvider,
	deleteProviderProfileByPubkey,
	nowNs,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for the reputation leaderboard (UX-006): a "Top Providers"
 * section rendered on /dashboard/reputation above the search box, ranked by
 * trust score and completed contracts.
 *
 * DB-direct seed (no reliance on ambient demo data):
 *   - Provider A: an account + provider profile (high trust) + 3 completed
 *     contracts -> rank #1.
 *   - Provider B: a provider profile (lower trust) + 1 completed contract.
 *   - Provider C: a provider profile but ZERO contracts -> the honesty gate
 *     must EXCLUDE it from the leaderboard.
 *   - Provider D: a provider profile + 2 cancelled contracts (0 completed).
 *     The honesty gate is `completed_contracts > 0`, NOT the older
 *     `total_contracts > 0`, so D must also be excluded — this is the
 *     cancelled-only case where requested-then-cancelled rentals
 *     must not look like a track record.
 *
 * Asserts the leaderboard is visible on landing (no search needed), the
 * honesty gate hides C and D, A ranks #1 with its trust/completion metrics,
 * and clicking the row navigates to A's reputation detail page.
 *
 * Not @smoke: moderate DB seeding (account + 4 profiles + 5 contracts) puts
 * it above the <5s smoke bar; it is a regular full-suite test.
 */

const COMPLETED_CONTRACT_SQL = `
	INSERT INTO contract_sign_requests (
		contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact,
		provider_pubkey, offering_id, payment_amount_e9s, request_memo,
		created_at_ns, status, status_updated_at_ns, payment_method,
		stripe_payment_intent_id, stripe_customer_id, currency
	) VALUES (
		decode('{cid}', 'hex'),
		decode('{requester}', 'hex'),
		'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIe2etestkey test@example.com',
		'email:test@example.com',
		decode('{provider}', 'hex'),
		'lb-off', {amount}, 'lb seed',
		{ts}, '{status}', {ts}, 'test', NULL, NULL, 'usd'
	)
`;

test.describe('/dashboard/reputation leaderboard', () => {
	test('renders honesty-gated ranking with metrics and row navigation', async ({ page }) => {
		const tag = randomHex(4);
		const { username: usernameA, seedPhrase: seedA } = await seedAccountDirect(
			`lb-prov-a-${tag}`,
		);
		const pubkeyA = pubkeyHexFromSeed(seedA);
		const pubkeyB = randomHex(32);
		const pubkeyC = randomHex(32);
		const pubkeyD = randomHex(32);
		const requester = randomHex(32);
		const ts = nowNs().toString();

		const nameA = `Leaderboard Prov A ${tag}`;
		const nameB = `Leaderboard Prov B ${tag}`;
		const nameC = `Leaderboard Prov C ${tag}`;
		const nameD = `Leaderboard Prov D ${tag}`;

		try {
			// provider_profiles: A (trust 95), B (trust 70), C (trust 100 but 0
			// contracts), D (trust 80, only cancelled contracts).
			await sql(`
				INSERT INTO provider_profiles (pubkey, name, trust_score, api_version, profile_version, updated_at_ns)
				VALUES (decode('${pubkeyA}', 'hex'), '${nameA}', 95, '1.0', '1.0', 0);
				INSERT INTO provider_profiles (pubkey, name, trust_score, api_version, profile_version, updated_at_ns)
				VALUES (decode('${pubkeyB}', 'hex'), '${nameB}', 70, '1.0', '1.0', 0);
				INSERT INTO provider_profiles (pubkey, name, trust_score, api_version, profile_version, updated_at_ns)
				VALUES (decode('${pubkeyC}', 'hex'), '${nameC}', 100, '1.0', '1.0', 0);
				INSERT INTO provider_profiles (pubkey, name, trust_score, api_version, profile_version, updated_at_ns)
				VALUES (decode('${pubkeyD}', 'hex'), '${nameD}', 80, '1.0', '1.0', 0);
			`);

			// Provider A: 3 completed contracts (1 ICP each).
			for (let i = 0; i < 3; i++) {
				await sql(
					COMPLETED_CONTRACT_SQL.replace('{cid}', randomHex(32))
						.replaceAll('{requester}', requester)
						.replaceAll('{provider}', pubkeyA)
						.replaceAll('{amount}', '1000000000')
						.replaceAll('{status}', 'completed')
						.replaceAll('{ts}', ts),
				);
			}
			// Provider B: 1 completed contract.
			await sql(
				COMPLETED_CONTRACT_SQL.replace('{cid}', randomHex(32))
					.replaceAll('{requester}', requester)
					.replaceAll('{provider}', pubkeyB)
					.replaceAll('{amount}', '1000000000')
					.replaceAll('{status}', 'completed')
					.replaceAll('{ts}', ts),
			);
			// Provider D: 2 cancelled contracts, 0 completed. The OLD
			// leaderboard gate (total_contracts > 0) would wrongly include D;
			// the strengthened gate (completed_contracts > 0) must exclude it.
			for (let i = 0; i < 2; i++) {
				await sql(
					COMPLETED_CONTRACT_SQL.replace('{cid}', randomHex(32))
						.replaceAll('{requester}', requester)
						.replaceAll('{provider}', pubkeyD)
						.replaceAll('{amount}', '0')
						.replaceAll('{status}', 'cancelled')
						.replaceAll('{ts}', ts),
				);
			}

			await page.goto('/dashboard/reputation');

			// The leaderboard section is present on landing (no search needed).
			await expect(page.getByRole('heading', { name: 'Top Providers' })).toBeVisible();

			// The first row resolves once the leaderboard fetch lands.
			const rowA = page.locator('tbody tr').first();

			// Provider A ranks #1 (higher trust + more completed contracts). It has
			// an account, so the row shows its username (displayName prefers
			// username over provider_name).
			await expect(rowA).toContainText(usernameA, { timeout: 10_000 });
			// Trust score, completed contracts, and completion rate render.
			await expect(rowA).toContainText('95');
			await expect(rowA).toContainText('3');
			await expect(rowA).toContainText('100%');

			// Provider B appears (has 1 contract) somewhere in the table.
			await expect(page.locator('table')).toContainText(nameB, { timeout: 10_000 });

			// Honesty gate: Provider C (0 contracts) AND Provider D
			// (cancelled-only, 0 completed) must NOT appear despite non-null
			// trust_scores. Cancelled rentals are not a track record.
			await expect(page.locator('table')).not.toContainText(nameC);
			await expect(page.locator('table')).not.toContainText(nameD);

			// Clicking row #1 navigates to A's reputation detail page.
			await rowA.click();
			await page.waitForURL(`**/dashboard/reputation/${usernameA}`, { timeout: 10_000 });
			await expect(page).toHaveURL(new RegExp(`/dashboard/reputation/${usernameA}$`));
		} finally {
			// Contracts first (NO-ACTION child tables), then profiles, then account.
			await deleteContractsByProvider(pubkeyA);
			await deleteContractsByProvider(pubkeyB);
			await deleteContractsByProvider(pubkeyD);
			await deleteProviderProfileByPubkey(pubkeyA);
			await deleteProviderProfileByPubkey(pubkeyB);
			await deleteProviderProfileByPubkey(pubkeyC);
			await deleteProviderProfileByPubkey(pubkeyD);
			await deleteAccountByUsername(usernameA);
		}
	});
});
