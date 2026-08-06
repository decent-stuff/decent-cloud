// Flow — stats-honesty
// Heuristic alarm: checks that /stats active_providers is plausible relative to
// the number of actually-rentable offerings (provider_online AND not is_example)
// observed in /offerings. A large divergence — e.g. active_providers>0 while
// ZERO offerings are online — points at the retired-table stat bug (stats
// counting providers from a stale table that no live offerings reference).
//
// Conservative by design: emits FINDINGs only, never hard-fails — this is a
// heuristic alarm, not an invariant (a brand-new prod with providers
// mid-onboarding could legitimately transiently show active>0 with no live
// offerings yet).

import { failDetail } from '../http.js';

const SAMPLE_LIMIT = 100;

const flow = {
	name: 'stats-honesty',
	description: 'Heuristic: /stats active_providers vs count of provider_online offerings',
	requires: [],
	async run(ctx) {
		const stats = await ctx.apiGet('/api/v1/stats');
		ctx.assert(stats.status === 200, failDetail('GET /stats must return 200', stats));
		const statsData = stats.json?.data;
		ctx.assert(statsData && typeof statsData === 'object', failDetail('/stats must have a data object', stats));

		const activeProviders = typeof statsData.active_providers === 'number' ? statsData.active_providers : null;
		const totalProviders = typeof statsData.total_providers === 'number' ? statsData.total_providers : null;
		const totalOfferings = typeof statsData.total_offerings === 'number' ? statsData.total_offerings : null;

		const offerings = await ctx.apiGet(`/api/v1/offerings?limit=${SAMPLE_LIMIT}`);
		ctx.assert(offerings.status === 200, failDetail('GET /offerings must return 200', offerings));
		ctx.assert(Array.isArray(offerings.json?.data), failDetail('/offerings must have a data array', offerings));
		const listings = offerings.json.data;

		// "rentable" = a real (non-example) offering with a live provider.
		const online = listings.filter((o) => o.is_example !== true && o.provider_online === true).length;

		// Did we observe the WHOLE catalog, or only a sample? Affects how strong
		// the signal is: with the whole catalog, active>0 + zero online is a
		// solid retired-table signature; with a partial sample it may just
		// undercount.
		const sawWholeCatalog = typeof totalOfferings === 'number' && listings.length >= totalOfferings;

		ctx.metric('stats-honesty.active_providers', activeProviders);
		ctx.metric('stats-honesty.total_providers', totalProviders);
		ctx.metric('stats-honesty.online_offerings', online);
		ctx.metric('stats-honesty.sample_size', listings.length);

		// Core heuristic: active_providers>0 with zero online offerings is the
		// retired-table stat-bug signature.
		if (activeProviders !== null && activeProviders > 0 && online === 0) {
			if (sawWholeCatalog) {
				ctx.note(
					`stats-honesty: active_providers=${activeProviders} but ZERO provider_online offerings ` +
						`across the full catalog (${listings.length}/${totalOfferings}). ` +
						`Likely the retired-table stat bug (stats counting a provider table no live offerings reference).`,
				);
			} else {
				ctx.note(
					`stats-honesty: active_providers=${activeProviders} but ZERO provider_online offerings in the first ` +
						`${SAMPLE_LIMIT} (catalog reports ${totalOfferings}; sample may undercount — verify with a wider query).`,
				);
			}
		}

		ctx.log(
			`stats-honesty: active_providers=${activeProviders}/${totalProviders}, online_offerings=${online} ` +
				`(of ${listings.length} sampled${sawWholeCatalog ? ', full catalog' : ''}).`,
		);
	},
};

export default flow;
