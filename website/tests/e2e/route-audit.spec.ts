/**
 * Comprehensive route-audit spec.
 *
 * Tours EVERY public + authenticated route of the marketplace and captures
 * functional + visual defects across seven categories:
 *   1. HTTP load failures (4xx/5xx) and SvelteKit error pages.
 *   2. Console `error` messages and uncaught `pageerror` exceptions
 *      (excluding the documented-benign dev warnings).
 *   3. Raw data leakage (`undefined`, `NaN`, `null`, `[object Object]`,
 *      raw JSON, stack traces) surfaced in user-visible text.
 *   4. Stuck spinners / skeleton placeholders that never resolve.
 *   5. Template / stub / slop copy (Lorem ipsum, TODO, WIP, "asdf", …).
 *   6. Empty states that show no helpful guidance (informational).
 *   7. Internal `<a href>` links that resolve to a 4xx.
 *
 * Design:
 *   - One testAccount (created DB-direct) is made a provider (own offering)
 *     and a tenant (one active contract) via the EXISTING seed helpers, so the
 *     populated branches of offerings/edit, marketplace detail, rentals detail,
 *     invoices etc. all render real data.
 *   - `test.describe.configure({ mode: 'serial' })` runs setup once.
 *   - Deterministic: no `networkidle` anywhere — content is gated on a body-text
 *     length predicate; spinners are only re-checked (3s grace) when one is
 *     actually present, so fast routes pay nothing.
 *   - Internal link health is checked with HTTP status, de-duplicated GLOBALLY
 *     so the repeated sidebar nav links are verified once, not once-per-route.
 *
 * KNOWN_BROKEN routes are flipped to expected-failure via `test.fail(...)` so
 * the committed suite stays green; the defects they pin are enumerated in the
 * accompanying audit report.
 */
import { test as authTest, expect } from './fixtures/test-account';
import { test as anonTest } from '@playwright/test';
import type { ConsoleMessage, Page, Response } from '@playwright/test';
import {
	pubkeyHexFromSeed,
	seedOffering,
	seedRentableOffering,
	seedContract,
	deleteOfferingsByProvider,
	deleteContractsForRequester,
	sql,
} from './fixtures/seed-helpers';

// ---------------------------------------------------------------------------
// Finding model
// ---------------------------------------------------------------------------
type Severity = 'Critical' | 'High' | 'Medium' | 'Low';

interface Finding {
	route: string;
	severity: Severity;
	category: string;
	issue: string;
	evidence: string;
}

/** Every finding discovered during the run (across all routes). Printed in afterAll. */
const ALL_FINDINGS: Finding[] = [];

/**
 * Routes with a confirmed defect. Keyed by the EXACT audited URL (for static
 * routes) or the route pattern (for dynamic routes, e.g. /dashboard/rentals/[id]).
 * Members are flipped to expected-failure via test.fail so the suite is green.
 */
const KNOWN_BROKEN = new Map<string, string>([
	// Discovered by the audit run. Each entry flips the route to expected-failure
	// so the committed suite is green; the defects remain listed in the report.
]);

// ---------------------------------------------------------------------------
// Noise filters — dev-only messages that are NOT application defects.
// ---------------------------------------------------------------------------
const BENIGN_CONSOLE = [
	/lit is in dev mode/i,
	/stripe\.js .*over http/i,
	/\[vite\]/i,
	/sourcemap/i,
	/\.map\b.*404/i,
	/favicon/i,
	/downloading the (react|vue) devtools/i,
	/listening on ws:\/\/\/ws/i, // vite dev websocket announce
	// SvelteKit aborts in-flight fetches on navigation; in dev mode under
	// parallel workers these surface as "TypeError: Failed to fetch". They are
	// NOT application defects — a real outage would instead trip the
	// checkStuckLoading / checkErrorPage consequence checks below, which remain.
	/Failed to fetch/i,
];

function isBenign(text: string): boolean {
	return BENIGN_CONSOLE.some((re) => re.test(text));
}

// ---------------------------------------------------------------------------
// Defect pattern tables
// ---------------------------------------------------------------------------
const LEAKAGE_PATTERNS: { re: RegExp; sev: Severity; label: string }[] = [
	{ re: /\[object Object\]/, sev: 'High', label: '[object Object]' },
	{ re: /\bundefined\b/i, sev: 'High', label: 'literal "undefined"' },
	{ re: /\bNaN\b/, sev: 'High', label: 'literal "NaN"' },
	{ re: /\bnull\b/i, sev: 'Medium', label: 'literal "null"' },
	{ re: /\{"[a-zA-Z0-9_-]+"\s*:/, sev: 'High', label: 'raw JSON object' },
	{
		// "Error: <message>" followed by a stack frame (at / path.js/.svelte/.ts)
		re: /Error:[^\n]{0,120}(\bat\s|\/[^\s]+\.(js|svelte|ts|mjs)|node_modules)/,
		sev: 'High',
		label: 'Error stack trace',
	},
];

const SLOP_PATTERNS: { re: RegExp; sev: Severity; label: string }[] = [
	{ re: /lorem ipsum/i, sev: 'High', label: 'Lorem ipsum' },
	{ re: /\basdf\b/i, sev: 'High', label: '"asdf"' },
	{ re: /\bWIP\b/, sev: 'High', label: 'WIP' },
	{ re: /not implemented/i, sev: 'High', label: 'not implemented' },
	{ re: /\bTODO\b/, sev: 'Medium', label: 'TODO' },
	{ re: /\blorem\b/i, sev: 'Medium', label: 'lorem' },
	{ re: /coming soon/i, sev: 'Low', label: 'Coming soon' },
	{ re: /\bplaceholder\b/i, sev: 'Low', label: 'placeholder' },
	{ re: /\bdummy\b/i, sev: 'Low', label: 'dummy' },
];

// Bare "test" as a page heading / button label is suspicious (real copy wouldn't
// title a section "test"); checked narrowly to avoid the test-data false flood.
const HEADING_TEST_RE = /^\s*test\s*$/i;

// SvelteKit +error.svelte markers (see src/routes/+error.svelte).
const ERROR_LABELS = [
	'Page not found',
	'Something went wrong',
	'Access denied',
	'Sign in required',
	'Unexpected error',
];

// ---------------------------------------------------------------------------
// Audit core
// ---------------------------------------------------------------------------
interface AuditOptions {
	authed?: boolean;
	/** Minimum ms to let client-rendered data settle after content appears. */
	settleMs?: number;
}

interface AuditContext {
	pubkey: string;
	username: string;
	ownOfferingId: string;
	marketplaceOffering: { providerPubkeyHex: string; offeringNumericId: string };
	contractId: string;
}

let ctx: AuditContext;

/**
 * Visit `url`, run every defect check, return the findings. Never throws on
 * navigation/content problems — records them as findings so the caller's single
 * `expect(findings).toEqual([])` is the only assertion that can fail.
 */
async function auditRoute(page: Page, url: string, opts: AuditOptions = {}): Promise<Finding[]> {
	const findings: Finding[] = [];
	const route = url;
	const consoleErrors: string[] = [];
	const pageErrors: string[] = [];

	const onConsole = (msg: ConsoleMessage) => {
		if (msg.type() === 'error') {
			const text = msg.text();
			if (!isBenign(text)) consoleErrors.push(text);
		}
	};
	const onPageError = (err: Error) => {
		const text = err.message || String(err);
		if (!isBenign(text)) pageErrors.push(text);
	};
	page.on('console', onConsole);
	page.on('pageerror', onPageError);

	const detach = () => {
		page.off('console', onConsole);
		page.off('pageerror', onPageError);
	};

	try {
		// 1. Navigate (capture HTTP status). Some routes (checkout) may redirect.
		let resp: Response | null = null;
		try {
			resp = await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 20000 });
		} catch (err) {
			findings.push({
				route,
				severity: 'Critical',
				category: 'navigation',
				issue: 'Navigation to route threw',
				evidence: err instanceof Error ? err.message : String(err),
			});
			return findings;
		}
		if (resp && resp.status() >= 400) {
			findings.push({
				route,
				severity: 'Critical',
				category: 'http',
				issue: `Route returned HTTP ${resp.status()}`,
				evidence: `GET ${url} -> ${resp.status()}`,
			});
		}

		// 2. Authed routes: confirm the session resolved (Logout chrome visible).
		if (opts.authed) {
			const authed = await page
				.getByRole('button', { name: 'Logout' })
				.waitFor({ state: 'visible', timeout: 8000 })
				.then(() => true)
				.catch(() => false);
			if (!authed) {
				findings.push({
					route,
					severity: 'High',
					category: 'auth',
					issue: 'Authenticated chrome (Logout button) never appeared',
					evidence: 'Logout button not visible within 8s',
				});
			}
		}

		// 3. Wait for meaningful rendered content (universal body-text signal).
		const contentReady = await waitForBodyContent(page, 12000);
		if (!contentReady) {
			findings.push({
				route,
				severity: 'High',
				category: 'render',
				issue: 'Page never rendered meaningful body content (>30 chars)',
				evidence: 'document.body.innerText stayed empty/thin past 12s',
			});
			findings.push(...(await checkStuckLoading(page, route)));
			return findings;
		}

		// 4. Settle for client-side fetch + hydration.
		await page.waitForTimeout(opts.settleMs ?? 700);

		// 5. Error-page markers (covers SSR 200-with-error-page too).
		findings.push(...(await checkErrorPage(page, route)));

		// 6. Console / page errors collected during load.
		for (const text of consoleErrors) {
			findings.push({
				route,
				severity: 'Medium',
				category: 'console-error',
				issue: 'console.error during route load',
				evidence: truncate(text, 240),
			});
		}
		for (const text of pageErrors) {
			findings.push({
				route,
				severity: 'High',
				category: 'pageerror',
				issue: 'Uncaught page exception during route load',
				evidence: truncate(text, 240),
			});
		}

		// 7. Raw data leakage + slop in visible text.
		const bodyText = await safeBodyText(page);
		findings.push(...scanLeakage(bodyText, route));
		findings.push(...scanSlop(bodyText, route));
		findings.push(...(await scanHeadingSlop(page, route)));

		// 8. Stuck spinners / skeletons (only pays 3s when one is present).
		findings.push(...(await checkStuckLoading(page, route)));

		// 9. Broken internal links (de-duplicated globally).
		findings.push(...(await checkBrokenLinks(page, route)));
	} finally {
		detach();
	}
	return findings;
}

async function waitForBodyContent(page: Page, timeout = 10000): Promise<boolean> {
	try {
		await page.waitForFunction(
			() => {
				const t = document.body && document.body.innerText ? document.body.innerText.trim() : '';
				return t.length > 30;
			},
			{ timeout },
		);
		return true;
	} catch {
		return false;
	}
}

async function safeBodyText(page: Page): Promise<string> {
	try {
		return (await page.locator('body').innerText({ timeout: 4000 })) ?? '';
	} catch {
		return '';
	}
}

async function checkErrorPage(page: Page, route: string): Promise<Finding[]> {
	const out: Finding[] = [];
	// Big mono status number rendered by +error.svelte (e.g. "404", "500").
	// Use count() (non-waiting) before textContent() — otherwise textContent()
	// auto-waits up to the test timeout for an element that only exists on the
	// error page, hanging every healthy route for ~60s.
	const monoLoc = page.locator('p.font-mono');
	if ((await monoLoc.count().catch(() => 0)) > 0) {
		const mono = (await monoLoc.first().textContent().catch(() => null)) || '';
		const statusNum = mono.trim();
		if (/^(4\d\d|5\d\d)$/.test(statusNum)) {
			out.push({
				route,
				severity: 'Critical',
				category: 'error-page',
				issue: `SvelteKit error page rendered (status ${statusNum})`,
				evidence: `prominent status "${statusNum}" matched +error.svelte layout`,
			});
			return out;
		}
	}
	for (const label of ERROR_LABELS) {
		// getByText(...).isVisible() returns immediately (no auto-wait).
		const visible = await page
			.getByText(label, { exact: true })
			.first()
			.isVisible()
			.catch(() => false);
		if (visible) {
			out.push({
				route,
				severity: 'Critical',
				category: 'error-page',
				issue: 'SvelteKit error page rendered',
				evidence: `error label "${label}" visible`,
			});
			break;
		}
	}
	return out;
}

function scanLeakage(bodyText: string, route: string): Finding[] {
	const out: Finding[] = [];
	if (!bodyText) return out;
	for (const { re, sev, label } of LEAKAGE_PATTERNS) {
		const m = bodyText.match(re);
		if (m) {
			out.push({
				route,
				severity: sev,
				category: 'data-leakage',
				issue: `Raw ${label} surfaced in user-visible text`,
				evidence: `matched /${re.source}/ near: "${snippet(bodyText, m.index ?? 0)}"`,
			});
		}
	}
	return out;
}

function scanSlop(bodyText: string, route: string): Finding[] {
	const out: Finding[] = [];
	if (!bodyText) return out;
	for (const { re, sev, label } of SLOP_PATTERNS) {
		const m = bodyText.match(re);
		if (m) {
			out.push({
				route,
				severity: sev,
				category: 'slop-text',
				issue: `Stub/placeholder copy ("${label}") present`,
				evidence: `matched /${re.source}/ near: "${snippet(bodyText, m.index ?? 0)}"`,
			});
		}
	}
	return out;
}

async function scanHeadingSlop(page: Page, route: string): Promise<Finding[]> {
	const out: Finding[] = [];
	const labels = await page
		.locator('h1, h2, h3, button')
		.allTextContents()
		.catch(() => [] as string[]);
	for (const raw of labels) {
		const t = (raw || '').trim();
		if (HEADING_TEST_RE.test(t)) {
			out.push({
				route,
				severity: 'Low',
				category: 'slop-text',
				issue: 'Heading/button labelled bare "test"',
				evidence: `"${t}" used as a heading or button label`,
			});
		}
	}
	return out;
}

async function checkStuckLoading(page: Page, route: string): Promise<Finding[]> {
	const out: Finding[] = [];
	const spinners = page.locator('.animate-spin').filter({ visible: true });
	const loadingText = page
		.locator(':text-matches(/^Loading\\.?\\.?$|^Loading…$/, "i")')
		.filter({ visible: true });
	const skeletons = page.locator('[class*="skeleton" i]').filter({ visible: true });

	let spinCount = await spinners.count().catch(() => 0);
	let loadCount = await loadingText.count().catch(() => 0);
	let skelCount = await skeletons.count().catch(() => 0);

	if (spinCount + loadCount + skelCount === 0) return out; // fast path

	// Give persistent indicators up to 3s to resolve, then re-check.
	await page.waitForTimeout(3000);
	spinCount = await spinners.count().catch(() => 0);
	loadCount = await loadingText.count().catch(() => 0);
	skelCount = await skeletons.count().catch(() => 0);

	if (spinCount > 0) {
		out.push({
			route,
			severity: 'High',
			category: 'stuck-loading',
			issue: 'animate-spin spinner still visible after 3s settle',
			evidence: `${spinCount} .animate-spin element(s) persisted`,
		});
	}
	if (loadCount > 0) {
		out.push({
			route,
			severity: 'High',
			category: 'stuck-loading',
			issue: '"Loading..." text still visible after 3s settle',
			evidence: `${loadCount} loading-text node(s) persisted`,
		});
	}
	if (skelCount > 0) {
		out.push({
			route,
			severity: 'Medium',
			category: 'stuck-loading',
			issue: 'Skeleton placeholder still visible after 3s settle',
			evidence: `${skelCount} skeleton element(s) persisted`,
		});
	}
	return out;
}

// Global de-dup of internal-link checks so repeated sidebar nav is verified once.
const CHECKED_LINKS = new Set<string>();

async function checkBrokenLinks(page: Page, route: string): Promise<Finding[]> {
	const out: Finding[] = [];
	let hrefs: string[] = [];
	try {
		hrefs = await page.$$eval('a[href]', (as) => as.map((a) => a.getAttribute('href') || ''));
	} catch {
		return out;
	}
	const origin = new URL(page.url()).origin;
	const unique = Array.from(new Set(hrefs))
		.filter((h) => h && h.startsWith('/') && !h.startsWith('//') && !h.startsWith('/api/'))
		.map((h) => h.split('#')[0].split('?')[0])
		.filter((h) => h.length > 1);

	for (const path of unique.slice(0, 25)) {
		if (CHECKED_LINKS.has(path)) continue;
		CHECKED_LINKS.add(path);
		const abs = origin + path;
		try {
			const resp = await page.request.get(abs, { maxRedirects: 5, timeout: 8000 });
			const status = resp.status();
			if (status >= 400) {
				out.push({
					route,
					severity: 'Medium',
					category: 'broken-link',
					issue: `Internal link resolves to HTTP ${status}`,
					evidence: `<a href="${path}"> -> GET ${status}`,
				});
			}
		} catch (err) {
			out.push({
				route,
				severity: 'Medium',
				category: 'broken-link',
				issue: 'Internal link request failed',
				evidence: `<a href="${path}"> -> ${err instanceof Error ? err.message : 'request error'}`,
			});
		}
	}
	return out;
}

function truncate(s: string, n: number): string {
	const t = s.replace(/\s+/g, ' ').trim();
	return t.length > n ? t.slice(0, n) + '…' : t;
}

function snippet(text: string, index: number, radius = 40): string {
	const start = Math.max(0, index - radius);
	const end = Math.min(text.length, index + radius);
	return text.slice(start, end).replace(/\s+/g, ' ').trim();
}

function formatFindings(findings: Finding[]): string {
	if (findings.length === 0) return '  (none)';
	return findings
		.map((f) => `  [${f.severity}] ${f.category}: ${f.issue}\n      evidence: ${f.evidence}`)
		.join('\n');
}

// ---------------------------------------------------------------------------
// Route tables
// ---------------------------------------------------------------------------
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

const AUTHED_STATIC = [
	'/dashboard',
	'/dashboard/account/profile',
	'/dashboard/account/billing',
	'/dashboard/account/notifications',
	'/dashboard/account/security',
	'/dashboard/account/subscription',
	'/dashboard/admin',
	'/dashboard/cloud/accounts',
	'/dashboard/cloud/resources',
	'/dashboard/invoices',
	'/dashboard/marketplace',
	'/dashboard/marketplace/compare',
	'/dashboard/offerings',
	'/dashboard/offerings/create',
	'/dashboard/provider/support',
	'/dashboard/provider/requests',
	'/dashboard/provider/agents',
	'/dashboard/provider/analytics',
	'/dashboard/provider/earnings',
	'/dashboard/provider/feedback',
	'/dashboard/provider/sla',
	'/dashboard/provider/password-resets',
	'/dashboard/provider/ssh-key-rotations',
	'/dashboard/provider/reseller',
	'/dashboard/rentals',
	'/dashboard/reputation',
	'/dashboard/saved',
	'/dashboard/transfers',
	'/dashboard/validators',
] as const;

// Dynamic routes: title is the route PATTERN (stable test name, also the
// KNOWN_BROKEN key); url is resolved at run time once `ctx` is seeded.
const AUTHED_DYNAMIC: { title: string; url: () => string }[] = [
	{ title: '/dashboard/marketplace/[id]', url: () => `/dashboard/marketplace/${ctx.marketplaceOffering.offeringNumericId}` },
	{ title: '/dashboard/offerings/[id]/edit', url: () => `/dashboard/offerings/${ctx.ownOfferingId}/edit` },
	{ title: '/dashboard/providers/[identifier]', url: () => `/dashboard/providers/${ctx.pubkey}` },
	{ title: '/dashboard/rentals/[contract_id]', url: () => `/dashboard/rentals/${ctx.contractId}` },
	{ title: '/dashboard/reputation/[identifier]', url: () => `/dashboard/reputation/${ctx.pubkey}` },
	{ title: '/dashboard/user/[identifier]', url: () => `/dashboard/user/${ctx.username}` },
];

// ---------------------------------------------------------------------------
// Public (anonymous) routes
// ---------------------------------------------------------------------------
anonTest.describe('route-audit (public)', () => {
	for (const url of PUBLIC_ROUTES) {
		anonTest(url, async ({ page }) => {
			anonTest.setTimeout(60_000);
			const reason = KNOWN_BROKEN.get(url);
			if (reason) anonTest.fail(true, reason);
			const findings = await auditRoute(page, url, { authed: false });
			ALL_FINDINGS.push(...findings);
			expect(
				findings,
				`Route ${url} — ${findings.length} defect(s):\n${formatFindings(findings)}`,
			).toEqual([]);
		});
	}
});

// ---------------------------------------------------------------------------
// Authenticated routes — one provider+tenant account, serial setup
// ---------------------------------------------------------------------------
authTest.describe('route-audit (authenticated)', () => {
	// NOTE: deliberately NOT `test.describe.configure({ mode: 'serial' })`.
	// Serial mode skips every test after the first failure, which would abort
	// the audit the moment one buggy route is found. The single-setup guarantee
	// the serial pattern exists for is already provided by the worker-scoped
	// `testAccount` fixture + `test.beforeAll` under `--workers 1` (one worker =
	// one account, one seeding pass, no parallel cleanup hazard). Non-serial lets
	// every route be checked independently regardless of sibling failures.

	authTest.beforeAll(async ({ testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const username = testAccount.username;

		// Realistic: verified email so rental/account pages don't show a
		// spurious "unverified" banner that could mask other findings.
		await sql(`
			UPDATE accounts SET email_verified = true
			WHERE id = (
				SELECT account_id FROM account_public_keys
				WHERE public_key = decode('${pubkey}', 'hex')
			)
		`);

		// (a) Provider side: an offering OWNED by the fixture account, so the
		// offerings list + edit page render the populated (not empty) branch.
		const ownOfferingId = await seedOffering(pubkey, {
			name: 'E2E Audit Own Offering',
			offeringSource: 'self_provisioned',
			currency: 'usd',
		});

		// (b) Marketplace side: a third-party self_provisioned offering so the
		// marketplace detail page shows a realistic online provider offering.
		const marketplaceOffering = await seedRentableOffering({
			name: 'E2E Audit Marketplace Offering',
		});

		// (c) Tenant side: one active contract for the fixture account renting
		// the marketplace offering — drives rentals list/detail + invoices.
		const contractId = await seedContract({
			requesterPubkeyHex: pubkey,
			status: 'active',
			paymentStatus: 'succeeded',
			providerPubkeyHex: marketplaceOffering.providerPubkeyHex,
			offeringId: marketplaceOffering.offeringId,
		});

		ctx = { pubkey, username, ownOfferingId, marketplaceOffering, contractId };
	});

	authTest.afterAll(async () => {
		if (ctx) {
			await deleteContractsForRequester(ctx.pubkey).catch(() => {});
			await deleteOfferingsByProvider(ctx.pubkey).catch(() => {});
			await deleteOfferingsByProvider(ctx.marketplaceOffering.providerPubkeyHex).catch(() => {});
		}
		// Print the full defect inventory so the report can be lifted from output.
		const bySev: Record<Severity, Finding[]> = { Critical: [], High: [], Medium: [], Low: [] };
		for (const f of ALL_FINDINGS) bySev[f.severity].push(f);
		const lines: string[] = ['\n================ ROUTE-AUDIT SUMMARY ================'];
		lines.push(`total findings: ${ALL_FINDINGS.length}`);
		lines.push(
			`Critical=${bySev.Critical.length} High=${bySev.High.length} Medium=${bySev.Medium.length} Low=${bySev.Low.length}`,
		);
		for (const sev of ['Critical', 'High', 'Medium', 'Low'] as Severity[]) {
			for (const f of bySev[sev]) {
				lines.push(`[${sev}] ${f.route} | ${f.category} | ${f.issue} | ${f.evidence}`);
			}
		}
		lines.push('====================================================\n');
		console.log(lines.join('\n'));
	});

	// Static authed routes.
	for (const url of AUTHED_STATIC) {
		authTest(url, async ({ page }) => {
			authTest.setTimeout(60_000);
			const reason = KNOWN_BROKEN.get(url);
			if (reason) authTest.fail(true, reason);
			const findings = await auditRoute(page, url, { authed: true });
			ALL_FINDINGS.push(...findings);
			expect(
				findings,
				`Route ${url} — ${findings.length} defect(s):\n${formatFindings(findings)}`,
			).toEqual([]);
		});
	}

	// Dynamic authed routes (url resolved at run time from seeded `ctx`).
	for (const r of AUTHED_DYNAMIC) {
		authTest(r.title, async ({ page }) => {
			authTest.setTimeout(60_000);
			const url = r.url();
			const reason = KNOWN_BROKEN.get(url) ?? KNOWN_BROKEN.get(r.title);
			if (reason) authTest.fail(true, reason);
			const findings = await auditRoute(page, url, { authed: true });
			ALL_FINDINGS.push(...findings);
			expect(
				findings,
				`Route ${url} — ${findings.length} defect(s):\n${formatFindings(findings)}`,
			).toEqual([]);
		});
	}
});
