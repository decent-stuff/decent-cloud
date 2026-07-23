/**
 * Resolve the API base URL for the current Playwright run.
 *
 * Why this exists: several specs make DIRECT API calls from Node
 * (`request.get`, `fetch`) instead of going through the website. Those calls
 * must hit the SAME stack the browser is driving, but the only per-stack signal
 * Playwright exposes to Node is `PLAYWRIGHT_BASE_URL` (set by playwright.config.ts
 * and scripts/e2e-shard.sh). `VITE_DECENT_CLOUD_API_URL` is injected into the Vite
 * dev-server process only — it is NOT visible to the Playwright Node process, so
 * reading it here always falls back to the default. The warm-stack port scheme is
 * always `api_port = web_port + 1` (59010/59011, 59110/59111, …), so we derive the
 * API URL from the web base URL unless an explicit override is provided.
 *
 * Resolution order:
 *   1. PLAYWRIGHT_API_URL  (explicit, set by e2e-shard.sh per shard)
 *   2. derive from PLAYWRIGHT_BASE_URL (api port = web port + 1)
 *   3. default warm stack http://localhost:59011
 */
function resolveApiBaseUrl(): string {
	const explicit = process.env.PLAYWRIGHT_API_URL || process.env.VITE_DECENT_CLOUD_API_URL;
	if (explicit) return explicit.replace(/\/$/, '');
	const webBase = process.env.PLAYWRIGHT_BASE_URL;
	if (webBase) {
		try {
			const url = new URL(webBase);
			const webPort = Number(url.port) || (url.protocol === 'https:' ? 443 : 80);
			url.port = String(webPort + 1);
			return url.toString().replace(/\/$/, '');
		} catch {
			// fall through to default
		}
	}
	return 'http://localhost:59011';
}

export const API_BASE_URL = resolveApiBaseUrl();
