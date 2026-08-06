// Flow #2 — marketplace
// Asserts the stats + offerings endpoints return the documented shape and
// records honest counts. Beyond shape, it asserts the catalog is HONEST:
//   - prod with ZERO offerings                  → FINDING (honest-empty, but a
//                                                  prod with nothing to sell)
//   - non-empty but ALL is_example              → FAIL on prod / FINDING else
//                                                  (synthetic demos masquerading
//                                                  as real inventory)
//   - non-empty but ZERO rentable offerings     → FAIL on prod / FINDING else
//     (provider_online + not example)             (nothing a user can rent)
// A prod catalog seeded only with fake demos (under the ASCII
// "example-offering-provider-identifier" pubkey) is the exact regression this
// flow exists to catch — without these assertions it passed while the catalog
// was dishonest.

import { failDetail } from '../http.js';

/** Best-effort decode of a hex pubkey to printable ASCII (for honest messages). */
function hexToAscii(hex) {
	if (typeof hex !== 'string' || !hex) return '';
	const clean = hex.replace(/^0x/, '');
	let out = '';
	for (let i = 0; i + 1 < clean.length; i += 2) {
		const code = parseInt(clean.slice(i, i + 2), 16);
		if (Number.isNaN(code)) return '';
		out += code >= 0x20 && code <= 0x7e ? String.fromCharCode(code) : '.';
	}
	return out;
}

const flow = {
	name: 'marketplace',
	description: 'GET /stats + /offerings; assert shape + catalog honesty; record counts',
	requires: [],
	async run(ctx) {
		const stats = await ctx.apiGet('/api/v1/stats');
		ctx.assert(stats.status === 200, failDetail('GET /stats must return 200', stats));
		const statsData = stats.json?.data;
		ctx.assert(statsData && typeof statsData === 'object', failDetail('/stats must have a data object', stats));
		ctx.assert(
			typeof statsData.total_offerings === 'number',
			failDetail('/stats.data.total_offerings must be a number', stats),
		);
		ctx.metric('stats.total_providers', statsData.total_providers);
		ctx.metric('stats.active_providers', statsData.active_providers);
		ctx.metric('stats.total_offerings', statsData.total_offerings);
		ctx.metric('stats.total_contracts', statsData.total_contracts);

		const offerings = await ctx.apiGet('/api/v1/offerings');
		ctx.assert(offerings.status === 200, failDetail('GET /offerings must return 200', offerings));
		ctx.assert(Array.isArray(offerings.json?.data), failDetail('/offerings must have a data array', offerings));
		const listings = offerings.json.data;
		ctx.metric('offerings.listed', listings.length);

		// Honest-empty is fine for non-prod; prod-empty is a finding.
		if (ctx.config.target === 'prod' && listings.length === 0) {
			ctx.note('prod marketplace returned 0 offerings — a real prod should have inventory.');
		}

		// ── Honesty assertions ───────────────────────────────────────────
		// A non-empty catalog must reflect real, rentable inventory — not a
		// pile of is_example synthetic demos under a fake pubkey.
		if (listings.length > 0) {
			const examples = listings.filter((o) => o.is_example === true);
			const rentable = listings.filter((o) => o.is_example !== true && o.provider_online === true);
			const allDemos = examples.length === listings.length;
			const noneRentable = rentable.length === 0;

			if (allDemos || noneRentable) {
				const reason = allDemos
					? 'every listing is is_example (synthetic demo data, not real inventory)'
					: 'no listing is actually rentable (zero non-example offerings with provider_online=true)';
				const counts = `total=${listings.length} example=${examples.length} rentable_online=${rentable.length}`;
				const pubkeyNote = examples[0]?.pubkey
					? ` example pubkey "${examples[0].pubkey}" decodes to ASCII "${hexToAscii(examples[0].pubkey)}"`
					: '';
				const msg =
					`DISHONEST marketplace catalog: non-empty but ${reason}. ${counts}.${pubkeyNote} ` +
					`A prod catalog seeded only with demos is the exact regression this harness exists to catch.`;
				// On prod a dishonest catalog is a hard FAIL (forces action);
				// on dev/stage it is a FINDING (may legitimately be demo-only
				// while the env is being built out).
				if (ctx.config.target === 'prod') {
					ctx.assert(false, msg);
				} else {
					ctx.note(msg);
				}
			}
		}

		ctx.log(
			`marketplace: ${listings.length} listed, stats reports ${statsData.total_offerings} total offerings ` +
				`(${statsData.active_providers}/${statsData.total_providers} providers active)`,
		);
	},
};

export default flow;
