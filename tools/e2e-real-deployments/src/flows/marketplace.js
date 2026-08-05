// Flow #2 — marketplace
// Asserts the stats + offerings endpoints return the documented shape and
// records honest counts. An empty marketplace is a PASS for a fresh stage, but
// zero offerings on prod is surfaced as a FINDING (a live prod with nothing to
// sell is a regression worth flagging).

import { failDetail } from '../http.js';

const flow = {
	name: 'marketplace',
	description: 'GET /stats + /offerings; assert shape; record counts',
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
		ctx.log(
			`marketplace: ${listings.length} listed, stats reports ${statsData.total_offerings} total offerings ` +
				`(${statsData.active_providers}/${statsData.total_providers} providers active)`,
		);
	},
};

export default flow;
