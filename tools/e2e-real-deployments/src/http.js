// Shared HTTP helper. Every request is bounded by a timeout via AbortController
// — there are NO unbounded fetches. Errors are thrown loudly (never swallowed).

const DEFAULT_TIMEOUT_MS = 30_000;

/**
 * Perform a bounded HTTP request and parse the response.
 *
 * @param {string} url
 * @param {{method?: string, headers?: Record<string,string>, body?: string, timeoutMs?: number}} [opts]
 * @returns {Promise<{status:number, ok:boolean, text:string, json:unknown}>}
 */
export async function httpJson(url, opts = {}) {
	const method = (opts.method ?? 'GET').toUpperCase();
	const timeoutMs = opts.timeoutMs ?? DEFAULT_TIMEOUT_MS;
	const controller = new AbortController();
	const timer = setTimeout(() => controller.abort(), timeoutMs);

	let res;
	try {
		res = await fetch(url, {
			method,
			headers: opts.headers ?? {},
			body: opts.body ?? undefined,
			signal: controller.signal,
			redirect: 'follow',
		});
	} catch (e) {
		clearTimeout(timer);
		if (e?.name === 'AbortError') {
			throw new Error(`HTTP ${method} ${redact(url)} timed out after ${timeoutMs}ms`);
		}
		throw new Error(`HTTP ${method} ${redact(url)} failed: ${e?.message ?? String(e)}`);
	}
	clearTimeout(timer);

	const text = await res.text();
	let json = undefined;
	if (text) {
		try {
			json = JSON.parse(text);
		} catch {
			/* non-JSON body — leave json undefined; caller inspects text */
		}
	}
	return { status: res.status, ok: res.ok, text, json };
}

/** Truncate a response body for debug output. */
export function excerpt(text, max = 300) {
	if (!text) return '<empty>';
	const t = typeof text === 'string' ? text : JSON.stringify(text);
	return t.length > max ? t.slice(0, max) + `… (+${t.length - max} more chars)` : t;
}

/**
 * Build a one-line failure detail string for assertions: the message plus the
 * last HTTP status + truncated body, so failures are debuggable from the log.
 *
 * @param {string} assertion
 * @param {{status?: number, json?: unknown, text?: string, extra?: string}} [http]
 */
export function failDetail(assertion, http) {
	const parts = [assertion];
	if (http && typeof http.status === 'number') parts.push(`HTTP ${http.status}`);
	if (http) parts.push(`body: ${excerpt(http.json ?? http.text)}`);
	if (http?.extra) parts.push(http.extra);
	return parts.join(' | ');
}

/** Scrub any embedded credentials from a URL for safe logging. */
export function redact(url) {
	return String(url).replace(/(https?:\/\/)[^/@:]+:[^/@]+@/g, '$1***@');
}
