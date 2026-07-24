import { test, expect } from './fixtures/test-admin-account';
import {
	identityFromSeedPhrase,
	signedApiCall,
	seedAccountDirect,
	deleteAccountByUsername,
	sql,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for Admin account-management MUTATIONS (FLOWS.md "Admin actions"
 * — the only ❌ in the catalog).
 *
 * These are real signed DB mutations exercised through the admin API handlers
 * (the same handlers `src/lib/services/admin-api.ts` calls from the UI). The
 * admin CALLER is the `adminAccount` fixture (DB-granted is_admin); each test
 * seeds its own SEPARATE throwaway TARGET via seedAccountDirect (a fresh
 * pubkey/identifier) — the admin acts ON the target, never on the shared
 * testAccount. No external service is touched.
 *
 * Signed in Node with `identityFromSeedPhrase` + `signedApiCall` (mirrors
 * `auth-api.ts:signRequest` exactly), not the browser-only `admin-api.ts`
 * helpers (which depend on the browser-side API_BASE_URL resolution).
 *
 * Three independent tests so a failure in one cannot pollute the others:
 *   1. setEmailVerified flips email_verified.
 *   2. setAdminStatus grants then revokes (toggles) is_admin.
 *   3. deleteAccount removes a non-admin target (the delete self-cleans).
 */
test.describe('Admin account mutations (/api/v1/admin/accounts)', () => {
	test.describe.configure({ mode: 'serial' });

	test('setEmailVerified flips the target account email_verified flag', async ({ adminAccount }) => {
		const admin = identityFromSeedPhrase(adminAccount.seedPhrase);
		const target = await seedAccountDirect();

		try {
			// Freshly seeded accounts start unverified.
			const before = await sql(
				`SELECT email_verified FROM accounts WHERE username = '${target.username.replace(/'/g, "''")}'`,
			);
			expect(before).toBe('f');

			const res = await signedApiCall(
				admin,
				'POST',
				`/api/v1/admin/accounts/${encodeURIComponent(target.username)}/email-verified`,
				{ verified: true },
			);
			expect(res.status).toBe(200);
			const body = await res.json();
			expect(body.success).toBe(true);

			const after = await sql(
				`SELECT email_verified FROM accounts WHERE username = '${target.username.replace(/'/g, "''")}'`,
			);
			expect(after).toBe('t');
		} finally {
			await deleteAccountByUsername(target.username);
		}
	});

	test('setAdminStatus grants and then revokes admin privileges', async ({ adminAccount }) => {
		const admin = identityFromSeedPhrase(adminAccount.seedPhrase);
		const target = await seedAccountDirect();

		try {
			expect(
				await sql(`SELECT is_admin FROM accounts WHERE username = '${target.username.replace(/'/g, "''")}'`),
			).toBe('f');

			// Grant.
			const grant = await signedApiCall(
				admin,
				'POST',
				`/api/v1/admin/accounts/${encodeURIComponent(target.username)}/admin-status`,
				{ isAdmin: true },
			);
			expect(grant.status).toBe(200);
			expect((await grant.json()).success).toBe(true);
			expect(
				await sql(`SELECT is_admin FROM accounts WHERE username = '${target.username.replace(/'/g, "''")}'`),
			).toBe('t');

			// Revoke.
			const revoke = await signedApiCall(
				admin,
				'POST',
				`/api/v1/admin/accounts/${encodeURIComponent(target.username)}/admin-status`,
				{ isAdmin: false },
			);
			expect(revoke.status).toBe(200);
			expect((await revoke.json()).success).toBe(true);
			expect(
				await sql(`SELECT is_admin FROM accounts WHERE username = '${target.username.replace(/'/g, "''")}'`),
			).toBe('f');
		} finally {
			await deleteAccountByUsername(target.username);
		}
	});

	test('deleteAccount removes a non-admin target and a re-fetch reports it gone', async ({ adminAccount }) => {
		const admin = identityFromSeedPhrase(adminAccount.seedPhrase);
		// CRITICAL: the target must NOT be an admin — the API refuses to delete
		// admin accounts ("Cannot delete admin accounts"). seedAccountDirect
		// creates a plain non-admin account, exactly what we need.
		const target = await seedAccountDirect();
		const safeUser = target.username.replace(/'/g, "''");

		// Delete the target.
		const del = await signedApiCall(
			admin,
			'DELETE',
			`/api/v1/admin/accounts/${encodeURIComponent(target.username)}`,
		);
		expect(del.status).toBe(200);
		const summary = await del.json();
		expect(summary.success).toBe(true);
		// The handler returns a deletion summary with resource counts.
		expect(summary.data).toBeTruthy();
		expect(typeof summary.data.publicKeysDeleted).toBe('number');

		// The account row is gone.
		const count = await sql(`SELECT count(*) FROM accounts WHERE username = '${safeUser}'`);
		expect(count).toBe('0');

		// A subsequent admin fetch of the target reports it gone. The handler
		// returns 200 with { success:false, error:"Account not found" } for an
		// unknown username (it does not emit a 404), so assert on the body.
		const refetch = await signedApiCall(
			admin,
			'GET',
			`/api/v1/admin/accounts/${encodeURIComponent(target.username)}`,
		);
		expect(refetch.status).toBe(200);
		const refetchBody = await refetch.json();
		expect(refetchBody.success).toBe(false);
		expect(refetchBody.error).toContain('Account not found');
	});
});
