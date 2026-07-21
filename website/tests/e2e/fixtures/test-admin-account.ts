import { test as base } from '@playwright/test';
import { execFile } from 'child_process';
import { promisify } from 'util';
import { setupConsoleLogging, type AuthCredentials } from './auth-helpers';
import { seedAccountDirect, deleteAccountByUsername } from './seed-helpers';

const execFileAsync = promisify(execFile);

// In the dev container PostgreSQL is reachable at hostname `postgres`; on host
// setups `localhost` is more common. Honour DATABASE_URL when provided.
const DATABASE_URL = process.env.DATABASE_URL || 'postgres://test:test@postgres:5432/test';

/**
 * Grant admin status to a user via a direct DB UPDATE.
 *
 * Why not `api-cli admin grant` or the admin-status endpoint? Both cost a full
 * `cargo run` (multi-second) or require an already-admin auth token we cannot
 * bootstrap from an empty DB. A direct UPDATE is the cheapest correct path.
 */
async function grantAdminStatus(username: string): Promise<void> {
	// Parse the connection string into psql args (avoids leaking it via `psql`'s
	// argv in process listings and works regardless of `psql://://` quoting).
	const url = new URL(DATABASE_URL);
	const host = url.hostname || 'postgres';
	const port = url.port || '5432';
	const user = url.username || 'test';
	const dbName = url.pathname.replace(/^\//, '') || 'test';
	const password = url.password || 'test';

	const { stdout } = await execFileAsync(
		'psql',
		[
			'--host',
			host,
			'--port',
			port,
			'--username',
			user,
			'--dbname',
			dbName,
			'--no-psqlrc',
			'--tuples-only',
			'--no-align',
			'--command',
			`UPDATE accounts SET is_admin = TRUE WHERE LOWER(username) = LOWER('${username.replace(/'/g, "''")}') RETURNING username`,
		],
		{ env: { ...process.env, PGPASSWORD: password } },
	);

	const returned = stdout.trim();
	if (!returned) {
		throw new Error(`grantAdminStatus: no rows updated for username="${username}"`);
	}
}

/**
 * Test fixture for admin user tests.
 * Creates account once per worker, grants admin status, and silently
 * authenticates each test by injecting the seed phrase into localStorage
 * (same fast-auth pattern as test-account.ts; no per-test UI sign-in).
 */
export const test = base.extend<{}, { adminAccount: AuthCredentials }>({
	adminAccount: [
		async ({}, use) => {
			const credentials = await seedAccountDirect();
			await grantAdminStatus(credentials.username);
			await use(credentials);
			// Teardown: delete the account to prevent data accumulation across
			// suite runs (same pattern as test-account.ts testAccount fixture).
			try {
				await deleteAccountByUsername(credentials.username);
			} catch (err) {
				console.warn(
					`adminAccount teardown: failed to delete account "${credentials.username}"`,
					err instanceof Error ? err.message : err,
				);
			}
		},
		{ scope: 'worker' },
	],

	// Override context: pre-seed seed_phrases + dismiss WelcomeModal.
	context: async ({ context, adminAccount }, use) => {
		const seed = adminAccount.seedPhrase;
		await context.addInitScript((s: string) => {
			localStorage.setItem('seed_phrases', JSON.stringify([s]));
			sessionStorage.setItem('first_login_onboarding_completed', 'true');
		}, seed);
		await use(context);
	},

	// Override page: skip UI sign-in; land directly on /dashboard authenticated.
	page: async ({ page }, use) => {
		setupConsoleLogging(page);
		await page.goto('/dashboard');
		// Logout button visibility IS the auth-ready signal; do not wait for
		// networkidle (vite HMR keeps the network busy and tanks parallel runs).
		await page.getByRole('button', { name: 'Logout' }).waitFor({ state: 'visible', timeout: 15000 });
		await use(page);
	},
});

export { expect } from '@playwright/test';
