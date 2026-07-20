import { test as base } from '@playwright/test';
import { execFile } from 'child_process';
import { promisify } from 'util';
import {
	registerNewAccount,
	signIn,
	setupConsoleLogging,
	type AuthCredentials,
} from './auth-helpers';

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
 * Creates account once per worker, grants admin status, and signs in before each test.
 */
export const test = base.extend<{}, { adminAccount: AuthCredentials }>({
	adminAccount: [
		async ({ browser }, use) => {
			const setupPage = await browser.newPage();
			setupConsoleLogging(setupPage);
			const credentials = await registerNewAccount(setupPage);
			await setupPage.close();

			// Grant admin status via direct DB UPDATE (fast; no cargo run).
			await grantAdminStatus(credentials.username);

			await use(credentials);
		},
		{ scope: 'worker' },
	],

	page: async ({ page, adminAccount }, use) => {
		setupConsoleLogging(page);
		await signIn(page, adminAccount);
		await page.waitForLoadState('networkidle');
		await page.locator(`text=@${adminAccount.username}`).waitFor({ state: 'visible', timeout: 10000 });
		await use(page);
	},
});

export { expect } from '@playwright/test';
