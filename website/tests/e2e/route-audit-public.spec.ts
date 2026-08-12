/**
 * Route audit — PUBLIC (anonymous) routes.
 *
 * Split out of the original `route-audit.spec.ts` so the audit's category
 * slices spread across workers (one file = one scheduling unit). The
 * defect-checking machinery + KNOWN_BROKEN source of truth live in
 * `fixtures/route-audit-helpers.ts`.
 *
 * These routes are hit ANONYMOUSLY (no `testAccount` sign-in), so there is no
 * seeding lifecycle and no shared findings state — each test asserts its own
 * findings.
 */
import { test as anonTest, expect } from '@playwright/test';
import { auditRoute, formatFindings, KNOWN_BROKEN } from './fixtures/route-audit-helpers';

const PUBLIC_ROUTES = [
	'/login',
	'/recover',
	'/verify-email',
	'/agents',
	'/agents/pricing',
	'/checkout/cancel',
	'/checkout/success',
	'/offline',
] as const;

anonTest.describe('route-audit (public)', () => {
	for (const url of PUBLIC_ROUTES) {
		anonTest(url, async ({ page }) => {
			anonTest.setTimeout(60_000);
			const reason = KNOWN_BROKEN.get(url);
			if (reason) anonTest.fail(true, reason);
			const findings = await auditRoute(page, url, { authed: false });
			expect(
				findings,
				`Route ${url} — ${findings.length} defect(s):\n${formatFindings(findings)}`,
			).toEqual([]);
		});
	}
});
