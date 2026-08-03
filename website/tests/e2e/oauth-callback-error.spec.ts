import { test, expect } from '@playwright/test';
import { API_BASE_URL } from './fixtures/api-base';

/**
 * Smoke finding 2026-08-03: GET /api/v1/oauth/google/callback without ?code=
 * (Google's consent-denial redirect carries ?error=access_denied, or a direct/
 * malformed hit carries neither) used to return a bare 400 "missing field code".
 * A real user hit this 4x in rapid succession in prod. The handler now redirects
 * those cases to the frontend login with a user-facing oauth_error message.
 *
 * These tests hit the API callback directly (no UI, no OAuth credentials needed:
 * the error branch short-circuits before token exchange) and assert the
 * graceful redirect — the contract the unit tests in api/src/oauth_simple.rs
 * (`oauth_callback_error_redirect`) also guard at the decision layer.
 */
test.describe('oauth google callback error handling', () => {
	test('consent denial (?error=access_denied) redirects to login, not 400', async ({ request }) => {
		const res = await request.get(
			`${API_BASE_URL}/api/v1/oauth/google/callback?error=access_denied&state=anything`,
			{ maxRedirects: 0 },
		);
		// Must NOT be a 400-class client error.
		expect(res.status(), 'must not be a bare 400').toBeLessThan(400);
		expect(res.status(), 'must be a redirect').toBe(302);
		const location = res.headers()['location'];
		expect(location, 'must redirect to /login').toContain('/login');
		expect(location, 'must carry an oauth_error reason').toContain('oauth_error=');
	});

	test('missing params (direct hit) redirects to login, not 400', async ({ request }) => {
		const res = await request.get(`${API_BASE_URL}/api/v1/oauth/google/callback`, {
			maxRedirects: 0,
		});
		expect(res.status(), 'must not be a bare 400').toBeLessThan(400);
		expect(res.status(), 'must be a redirect').toBe(302);
		const location = res.headers()['location'];
		expect(location, 'must redirect to /login').toContain('/login');
		expect(location, 'must carry an oauth_error reason').toContain('oauth_error=');
	});
});
