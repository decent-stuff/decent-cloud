// Flow #1 — health
// Asserts the API is up, reports the right environment, and parses capabilities.
// The capabilities check degrades to a FINDING (not a hard fail) on a clean 404:
// some deployments (notably older staging rolls) do not expose the endpoint, and
// a missing supplementary public endpoint is a flag, not an outage. 5xx/parse
// errors remain hard FAILs.

import { failDetail } from '../http.js';

const flow = {
	name: 'health',
	description: 'GET /api/v1/health (up + environment) and GET /auth/capabilities (parses)',
	requires: [],
	async run(ctx) {
		const { apiUrl, expectedEnvironment } = ctx.config;

		const health = await ctx.apiGet('/api/v1/health');
		ctx.assert(
			health.status === 200 && health.json?.success === true,
			failDetail('GET /health must return 200 + {success:true}', health),
		);
		const env = health.json?.environment;
		ctx.assert(typeof env === 'string' && env.length > 0, failDetail('/health missing environment field', health));
		if (expectedEnvironment) {
			ctx.assertEquals(
				env,
				expectedEnvironment,
				`/health environment mismatch (target expects '${expectedEnvironment}')`,
			);
		}
		ctx.metric('environment', env);
		ctx.log(`health up, environment=${env}`);

		// Capabilities: hard-assert parse on 200; FINDING on 404; FAIL otherwise.
		const caps = await ctx.apiGet('/api/v1/auth/capabilities');
		if (caps.status === 200) {
			ctx.assert(
				caps.json != null && typeof caps.json === 'object',
				failDetail('/auth/capabilities must parse to an object', caps),
			);
			ctx.metric('google_oauth', caps.json?.google_oauth === true);
			ctx.log(`capabilities parsed, google_oauth=${caps.json?.google_oauth === true}`);
		} else if (caps.status === 404) {
			ctx.note(
				`GET /auth/capabilities returned 404 — the capabilities endpoint is not deployed on this target ` +
					`(expected by the harness contract; auth-method detection will be unavailable to clients).`,
			);
		} else {
			ctx.assert(false, failDetail('GET /auth/capabilities must return 200 (or 404)', caps));
		}
	},
};

export default flow;
