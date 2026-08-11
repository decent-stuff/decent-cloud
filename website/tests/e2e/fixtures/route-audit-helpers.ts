/**
 * Shared infrastructure for the split `route-audit-*.spec.ts` suite.
 *
 * The original single-file audit (`route-audit.spec.ts`, 751 lines / 41 tests)
 * could only occupy one worker-slot, so the whole ~100 s cost was serial. The
 * suite is now split into category files (`-public`, `-dashboard`,
 * `-provider`, `-marketplace`, `-misc`) that spread across workers; every file
 * imports its defect-checking machinery + seeding lifecycle from HERE so the
 * audit logic stays in one place.
 *
 * What lives here:
 *   - The `Finding` model + the seven defect-category checks (`auditRoute`).
 *   - Noise filters + pattern tables (console noise, leakage, slop, error pages,
 *     stuck loading, broken internal links).
 *   - `KNOWN_BROKEN` — the single source of truth for routes flipped to
 *     expected-failure so the committed suite stays green.
 *   - `seedAuditContext` / `cleanupAuditContext` — the provider+tenant seed the
 *     populated branches of the audited routes render against. DB-direct and
 *     cheap, so each authed split file runs its own copy.
 *   - `printFindingsSummary` — the per-file report printer (cross-worker state
 *     can't be shared via a module array; each worker is its own process, so
 *     each file reports its own slice).
 *
 * Design notes carried over from the original spec:
 *   - No `networkidle` anywhere. Content is gated on a body-text length
 *     predicate; spinners are only re-checked (3 s grace) when one is present,
 *     so fast routes pay nothing.
 *   - Internal-link health is checked with HTTP status, de-duplicated PER
 *     PROCESS via `CHECKED_LINKS` (sidebar nav verified once within a worker).
 */
import type { ConsoleMessage, Page, Request, Response } from '@playwright/test';
import {
	pubkeyHexFromSeed,
	seedOffering,
	seedRentableOffering,
	seedContract,
	deleteOfferingsByProvider,
	deleteContractsForRequester,
	verifyAccountEmail,
} from './seed-helpers';

// ---------------------------------------------------------------------------
// Finding model
// ---------------------------------------------------------------------------
export type Severity = 'Critical' | 'High' | 'Medium' | 'Low';

export interface Finding {
	route: string;
	severity: Severity;
	category: string;
	issue: string;
	evidence: string;
}

/**
 * The shape of the account-level seed the audit pages render against. Produced
 * by `seedAuditContext`, consumed by the dynamic-route URL resolvers in the
 * split spec files (`/dashboard/rentals/[contract_id]` etc.).
 */
export interface AuditContext {
	pubkey: string;
	username: string;
	ownOfferingId: string;
	marketplaceOffering: { providerPubkeyHex: string; offeringNumericId: string };
	contractId: string;
}

/**
 * Routes with a confirmed defect. Keyed by the EXACT audited URL (for static
 * routes) or the route pattern (for dynamic routes, e.g. /dashboard/rentals/[id]).
 * Members are flipped to expected-failure via test.fail so the suite is green.
 *
 * Currently empty: the previously-pinned defects were resolved. New defects the
 * audit catches should be triaged (fixed or pinned here) before merge.
 */
export const KNOWN_BROKEN = new Map<string, string>([]);

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
	// Chrome network-layer errors (e.g. "Failed to load resource:
	// net::ERR_CONNECTION_CLOSED") fired by EXTERNAL resources — Google Fonts
	// CDN, analytics, etc. — when 4 Chromium workers contend for CPU/network.
	// These are never first-party app defects: the app's own API failures
	// surface as HTTP status codes ("the server responded with a status of
	// 4xx/5xx"), and a real outage still trips checkStuckLoading / content-ready.
	/net::ERR_/i,
];

function isBenign(text: string, sourceUrl?: string): boolean {
	if (BENIGN_CONSOLE.some((re) => re.test(text))) return true;
	// Resource-load failures ("Failed to load resource: ...") whose source
	// location is an EXTERNAL host — Google Fonts CDN (a stale woff2 hash 404s
	// every load), analytics, etc. — are not first-party app defects. The
	// console message's source location IS the resource URL that failed, so we
	// can scope the filter to non-localhost hosts and keep real localhost asset
	// failures (which `checkBrokenLinks`/content-ready may not otherwise catch).
	if (sourceUrl && /Failed to load resource/.test(text)) {
		try {
			const { host } = new URL(sourceUrl);
			if (host !== 'localhost' && host !== '127.0.0.1' && !sourceUrl.includes('/api/v1/')) {
				return true;
			}
		} catch {
			// sourceUrl wasn't a real URL — keep the error as a finding.
		}
	}
	return false;
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
export interface AuditOptions {
	authed?: boolean;
	/** Max ms to wait for in-flight `/api/v1/` requests to settle after content
	 * appears. The settle ONLY fires when a client API request is actually
	 * pending — routes whose content is fully SSR'd pay nothing. */
	settleMs?: number;
}

/**
 * Visit `url`, run every defect check, return the findings. Never throws on
 * navigation/content problems — records them as findings so the caller's single
 * `expect(findings).toEqual([])` is the only assertion that can fail.
 */
export async function auditRoute(page: Page, url: string, opts: AuditOptions = {}): Promise<Finding[]> {
	const findings: Finding[] = [];
	const route = url;
	const consoleErrors: string[] = [];
	const pageErrors: string[] = [];

	const onConsole = (msg: ConsoleMessage) => {
		if (msg.type() === 'error') {
			const text = msg.text();
			// msg.location().url is the resource URL that failed for
			// "Failed to load resource" messages — used to scope the external-CDN
			// benign filter in isBenign (keeps real localhost asset failures).
			if (!isBenign(text, msg.location()?.url)) consoleErrors.push(text);
		}
	};
	const onPageError = (err: Error) => {
		const text = err.message || String(err);
		if (!isBenign(text)) pageErrors.push(text);
	};
	page.on('console', onConsole);
	page.on('pageerror', onPageError);

	// Track in-flight /api/v1/ requests so the settle after content-ready only
	// fires when a client fetch is actually pending (replaces the previous
	// blanket 700ms timeout that fired on every route).
	const apiTracker = trackPendingApiRequests(page);

	const detach = () => {
		page.off('console', onConsole);
		page.off('pageerror', onPageError);
		apiTracker.detach();
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

		// 4. Settle ONLY if a client `/api/v1/` fetch is still in flight; routes
		// whose content is fully SSR'd pay nothing (was a blanket 700ms sleep).
		await settleForPendingApi(page, apiTracker, opts.settleMs ?? 2000);

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

		// 9. Broken internal links (de-duplicated per process).
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

/**
 * Tracker for in-flight `/api/v1/` requests on a page. Install before
 * navigating so the request/response listeners see every client fetch the
 * route triggers; `count()` reports how many are still pending; `detach()`
 * removes the listeners. Used by `settleForPendingApi` to replace the previous
 * blanket 700ms timeout — routes whose content is fully SSR'd (no client
 * fetch) pay zero settle, while routes that fire a client fetch wait just long
 * enough for it to resolve (bounded).
 */
interface ApiRequestTracker {
	count: () => number;
	detach: () => void;
}

function trackPendingApiRequests(page: Page): ApiRequestTracker {
	let pending = 0;
	const onRequest = (req: Request) => {
		if (req.url().includes('/api/v1/')) pending++;
	};
	const onResponse = (resp: Response) => {
		if (resp.url().includes('/api/v1/')) pending = Math.max(0, pending - 1);
	};
	page.on('request', onRequest);
	page.on('response', onResponse);
	return {
		count: () => pending,
		detach: () => {
			page.off('request', onRequest);
			page.off('response', onResponse);
		},
	};
}

/**
 * Wait for in-flight `/api/v1/` requests to drain, bounded by `timeout`. Fast
 * path: if nothing is pending, returns immediately (no sleep). The
 * `checkStuckLoading` 3s grace further down the pipeline catches the
 * "never resolved" tail, so this bound can stay tight.
 */
async function settleForPendingApi(page: Page, tracker: ApiRequestTracker, timeout = 2000): Promise<void> {
	if (tracker.count() === 0) return; // fast path: no client API fetch in flight
	const deadline = Date.now() + timeout;
	while (tracker.count() > 0 && Date.now() < deadline) {
		await page.waitForTimeout(50);
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

// Per-process de-dup of internal-link checks so repeated sidebar nav is verified
// once within a worker. (Cross-worker re-checking is acceptable — minor HTTP cost.)
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

export function formatFindings(findings: Finding[]): string {
	if (findings.length === 0) return '  (none)';
	return findings
		.map((f) => `  [${f.severity}] ${f.category}: ${f.issue}\n      evidence: ${f.evidence}`)
		.join('\n');
}

// ---------------------------------------------------------------------------
// Seed lifecycle — the provider+tenant state the audited pages render against.
// DB-direct + cheap, so each authed split file runs its own copy.
// ---------------------------------------------------------------------------

/**
 * Build the audit seed for one `testAccount`: a verified-email provider with
 * its OWN offering (offerings list/edit populated), a third-party rentable
 * marketplace offering (marketplace detail populated), and one active contract
 * renting it (rentals list/detail + invoices populated). Returns the handles
 * the dynamic-route URL resolvers need.
 */
export async function seedAuditContext(testAccount: {
	seedPhrase: string;
	username: string;
}): Promise<AuditContext> {
	const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
	const username = testAccount.username;

	// Realistic: verified email so rental/account pages don't show a
	// spurious "unverified" banner that could mask other findings.
	await verifyAccountEmail(pubkey);

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

	return { pubkey, username, ownOfferingId, marketplaceOffering, contractId };
}

/**
 * Tear down a `seedAuditContext` seed: the fixture account's contracts +
 * offerings, and the third-party marketplace provider's offerings. Best-effort
 * (errors swallowed) so one stuck cleanup never aborts the suite.
 */
export async function cleanupAuditContext(ctx: AuditContext): Promise<void> {
	await deleteContractsForRequester(ctx.pubkey).catch(() => undefined);
	await deleteOfferingsByProvider(ctx.pubkey).catch(() => undefined);
	await deleteOfferingsByProvider(ctx.marketplaceOffering.providerPubkeyHex).catch(() => undefined);
}

/**
 * Print the per-file defect inventory so the report can be lifted from output.
 * Cross-worker aggregation isn't possible via module state (each worker is its
 * own process), so each split file reports its own slice.
 */
export function printFindingsSummary(scope: string, findings: Finding[]): void {
	const bySev: Record<Severity, Finding[]> = { Critical: [], High: [], Medium: [], Low: [] };
	for (const f of findings) bySev[f.severity].push(f);
	const lines: string[] = [`\n================ ROUTE-AUDIT SUMMARY (${scope}) ================`];
	lines.push(`total findings: ${findings.length}`);
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
}
