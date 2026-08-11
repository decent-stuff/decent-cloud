/**
 * Route audit — DASHBOARD routes (`/dashboard/*` excluding provider/marketplace).
 *
 * Split out of the original `route-audit.spec.ts` so the audit's category
 * slices spread across workers (one file = one scheduling unit). The
 * defect-checking machinery, KNOWN_BROKEN source of truth, and the
 * provider+tenant seed lifecycle live in `fixtures/route-audit-helpers.ts`.
 *
 * NOTE: deliberately NOT `test.describe.configure({ mode: 'serial' })`. Serial
 * mode skips every test after the first failure, which would abort the audit
 * the moment one buggy route is found. The single-setup guarantee serial mode
 * exists for is already provided by the worker-scoped `testAccount` fixture +
 * `test.beforeAll` (one worker = one account, one seeding pass). Non-serial
 * lets every route be checked independently regardless of sibling failures.
 */
import { test as authTest, expect } from './fixtures/test-account';
import {
	auditRoute,
	cleanupAuditContext,
	formatFindings,
	KNOWN_BROKEN,
	printFindingsSummary,
	seedAuditContext,
	type AuditContext,
	type Finding,
} from './fixtures/route-audit-helpers';

const DASHBOARD_ROUTES = [
	'/dashboard',
	'/dashboard/account/profile',
	'/dashboard/account/billing',
	'/dashboard/account/notifications',
	'/dashboard/account/security',
	'/dashboard/admin',
	'/dashboard/cloud/accounts',
	'/dashboard/cloud/resources',
	'/dashboard/invoices',
	'/dashboard/offerings',
	'/dashboard/offerings/create',
	'/dashboard/rentals',
	'/dashboard/reputation',
	'/dashboard/saved',
] as const;

const findings: Finding[] = [];
let ctx: AuditContext;

authTest.describe('route-audit (dashboard)', () => {
	authTest.beforeAll(async ({ testAccount }) => {
		ctx = await seedAuditContext(testAccount);
	});

	authTest.afterAll(async () => {
		if (ctx) await cleanupAuditContext(ctx);
		printFindingsSummary('dashboard', findings);
	});

	for (const url of DASHBOARD_ROUTES) {
		authTest(url, async ({ page }) => {
			authTest.setTimeout(60_000);
			const reason = KNOWN_BROKEN.get(url);
			if (reason) authTest.fail(true, reason);
			const routeFindings = await auditRoute(page, url, { authed: true });
			findings.push(...routeFindings);
			expect(
				routeFindings,
				`Route ${url} — ${routeFindings.length} defect(s):\n${formatFindings(routeFindings)}`,
			).toEqual([]);
		});
	}
});
