// Flow runner: dependency expansion, shared context, result aggregation.
//
// Each flow is { name, description, requires[], run(ctx) }. A flow throws an
// AssertionError (or any Error) on hard failure; the runner records FAIL. Flows
// use ctx.note() to record non-fatal FINDINGs (printed, do not fail the run).
//
// Exit code: 0 only if every selected flow PASSED (findings are OK); non-zero if
// any flow FAILED or if config validation failed.

import { AssertionError } from './assert.js';
import { httpJson, failDetail } from './http.js';
import { signRequest } from './crypto.js';
import { launchBrowser } from './browser.js';
import { FLOW_REQUIREMENTS, validateConfig } from './config.js';

import health from './flows/health.js';
import marketplace from './flows/marketplace.js';
import signup from './flows/signup.js';
import providerOnboardPathA from './flows/providerOnboardPathA.js';
import rentProvisionCancel from './flows/rentProvisionCancel.js';

/** Registry (order = display order). */
export const FLOWS = [
	health,
	marketplace,
	signup,
	providerOnboardPathA,
	rentProvisionCancel,
];

/** Names in order. */
export function listFlowNames() {
	return FLOWS.map((f) => f.name);
}

/** Expand a selection to include required prerequisites (deduped, ordered). */
export function expandSelection(selected) {
	const byName = new Map(FLOWS.map((f) => [f.name, f]));
	const out = [];
	const seen = new Set();
	const visit = (name) => {
		if (seen.has(name)) return;
		const flow = byName.get(name);
		if (!flow) throw new Error(`Unknown flow '${name}'. Available: ${listFlowNames().join(', ')}`);
		for (const dep of flow.requires ?? []) {
			const depName = typeof dep === 'string' ? dep : dep;
			visit(depName);
		}
		seen.add(name);
		out.push(flow);
	};
	if (!selected || selected.length === 0) {
		// Default: the non-gated flows (rent-provision-cancel is opt-in).
		for (const f of FLOWS) if (f.name !== 'rent-provision-cancel') visit(f.name);
	} else {
		for (const n of selected) visit(n);
	}
	return out;
}

/**
 * @param {{config: object, selectedFlows: object[], log: (s:string)=>void, onResult?: (r:object)=>void}} args
 */
export async function runFlows({ config, selectedFlows, log, onResult }) {
	// Build the shared, mutable context that flows read/write.
	/** @type {any} */
	const ctx = {
		config,
		account: null,
		offering: null,
		_findings: [],
		_metrics: {},
		browser: null,
	};
	ctx.log = (s) => log(`    ↳ ${s}`);
	ctx.note = (s) => ctx._findings.push(s);
	ctx.metric = (k, v) => {
		ctx._metrics[k] = v;
	};
	ctx.assert = (cond, detail) => {
		if (!cond) throw new AssertionError(detail);
	};
	ctx.assertEquals = (a, b, label) => {
		if (a !== b) throw new AssertionError(`${label}: expected ${JSON.stringify(b)}, got ${JSON.stringify(a)}`);
	};

	// HTTP helpers.
	ctx.apiGet = (path, opts) => httpJson(`${config.apiUrl}${path}`, { method: 'GET', ...opts });
	ctx.signed = async (identity, method, path, bodyData, opts) => {
		const signed = await signRequest(identity, method, path, bodyData);
		return httpJson(`${config.apiUrl}${path}`, {
			method,
			headers: signed.headers,
			body: signed.body,
			...opts,
		});
	};

	const results = [];
	let browserLaunched = false;
	try {
		for (const flow of selectedFlows) {
			// Lazy-launch a single shared browser the first time a flow needs it.
			if (!ctx.browser && (flow.name === 'signup' || flow.requires?.includes('signup'))) {
				log(`launching own headless Chromium…`);
				ctx.browser = await launchBrowser();
				browserLaunched = true;
			}

			const started = Date.now();
			ctx._findings = [];
			let status = 'pass';
			let detail = '';
			try {
				await flow.run(ctx);
				detail = 'ok';
			} catch (e) {
				status = 'fail';
				detail = e instanceof AssertionError ? e.detail : `${e?.name ?? 'Error'}: ${e?.message ?? String(e)}`;
			}
			const result = {
				name: flow.name,
				status,
				detail,
				findings: [...ctx._findings],
				durationMs: Date.now() - started,
			};
			results.push(result);
			onResult?.(result);
		}
	} finally {
		if (browserLaunched && ctx.browser) {
			await ctx.browser.close().catch(() => {});
		}
	}
	return { results, metrics: ctx._metrics };
}

/** Compute the final exit code: 0 iff no FAIL. */
export function summarize(results) {
	const fails = results.filter((r) => r.status === 'fail');
	const findings = results.flatMap((r) => r.findings);
	return {
		total: results.length,
		passed: results.length - fails.length,
		failed: fails.length,
		findings: findings.length,
		exitCode: fails.length === 0 ? 0 : 1,
	};
}
