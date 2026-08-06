// Flow — console-errors
// Loads the key PUBLIC website pages headlessly and surfaces severe browser-
// console errors that are invisible to the API-only flows: uncaught JS
// exceptions, blocked responses (ERR_BLOCKED_BY_RESPONSE / X-Frame-Options
// frame refusals), and 4xx/5xx on FIRST-PARTY assets. This catches defects like
// the broken Chatwoot widget (script 404 + iframe frame-blocked) that currently
// throws console errors on every prod page.
//
// Read-only + gentle: two pages, one browser context, closed promptly. Severity:
//   - per severe error                       → FINDING (name page + signal)
//   - navigation failure / 5xx on the doc    → FAIL (the page itself is broken)
//   - a pile-up (>= SEVERE_FAIL_THRESHOLD)   → FAIL (systematic breakage, e.g.
//                                               a widget erroring on every page)
// A single transient third-party hiccup stays a FINDING; a repeated defect
// across both pages becomes a FAIL (forces action).

import { failDetail } from '../http.js';

// Treat these console messages as "severe" (vs. benign third-party noise).
const SEVERE_PATTERNS = [
	/ERR_BLOCKED_BY_RESPONSE/i,
	/Refused to display .* frame/i,
	/X-Frame-Options/i,
	/Failed to load resource/i, // the script 404 that often precedes a frame block
];

const SEVERE_FAIL_THRESHOLD = 3;

function isSevereConsole(text) {
	return SEVERE_PATTERNS.some((re) => re.test(text));
}

// First-party = the website origin or any *.decent-cloud.org subdomain. Third-
// party assets (analytics, CDNs, the widget host) are ignored so a flaky
// external dependency can't flap this flow.
function firstParty(url, webUrl) {
	try {
		const u = new URL(url);
		const w = new URL(webUrl);
		return u.hostname === w.hostname || u.hostname.endsWith('.decent-cloud.org');
	} catch {
		return false;
	}
}

const flow = {
	name: 'console-errors',
	description: 'Load / + /dashboard/marketplace; surface severe browser-console errors',
	requires: [],
	needsBrowser: true, // runner lazy-launches a shared headless Chromium
	async run(ctx) {
		const { webUrl } = ctx.config;
		const pages = ['/', '/dashboard/marketplace'];
		const severe = []; // { page, kind, text }

		const context = await ctx.browser.newContext({ baseURL: webUrl });
		try {
			for (const path of pages) {
				const page = await context.newPage();
				page.on('pageerror', (err) => {
					severe.push({ page: path, kind: 'uncaught-exception', text: err?.message ?? String(err) });
				});
				page.on('console', (msg) => {
					if (msg.type() !== 'error') return;
					const t = msg.text();
					if (isSevereConsole(t)) severe.push({ page: path, kind: 'console-error', text: t });
				});
				page.on('response', (r) => {
					// The navigation document is checked via page.goto below; only
					// asset/sub-resource failures are collected here.
					if (r.status() < 400) return;
					if (r.request().resourceType() === 'document') return;
					if (!firstParty(r.url(), webUrl)) return;
					severe.push({ page: path, kind: 'http-error', text: `HTTP ${r.status()} ${r.url()}` });
				});
				page.on('requestfailed', (req) => {
					if (!firstParty(req.url(), webUrl)) return;
					const failure = req.failure()?.errorText ?? 'request_failed';
					severe.push({ page: path, kind: 'request-failed', text: `${failure} ${req.url()}` });
				});

				const resp = await page.goto(path, { waitUntil: 'domcontentloaded', timeout: 30_000 }).catch((e) => e);
				if (resp instanceof Error) {
					// The page's HTML would not load at all — a hard outage.
					ctx.assert(false, failDetail(`navigation to ${path} failed`, { extra: resp.message }));
				} else if (resp && resp.status() >= 500) {
					ctx.assert(false, failDetail(`${path} returned HTTP ${resp.status()} for the document`, { status: resp.status() }));
				} else if (resp && resp.status() >= 400) {
					// 4xx on the document (e.g. an unauth dashboard route that failed
					// to redirect) is a flag, not necessarily an outage.
					ctx.note(`[${path}] document HTTP ${resp.status()} (expected an auth redirect)`);
				}
				// Give late-loaded widgets (Chatwoot) a beat to fire their errors,
				// then close the page promptly to stay gentle.
				await page.waitForTimeout(1500);
				await page.close().catch(() => {});
			}
		} finally {
			await context.close().catch(() => {});
		}

		for (const s of severe) ctx.note(`[${s.page}] ${s.kind}: ${s.text}`);

		// A pile-up = the same defect hitting multiple pages (systematic, not a
		// transient third-party hiccup) → FAIL so the operator must act.
		if (severe.length >= SEVERE_FAIL_THRESHOLD) {
			ctx.assert(
				false,
				`${severe.length} severe browser-console error(s) across ${pages.length} page(s) — ` +
					`systematic console breakage (e.g. a broken widget). See the FINDING lines above.`,
			);
		}

		ctx.metric('console-errors.severe', severe.length);
		ctx.log(`console-errors: ${severe.length} severe error(s) across ${pages.length} pages`);
	},
};

export default flow;
