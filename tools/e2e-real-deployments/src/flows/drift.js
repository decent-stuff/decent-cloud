// Flow — drift
// Compares key READ-ONLY API endpoints between the CURRENT target and a
// reference origin (prod, the source of truth). Catches config/schema drift
// between envs that the per-target flows can't see on their own, e.g.:
//   - stage returning GET /auth/capabilities 404 while prod 200s,
//   - stage still serving the retired ICP currency while prod serves USD/Stripe,
//   - a non-prod env reporting environment='prod' (config bleed).
//
// Graceful by design: a DNS/network failure reaching EITHER side is itself a
// drift FINDING, never a crash. The stage-* hostnames DNS-fail today, so this
// flow is normally run with --target dev (the legacy stage proxy). When the
// current target IS the reference (prod), there is nothing to diff → PASS with
// a note and no extra requests.
//
// The reference origin defaults to prod; override with
// DC_E2E_DRIFT_REFERENCE_API for split-brain debugging. All FINDINGs only —
// drift is a comparison surface, differences are surfaced not failed on.

import { httpJson, redact } from '../http.js';

const REFERENCE_API_DEFAULT = 'https://api.decent-cloud.org';

const PATHS = {
	health: '/api/v1/health',
	capabilities: '/api/v1/auth/capabilities',
	stats: '/api/v1/stats',
	offerings: '/api/v1/offerings?limit=20',
};

// Currencies whose presence means a target is serving RETIRED payment rails
// (ICPay/BTC are fully retired — Stripe is the sole rail).
const RETIRED_CURRENCIES = ['icp', 'btc'];

/** GET that never throws — returns a normalized result envelope. */
async function safeGet(label, origin, path) {
	const url = `${origin}${path}`;
	try {
		const r = await httpJson(url, { method: 'GET', timeoutMs: 20_000 });
		return { label, url, ok: true, status: r.status, json: r.json };
	} catch (e) {
		return { label, url, ok: false, error: e?.message ?? String(e) };
	}
}

/** Distinct currencies / payment_methods across an offerings sample. */
function offeringSets(json) {
	const data = Array.isArray(json?.data) ? json.data : [];
	const cur = new Set();
	const pay = new Set();
	for (const o of data) {
		if (typeof o.currency === 'string' && o.currency) cur.add(o.currency.toLowerCase());
		if (typeof o.payment_methods === 'string' && o.payment_methods) {
			for (const p of o.payment_methods.split(',')) {
				const t = p.trim().toLowerCase();
				if (t) pay.add(t);
			}
		}
	}
	return { currencies: [...cur].sort(), paymentMethods: [...pay].sort(), count: data.length };
}

const flow = {
	name: 'drift',
	description: 'Diff read-only API endpoints between this target and prod (reference)',
	requires: [],
	async run(ctx) {
		const targetApi = String(ctx.config.apiUrl).replace(/\/$/, '');
		const referenceApi = (process.env.DC_E2E_DRIFT_REFERENCE_API || REFERENCE_API_DEFAULT).replace(/\/$/, '');

		// Nothing to compare when the current target IS the reference (prod).
		if (targetApi === referenceApi) {
			ctx.note(`drift: current target is the reference (${referenceApi}); nothing to compare.`);
			ctx.log('drift: target === reference; skipped.');
			return;
		}

		const sides = {
			target: { label: 'target', origin: targetApi },
			ref: { label: 'prod', origin: referenceApi },
		};
		const results = {};
		for (const [key, side] of Object.entries(sides)) {
			results[key] = {
				health: await safeGet(`${side.label}.health`, side.origin, PATHS.health),
				capabilities: await safeGet(`${side.label}.capabilities`, side.origin, PATHS.capabilities),
				stats: await safeGet(`${side.label}.stats`, side.origin, PATHS.stats),
				offerings: await safeGet(`${side.label}.offerings`, side.origin, PATHS.offerings),
			};
		}
		const target = results.target;
		const ref = results.ref;
		ctx.metric('drift.reference', referenceApi);

		// DNS / reachability: an unreachable side is itself a drift FINDING.
		for (const side of [target, ref]) {
			for (const key of Object.keys(side)) {
				if (!side[key].ok) {
					ctx.note(`drift: ${side[key].label} unreachable — ${side[key].error} (${redact(side[key].url)})`);
				}
			}
		}

		// 1. environment: each env should report its own value. Flag a non-prod
		//    target reporting 'prod' (config bleed) or a missing field.
		if (target.health.ok && target.health.status === 200) {
			const env = target.health.json?.environment ?? target.health.json?.data?.environment;
			ctx.metric('drift.target.environment', env ?? null);
			if (!env) {
				ctx.note('drift: target /health has no environment field.');
			} else if (env === 'prod' && ctx.config.target !== 'prod') {
				ctx.note(`drift: non-prod target reports environment='prod' (config bleed from prod).`);
			}
		}
		if (ref.health.ok && ref.health.status === 200) {
			ctx.metric('drift.ref.environment', ref.health.json?.environment ?? null);
		}

		// 2. capabilities: status + google_oauth surface should match prod.
		if (target.capabilities.ok && ref.capabilities.ok) {
			if (target.capabilities.status !== ref.capabilities.status) {
				ctx.note(
					`drift: /auth/capabilities status differs — target=${target.capabilities.status} vs prod=${ref.capabilities.status} ` +
						`(prod google_oauth=${ref.capabilities.json?.google_oauth ?? 'n/a'}).`,
				);
			} else if (target.capabilities.status === 200) {
				const t = target.capabilities.json?.google_oauth;
				const r = ref.capabilities.json?.google_oauth;
				if (t !== r) ctx.note(`drift: google_oauth differs — target=${t} vs prod=${r}.`);
			}
		}

		// 3. stats: surface large-order-of-magnitude divergence (envs
		//    legitimately differ in volume, so no exact-equality assert).
		if (target.stats.ok && ref.stats.ok && target.stats.status === 200 && ref.stats.status === 200) {
			const td = target.stats.json?.data;
			const rd = ref.stats.json?.data;
			ctx.metric('drift.target.total_offerings', td?.total_offerings ?? null);
			ctx.metric('drift.ref.total_offerings', rd?.total_offerings ?? null);
			if (
				typeof td?.total_offerings === 'number' &&
				typeof rd?.total_offerings === 'number' &&
				rd.total_offerings > 0 &&
				td.total_offerings > rd.total_offerings * 2
			) {
				ctx.note(
					`drift: target total_offerings (${td.total_offerings}) >> prod (${rd.total_offerings}) — possible stale demo accumulation.`,
				);
			}
		}

		// 4. offerings: currency/payment_methods surface (stage served retired
		//    ICP while prod served USD/Stripe).
		if (
			target.offerings.ok &&
			ref.offerings.ok &&
			target.offerings.status === 200 &&
			ref.offerings.status === 200
		) {
			const t = offeringSets(target.offerings.json);
			const r = offeringSets(ref.offerings.json);
			ctx.metric('drift.target.currencies', t.currencies.join(','));
			ctx.metric('drift.ref.currencies', r.currencies.join(','));
			const tRetired = t.currencies.filter((c) => RETIRED_CURRENCIES.includes(c));
			if (tRetired.length && !r.currencies.some((c) => RETIRED_CURRENCIES.includes(c))) {
				ctx.note(
					`drift: target serves retired currency ${tRetired.join(',')} (offerings sample) while prod does not — ` +
						`target currencies=[${t.currencies.join(',')}] vs prod=[${r.currencies.join(',')}]`,
				);
			}
		}

		ctx.log(`drift: compared target (${ctx.config.target}) vs reference (${referenceApi}).`);
	},
};

export default flow;
