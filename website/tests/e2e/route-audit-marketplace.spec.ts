/**
 * Route audit — MARKETPLACE routes (`/dashboard/marketplace/*`).
 *
 * Split out of the original `route-audit.spec.ts` so the audit's category
 * slices spread across workers (one file = one scheduling unit). The
 * defect-checking machinery, KNOWN_BROKEN source of truth, and the
 * provider+tenant seed lifecycle live in `fixtures/route-audit-helpers.ts`.
 *
 * NOTE: deliberately NOT `test.describe.configure({ mode: 'serial' })` — see
 * route-audit-dashboard.spec.ts for the rationale (audit independence > the
 * fail-fast abort serial mode would impose).
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

const MARKETPLACE_STATIC = ['/dashboard/marketplace', '/dashboard/marketplace/compare'] as const;

// Dynamic routes: title is the route PATTERN (stable test name, also the
// KNOWN_BROKEN key); url is resolved at run time once `ctx` is seeded.
const MARKETPLACE_DYNAMIC: { title: string; url: () => string }[] = [
	{
		title: '/dashboard/marketplace/[id]',
		url: () => `/dashboard/marketplace/${ctx.marketplaceOffering.offeringNumericId}`,
	},
];

const findings: Finding[] = [];
let ctx: AuditContext;

authTest.describe('route-audit (marketplace)', () => {
	authTest.beforeAll(async ({ testAccount }) => {
		ctx = await seedAuditContext(testAccount);
	});

	authTest.afterAll(async () => {
		if (ctx) await cleanupAuditContext(ctx);
		printFindingsSummary('marketplace', findings);
	});

	for (const url of MARKETPLACE_STATIC) {
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

	for (const r of MARKETPLACE_DYNAMIC) {
		authTest(r.title, async ({ page }) => {
			authTest.setTimeout(60_000);
			const url = r.url();
			const reason = KNOWN_BROKEN.get(url) ?? KNOWN_BROKEN.get(r.title);
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
