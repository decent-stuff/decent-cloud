import { test as base } from '@playwright/test';
import { exec } from 'child_process';
import { promisify } from 'util';
import {
	registerNewAccount,
	signIn,
	setupConsoleLogging,
	type AuthCredentials,
} from './auth-helpers';

const execAsync = promisify(exec);

/**
 * Grant admin status to a user via api-cli
 */
async function grantAdminStatus(username: string): Promise<void> {
	const dbPath = 'sqlite:../api/e2e-test.db';
	const cmd = `DATABASE_URL="${dbPath}" SQLX_OFFLINE=true cargo run --bin api-cli -- admin grant ${username}`;
	await execAsync(cmd, { cwd: process.cwd() });
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

			// Grant admin status via api-cli
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
